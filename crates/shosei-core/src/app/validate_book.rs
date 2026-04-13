use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::Serialize;

use crate::{
    cli_api::CommandContext,
    config,
    diagnostics::{Severity, ValidationIssue},
    domain::ProjectType,
    fs::join_repo_path,
    manga, pipeline,
    repo::{self, RepoError},
    toolchain::{self, ToolStatus},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ValidateBookResult {
    pub summary: String,
    pub plan: Option<pipeline::ValidatePlan>,
    pub report_path: PathBuf,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Error)]
pub enum ValidateBookError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("failed to write validation report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize validation report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("validation planning is not implemented yet for {project_type}")]
    UnsupportedProjectType { project_type: ProjectType },
    #[error("requested target `{target}` is not enabled for this book")]
    TargetNotEnabled { target: String },
}

pub fn validate_book(command: &CommandContext) -> Result<ValidateBookResult, ValidateBookError> {
    let toolchain = toolchain::inspect_default_toolchain();
    validate_book_with_toolchain(command, &toolchain)
}

fn validate_book_with_toolchain(
    command: &CommandContext,
    toolchain: &toolchain::ToolchainReport,
) -> Result<ValidateBookResult, ValidateBookError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    if let Some(book) = context.book.clone() {
        let resolved = config::resolve_book_config(&context)?;
        let project_type = resolved.effective.project.project_type;
        let report_path = report_path(&resolved);
        let selected_channel = pipeline::selected_output_channel(command);
        let (plan, mut issues) = match match project_type {
            ProjectType::Manga => pipeline::manga_validate_plan_with_toolchain(
                context,
                &resolved,
                toolchain,
                selected_channel,
            ),
            _ => pipeline::prose_validate_plan_with_toolchain(
                context,
                &resolved,
                toolchain,
                selected_channel,
            ),
        } {
            Ok(plan) => (Some(plan), Vec::new()),
            Err(pipeline::PipelineError::PreflightFailed { diagnostics, .. }) => (
                None,
                diagnostics
                    .into_iter()
                    .map(|diagnostic| validation_issue_from_diagnostic(&resolved, diagnostic))
                    .collect(),
            ),
        };
        let outputs = selected_outputs(&resolved, command.output_target.as_deref());
        if command.output_target.is_some() && outputs.is_empty() {
            return Err(ValidateBookError::TargetNotEnabled {
                target: command
                    .output_target
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            });
        }
        if let Some(plan) = &plan {
            issues.extend(issues_from_checks(plan));
            issues.extend(schema_warning_issues(&resolved));
            issues.extend(match project_type {
                ProjectType::Manga => manga_validation_issues(&resolved, plan),
                _ => prose_validation_issues(&resolved, plan),
            });
        }
        issues.extend(cover_validation_issues(&resolved));
        let report = ValidateReport {
            book_id: book.id.clone(),
            outputs: outputs.clone(),
            checks: plan
                .as_ref()
                .map(|plan| {
                    plan.checks
                        .iter()
                        .map(|check| ValidationCheckReport {
                            name: check.name.to_string(),
                            target: check.target.to_string(),
                            tool: check.tool.map(str::to_string),
                            status: check.tool_status.to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            issues: issues.clone(),
        };
        write_report(&report_path, &report)?;
        let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);
        return Ok(ValidateBookResult {
            summary: format!(
                "validation completed for {} with outputs: {}, issues: {}, report: {}",
                book.id,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                },
                issues.len(),
                report_path.display()
            ),
            plan,
            report_path,
            issue_count: issues.len(),
            has_errors,
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}

#[derive(Debug, Clone, Serialize)]
struct ValidateReport {
    book_id: String,
    outputs: Vec<String>,
    checks: Vec<ValidationCheckReport>,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationCheckReport {
    name: String,
    target: String,
    tool: Option<String>,
    status: String,
}

fn issues_from_checks(plan: &pipeline::ValidatePlan) -> Vec<ValidationIssue> {
    plan.checks
        .iter()
        .filter_map(|check| match check.tool_status {
            ToolStatus::Missing => Some(ValidationIssue::error(
                check.target,
                format!(
                    "required validation tool is missing: {}",
                    check.tool.unwrap_or(check.name)
                ),
                "shosei doctor を実行して、必要な外部依存を PATH に追加してください。",
            )),
            ToolStatus::NotYetImplemented => Some(ValidationIssue::warning(
                check.target,
                format!("validation step is not implemented yet: {}", check.name),
                "この target の validate 実装を追加するまで、追加の手動確認が必要です。",
            )),
            _ => None,
        })
        .collect()
}

fn validation_issue_from_diagnostic(
    resolved: &config::ResolvedBookConfig,
    diagnostic: crate::diagnostics::Diagnostic,
) -> ValidationIssue {
    match diagnostic.code {
        "missing-manuscript" => {
            let issue = ValidationIssue::error(
                "common",
                diagnostic.message,
                "対象ファイルを追加するか、book.yml の manuscript 設定を修正してください。",
            );
            match diagnostic.path {
                Some(path) => issue.at(path),
                None => issue,
            }
        }
        "missing-manga-pages" => issue_from_severity(
            resolved.effective.validation.missing_image,
            "common",
            diagnostic.message,
            "manga/pages/ に PNG または JPEG のページ画像を追加してください。",
            diagnostic.path,
        ),
        _ => {
            let issue = ValidationIssue::error(
                "common",
                diagnostic.message,
                "設定と入力ファイルを確認してください。",
            );
            match diagnostic.path {
                Some(path) => issue.at(path),
                None => issue,
            }
        }
    }
}

fn schema_warning_issues(resolved: &config::ResolvedBookConfig) -> Vec<ValidationIssue> {
    let config_path = resolved
        .repo
        .book
        .as_ref()
        .expect("book context must exist")
        .config_path
        .clone();
    let project_type = resolved.effective.project.project_type;
    let mut issues = Vec::new();

    if project_type == ProjectType::Manga && resolved.has_path(&["manuscript", "chapters"]) {
        issues.push(
            ValidationIssue::warning(
                "common",
                "project.type is manga but manuscript.chapters is also present".to_string(),
                "manga project では manuscript ではなく manga/pages を使ってください。",
            )
            .at(config_path.clone()),
        );
    }

    if project_type != ProjectType::Manga && resolved.has_path(&["manga"]) {
        issues.push(
            ValidationIssue::warning(
                "common",
                "project.type is not manga but a manga section is present".to_string(),
                "manga セクションを削除するか、project.type を manga に変更してください。",
            )
            .at(config_path.clone()),
        );
    }

    if resolved.effective.outputs.print.is_some() && !resolved.has_path(&["print"]) {
        issues.push(
            ValidationIssue::warning(
                "print",
                "print output is enabled but no print section is defined".to_string(),
                "print セクションを追加して trim size や bleed などの印刷設定を明示してください。",
            )
            .at(config_path),
        );
    }

    if project_type.is_prose()
        && resolved.effective.outputs.print.is_some()
        && !resolved.has_path(&["pdf"])
    {
        issues.push(
            ValidationIssue::warning(
                "print",
                "print output is enabled but no pdf section is defined".to_string(),
                "pdf セクションを追加して engine や running_header などの PDF 設定を明示してください。",
            )
            .at(
                resolved
                    .repo
                    .book
                    .as_ref()
                    .expect("book context must exist")
                    .config_path
                    .clone(),
            ),
        );
    }

    issues
}

fn cover_validation_issues(resolved: &config::ResolvedBookConfig) -> Vec<ValidationIssue> {
    let Some(cover_path) = resolved.effective.cover.ebook_image.as_ref() else {
        return Vec::new();
    };
    let fs_path = join_repo_path(&resolved.repo.repo_root, cover_path);
    if fs_path.is_file() {
        return Vec::new();
    }

    vec![issue_from_severity(
        resolved.effective.validation.missing_image,
        if resolved.effective.outputs.kindle.is_some() {
            "kindle"
        } else {
            "common"
        },
        format!("cover image file not found: {}", cover_path.as_str()),
        "cover.ebook_image を修正するか、対象ファイルを追加してください。",
        Some(fs_path),
    )]
}

fn prose_validation_issues(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::ValidatePlan,
) -> Vec<ValidationIssue> {
    let Some(manuscript) = resolved.effective.manuscript.as_ref() else {
        return Vec::new();
    };

    let chapter_paths = manuscript
        .chapters
        .iter()
        .map(|path| join_repo_path(&resolved.repo.repo_root, path))
        .collect::<Vec<_>>();
    let mut issues = Vec::new();

    for file_path in &plan.manuscript_files {
        let analysis = match analyze_markdown_file(file_path) {
            Ok(analysis) => analysis,
            Err(source) => {
                issues.push(ValidationIssue::error(
                    "common",
                    format!(
                        "failed to read manuscript file during validation: {}",
                        file_path.display()
                    ),
                    format!("ファイルを読めませんでした: {source}"),
                ));
                continue;
            }
        };

        for image in &analysis.images {
            if image.alt.trim().is_empty() {
                issues.push(issue_from_severity(
                    resolved.effective.validation.missing_alt,
                    if resolved.effective.outputs.kindle.is_some() {
                        "kindle"
                    } else {
                        "common"
                    },
                    format!("image is missing alt text: {}", image.destination),
                    "画像参照に代替テキストを追加してください。",
                    Some(file_path.clone()),
                ));
            }
            if !image.is_external
                && !resolved_path_exists(file_path, &resolved.repo.repo_root, &image.destination)
            {
                issues.push(issue_from_severity(
                    resolved.effective.validation.missing_image,
                    "common",
                    format!("image reference target not found: {}", image.destination),
                    "画像パスを修正するか、対象ファイルを追加してください。",
                    Some(file_path.clone()),
                ));
            }
        }

        for link in &analysis.links {
            if !link.is_external
                && !resolved_path_exists(file_path, &resolved.repo.repo_root, &link.destination)
            {
                issues.push(issue_from_severity(
                    resolved.effective.validation.broken_link,
                    "common",
                    format!("link target not found: {}", link.destination),
                    "リンク先パスを修正するか、対象ファイルを追加してください。",
                    Some(file_path.clone()),
                ));
            }
        }

        if chapter_paths.iter().any(|chapter| chapter == file_path) {
            if analysis.heading_levels.first().copied() != Some(1)
                && let Some(issue) = accessibility_issue(
                    resolved,
                    "common",
                    "chapter file does not begin with a level-1 heading".to_string(),
                    "各 chapter ファイルの先頭に `#` 見出しを置き、navigation の導出元を明確にしてください。",
                    file_path.clone(),
                )
            {
                issues.push(issue);
            }
            for (previous, current) in analysis.heading_levels.windows(2).filter_map(|levels| {
                if levels[1] > levels[0] + 1 {
                    Some((levels[0], levels[1]))
                } else {
                    None
                }
            }) {
                if let Some(issue) = accessibility_issue(
                    resolved,
                    "common",
                    format!("heading hierarchy skips levels: h{previous} -> h{current}"),
                    "見出しレベルを段階的に増やしてください。例: h1 の次は h2 を使います。",
                    file_path.clone(),
                ) {
                    issues.push(issue);
                }
            }
        }
    }

    if resolved.effective.outputs.kindle.is_some() && resolved.effective.cover.ebook_image.is_none()
    {
        issues.push(ValidationIssue::warning(
            "kindle",
            "kindle output is enabled but cover.ebook_image is not set".to_string(),
            "Kindle 向けメタデータ整合のため、cover.ebook_image を設定してください。",
        ));
    }

    issues
}

#[derive(Debug, Clone)]
struct MarkdownLink {
    destination: String,
    alt: String,
    is_external: bool,
}

#[derive(Debug, Default)]
struct MarkdownAnalysis {
    heading_levels: Vec<u32>,
    links: Vec<MarkdownLink>,
    images: Vec<MarkdownLink>,
}

fn analyze_markdown_file(path: &Path) -> Result<MarkdownAnalysis, std::io::Error> {
    let contents = fs::read_to_string(path)?;
    let heading_regex = Regex::new(r"(?m)^(#{1,6})[ \t]+(.+?)\s*$").expect("valid heading regex");
    let image_regex =
        Regex::new(r"!\[(?P<alt>[^\]]*)\]\((?P<dest>[^)]+)\)").expect("valid image regex");
    let link_regex =
        Regex::new(r"\[(?P<label>[^\]]*)\]\((?P<dest>[^)]+)\)").expect("valid link regex");

    Ok(MarkdownAnalysis {
        heading_levels: heading_regex
            .captures_iter(&contents)
            .map(|capture| capture[1].len() as u32)
            .collect(),
        images: image_regex
            .captures_iter(&contents)
            .map(|capture| {
                let destination = normalize_markdown_destination(&capture["dest"]);
                MarkdownLink {
                    is_external: is_external_destination(&destination),
                    destination,
                    alt: capture["alt"].to_string(),
                }
            })
            .collect(),
        links: link_regex
            .captures_iter(&contents)
            .filter(|capture| {
                capture
                    .get(0)
                    .map(|m| m.start() == 0 || contents.as_bytes()[m.start() - 1] != b'!')
                    .unwrap_or(false)
            })
            .map(|capture| {
                let destination = normalize_markdown_destination(&capture["dest"]);
                MarkdownLink {
                    is_external: is_external_destination(&destination),
                    destination,
                    alt: String::new(),
                }
            })
            .collect(),
    })
}

fn normalize_markdown_destination(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_title = trimmed.split_whitespace().next().unwrap_or(trimmed);
    without_title
        .trim_matches('<')
        .trim_matches('>')
        .to_string()
}

fn is_external_destination(destination: &str) -> bool {
    destination.starts_with("http://")
        || destination.starts_with("https://")
        || destination.starts_with("mailto:")
        || destination.starts_with("data:")
        || destination.starts_with('#')
}

fn resolved_path_exists(source_path: &Path, repo_root: &Path, destination: &str) -> bool {
    let Some(base_destination) = destination.split('#').next() else {
        return true;
    };
    if base_destination.is_empty() {
        return true;
    }

    let candidate = if base_destination.starts_with('/') {
        repo_root.join(base_destination.trim_start_matches('/'))
    } else {
        source_path
            .parent()
            .unwrap_or(repo_root)
            .join(base_destination)
    };
    candidate.exists()
}

fn accessibility_issue(
    resolved: &config::ResolvedBookConfig,
    target: impl Into<String>,
    cause: impl Into<String>,
    remedy: impl Into<String>,
    path: PathBuf,
) -> Option<ValidationIssue> {
    match resolved.effective.validation.accessibility {
        config::ValidationLevel::Off => None,
        config::ValidationLevel::Warn => {
            Some(ValidationIssue::warning(target, cause, remedy).at(path))
        }
        config::ValidationLevel::Error => {
            Some(ValidationIssue::error(target, cause, remedy).at(path))
        }
    }
}

fn manga_validation_issues(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::ValidatePlan,
) -> Vec<ValidationIssue> {
    let page_issue = |cause: String, path: Option<PathBuf>| {
        issue_from_severity(
            resolved.effective.validation.missing_image,
            "common",
            cause,
            "manga/pages/ の画像を修正し、すべてのページを同じ仕上がりサイズに揃えてください。",
            path,
        )
    };

    let page_assets = match manga::inspect_page_assets(&plan.manuscript_files) {
        Ok(page_assets) => page_assets,
        Err(manga::MangaRenderError::DecodePage { path }) => {
            return vec![page_issue(
                format!("failed to decode manga page: {}", path.display()),
                Some(path),
            )];
        }
        Err(_) => return Vec::new(),
    };

    let Some(first_page) = page_assets.first() else {
        return Vec::new();
    };
    let expected = (first_page.width_px, first_page.height_px);
    let mut issues = page_assets
        .iter()
        .enumerate()
        .filter(|(_, page)| (page.width_px, page.height_px) != expected)
        .map(|(index, page)| {
            page_issue(
                format!(
                    "manga page size mismatch on page {}: expected {}x{}, got {}x{}",
                    index + 1,
                    expected.0,
                    expected.1,
                    page.width_px,
                    page.height_px
                ),
                Some(
                    resolved
                        .repo
                        .book
                        .as_ref()
                        .expect("book context must exist")
                        .root
                        .join("manga/pages")
                        .join(&page.file_name),
                ),
            )
        })
        .collect::<Vec<_>>();

    if resolved.effective.outputs.kindle.is_some() {
        issues.extend(kindle_spread_policy_issues(resolved, &page_assets));
    }
    issues.extend(manga_color_policy_issues(resolved, &page_assets));

    issues
}

fn kindle_spread_policy_issues(
    resolved: &config::ResolvedBookConfig,
    page_assets: &[manga::MangaPageAsset],
) -> Vec<ValidationIssue> {
    let Some(manga_settings) = resolved.effective.manga.as_ref() else {
        return Vec::new();
    };
    let wide_pages = page_assets
        .iter()
        .filter(|page| page.is_wide_spread_candidate())
        .collect::<Vec<_>>();
    if wide_pages.is_empty() {
        return Vec::new();
    }

    let page_path = |file_name: &str| {
        resolved
            .repo
            .book
            .as_ref()
            .expect("book context must exist")
            .root
            .join("manga/pages")
            .join(file_name)
    };

    match manga_settings.spread_policy_for_kindle {
        config::SpreadPolicyForKindle::Split => Vec::new(),
        config::SpreadPolicyForKindle::SinglePage => wide_pages
            .into_iter()
            .map(|page| {
                ValidationIssue::warning(
                    "kindle",
                    format!(
                        "wide manga page will be emitted as a single Kindle page: {}",
                        page.file_name
                    ),
                    "manga.spread_policy_for_kindle を split にすると、見開き候補を 2 ページへ分割します。",
                )
                .at(page_path(&page.file_name))
            })
            .collect(),
        config::SpreadPolicyForKindle::Skip => {
            if wide_pages.len() == page_assets.len() {
                vec![ValidationIssue::error(
                    "kindle",
                    "kindle spread policy would skip every manga page".to_string(),
                    "manga.spread_policy_for_kindle を split または single-page に変更してください。",
                )]
            } else {
                wide_pages
                    .into_iter()
                    .map(|page| {
                        ValidationIssue::warning(
                            "kindle",
                            format!(
                                "wide manga page will be omitted from Kindle output: {}",
                                page.file_name
                            ),
                            "manga.spread_policy_for_kindle を split または single-page に変更すると、このページも Kindle 出力へ含められます。",
                        )
                        .at(page_path(&page.file_name))
                    })
                    .collect()
            }
        }
    }
}

fn manga_color_policy_issues(
    resolved: &config::ResolvedBookConfig,
    page_assets: &[manga::MangaPageAsset],
) -> Vec<ValidationIssue> {
    let Some(manga_settings) = resolved.effective.manga.as_ref() else {
        return Vec::new();
    };

    let page_path = |file_name: &str| {
        resolved
            .repo
            .book
            .as_ref()
            .expect("book context must exist")
            .root
            .join("manga/pages")
            .join(file_name)
    };

    let front_color_pages = manga_settings.front_color_pages as usize;
    if front_color_pages > page_assets.len() {
        return vec![ValidationIssue::error(
            "common",
            format!(
                "manga.front_color_pages exceeds resolved page count: {} > {}",
                front_color_pages,
                page_assets.len()
            ),
            "manga.front_color_pages を実際のページ数以下に修正してください。",
        )];
    }

    let mut issues = page_assets
        .iter()
        .take(front_color_pages)
        .filter(|page| !page.is_color)
        .map(|page| {
            ValidationIssue::warning(
                "common",
                format!(
                    "front color page is not detected as color: {}",
                    page.file_name
                ),
                "巻頭カラーページ数を減らすか、該当ページの画像内容を確認してください。",
            )
            .at(page_path(&page.file_name))
        })
        .collect::<Vec<_>>();

    let body_pages = &page_assets[front_color_pages..];
    match manga_settings.body_mode {
        config::MangaBodyMode::Monochrome => {
            issues.extend(body_pages.iter().filter(|page| page.is_color).map(|page| {
                ValidationIssue::error(
                    "common",
                    format!(
                        "body page is detected as color while manga.body_mode is monochrome: {}",
                        page.file_name
                    ),
                    "manga.body_mode を mixed または color に変更するか、本文ページをモノクロ画像へ揃えてください。",
                )
                .at(page_path(&page.file_name))
            }));
        }
        config::MangaBodyMode::Color => {
            issues.extend(body_pages.iter().filter(|page| !page.is_color).map(|page| {
                ValidationIssue::warning(
                    "common",
                    format!(
                        "body page is not detected as color while manga.body_mode is color: {}",
                        page.file_name
                    ),
                    "manga.body_mode を mixed または monochrome に変更するか、本文ページの画像内容を確認してください。",
                )
                .at(page_path(&page.file_name))
            }));
        }
        config::MangaBodyMode::Mixed => {}
    }

    issues
}

fn issue_from_severity(
    severity: config::ValidationSeverity,
    target: impl Into<String>,
    cause: impl Into<String>,
    remedy: impl Into<String>,
    path: Option<PathBuf>,
) -> ValidationIssue {
    let issue = match severity {
        config::ValidationSeverity::Warn => ValidationIssue::warning(target, cause, remedy),
        config::ValidationSeverity::Error => ValidationIssue::error(target, cause, remedy),
    };
    match path {
        Some(path) => issue.at(path),
        None => issue,
    }
}

fn report_path(resolved: &config::ResolvedBookConfig) -> PathBuf {
    let book_id = resolved
        .repo
        .book
        .as_ref()
        .map(|book| book.id.as_str())
        .unwrap_or("default");
    resolved
        .repo
        .repo_root
        .join("dist")
        .join("reports")
        .join(format!("{book_id}-validate.json"))
}

fn selected_outputs(
    resolved: &config::ResolvedBookConfig,
    selected_channel: Option<&str>,
) -> Vec<String> {
    let mut outputs = Vec::new();
    if (selected_channel.is_none() || selected_channel == Some("kindle"))
        && let Some(target) = &resolved.effective.outputs.kindle
    {
        outputs.push(target.clone());
    }
    if (selected_channel.is_none() || selected_channel == Some("print"))
        && let Some(target) = &resolved.effective.outputs.print
    {
        outputs.push(target.clone());
    }
    outputs
}

fn write_report(path: &std::path::Path, report: &ValidateReport) -> Result<(), ValidateBookError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ValidateBookError::WriteReport {
            path: path.to_path_buf(),
            source,
        })?;
    }
    let contents = serde_json::to_string_pretty(report).map_err(|source| {
        ValidateBookError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, contents).map_err(|source| ValidateBookError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::PathBuf};

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use crate::toolchain::{ToolRecord, ToolchainReport};

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shosei-validate-book-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_toolchain(epubcheck: ToolStatus) -> ToolchainReport {
        ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "epubcheck",
                    display_name: "epubcheck",
                    status: epubcheck,
                    detected_as: Some("epubcheck".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install epubcheck and ensure the launcher is available on PATH.",
                },
                ToolRecord {
                    key: "pdf-engine",
                    display_name: "PDF engine",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install one supported PDF engine such as weasyprint, typst, or lualatex.",
                },
            ],
        }
    }

    fn write_book(root: &std::path::Path) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
  epubcheck: true
git:
  lfs: true
"#,
        )
        .unwrap();
    }

    fn write_book_with_chapter_contents(
        root: &std::path::Path,
        chapter_contents: &str,
        validation_block: &str,
    ) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01.md"), chapter_contents).unwrap();
        fs::write(
            root.join("book.yml"),
            format!(
                r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
{validation_block}
git:
  lfs: true
"#
            ),
        )
        .unwrap();
    }

    fn write_book_with_cover(root: &std::path::Path, missing_image: &str, create_cover: bool) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        fs::write(
            root.join("book.yml"),
            format!(
                r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
  epubcheck: true
  missing_image: {missing_image}
git:
  lfs: true
cover:
  ebook_image: assets/cover/front.png
"#
            ),
        )
        .unwrap();
        if create_cover {
            fs::create_dir_all(root.join("assets/cover")).unwrap();
            fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
        }
    }

    fn write_manga_book(root: &std::path::Path) {
        write_manga_book_with_options(root, "error", "split", 0, "monochrome");
    }

    fn write_manga_book_with_missing_image(root: &std::path::Path, missing_image: &str) {
        write_manga_book_with_options(root, missing_image, "split", 0, "monochrome");
    }

    fn write_manga_book_with_options(
        root: &std::path::Path,
        missing_image: &str,
        spread_policy: &str,
        front_color_pages: usize,
        body_mode: &str,
    ) {
        fs::write(
            root.join("book.yml"),
            format!(
                r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
  missing_image: {missing_image}
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: {spread_policy}
  front_color_pages: {front_color_pages}
  body_mode: {body_mode}
"#,
            ),
        )
        .unwrap();
    }

    fn tiny_png() -> &'static [u8] {
        &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ]
    }

    fn wide_png() -> &'static [u8] {
        &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0xf4, 0x22, 0x7f, 0x8a, 0x00, 0x00, 0x00, 0x0e, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x84, 0x01, 0x11, 0xf7, 0x03, 0xfd, 0xe3,
            0xc5, 0xf5, 0xef, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60,
            0x82,
        ]
    }

    fn solid_png(r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([r, g, b, 255])));
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn validate_writes_report_when_epubcheck_is_missing() {
        let root = temp_dir("missing-epubcheck");
        write_book(&root);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        assert!(result.report_path.is_file());
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("required validation tool is missing"));
        assert!(report.contains("\"severity\": \"error\""));
    }

    #[test]
    fn validate_reports_missing_cover_image() {
        let root = temp_dir("missing-cover-image");
        write_book_with_cover(&root, "error", false);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("cover image file not found"));
        assert!(report.contains("assets/cover/front.png"));
    }

    #[test]
    fn validate_can_warn_for_missing_cover_image() {
        let root = temp_dir("missing-cover-image-warn");
        write_book_with_cover(&root, "warn", false);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("cover image file not found"));
        assert!(report.contains("\"severity\": \"warning\""));
    }

    #[test]
    fn validate_manga_reports_missing_page_directory() {
        let root = temp_dir("manga-missing-pages");
        write_manga_book(&root);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("manga page directory not found"));
        assert!(report.contains("manga/pages"));
    }

    #[test]
    fn validate_manga_can_warn_for_missing_pages() {
        let root = temp_dir("manga-missing-pages-warn");
        write_manga_book_with_missing_image(&root, "warn");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("\"severity\": \"warning\""));
    }

    #[test]
    fn validate_manga_reports_size_mismatch() {
        let root = temp_dir("manga-size-mismatch");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), tiny_png()).unwrap();
        fs::write(root.join("manga/pages/002.png"), wide_png()).unwrap();
        write_manga_book(&root);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("manga page size mismatch"));
        assert!(report.contains("002.png"));
    }

    #[test]
    fn validate_manga_warns_for_single_page_kindle_spread_policy() {
        let root = temp_dir("manga-single-page-policy");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), wide_png()).unwrap();
        write_manga_book_with_options(&root, "error", "single-page", 0, "mixed");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("single Kindle page"));
        assert!(report.contains("001.png"));
    }

    #[test]
    fn validate_manga_errors_when_skip_policy_removes_every_page() {
        let root = temp_dir("manga-skip-policy-empty");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), wide_png()).unwrap();
        write_manga_book_with_options(&root, "error", "skip", 0, "monochrome");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("kindle spread policy would skip every manga page"));
    }

    #[test]
    fn validate_manga_warns_when_front_color_page_is_not_detected_as_color() {
        let root = temp_dir("manga-front-color-warning");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), solid_png(120, 120, 120)).unwrap();
        write_manga_book_with_options(&root, "error", "split", 1, "mixed");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("front color page is not detected as color"));
        assert!(report.contains("001.png"));
    }

    #[test]
    fn validate_manga_errors_when_monochrome_body_contains_color_page() {
        let root = temp_dir("manga-monochrome-body-color");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), solid_png(120, 120, 120)).unwrap();
        fs::write(root.join("manga/pages/002.png"), solid_png(255, 0, 0)).unwrap();
        write_manga_book_with_options(&root, "error", "split", 1, "monochrome");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(
            report.contains("body page is detected as color while manga.body_mode is monochrome")
        );
        assert!(report.contains("002.png"));
    }

    #[test]
    fn validate_manga_errors_when_front_color_pages_exceed_total_pages() {
        let root = temp_dir("manga-front-color-overflow");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), solid_png(255, 0, 0)).unwrap();
        write_manga_book_with_options(&root, "error", "split", 2, "mixed");

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("manga.front_color_pages exceeds resolved page count"));
    }

    #[test]
    fn validate_warns_when_non_manga_book_contains_manga_section() {
        let root = temp_dir("novel-with-manga-section");
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx1a
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: monochrome
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("project.type is not manga but a manga section is present"));
        assert!(report.contains("print output is enabled but no print section is defined"));
    }

    #[test]
    fn validate_warns_when_manga_book_contains_manuscript_section() {
        let root = temp_dir("manga-with-manuscript");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), solid_png(120, 120, 120)).unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: mixed
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("project.type is manga but manuscript.chapters is also present"));
    }

    #[test]
    fn validate_reports_missing_alt_and_broken_link_in_prose() {
        let root = temp_dir("prose-missing-alt-broken-link");
        write_book_with_chapter_contents(
            &root,
            "# Chapter 1\n\n![ ](assets/missing.png)\n\n[See appendix](missing.md)\n",
            r#"validation:
  strict: true
  epubcheck: true
  missing_alt: error
  broken_link: warn"#,
        );

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("image is missing alt text"));
        assert!(report.contains("link target not found"));
        assert!(report.contains("\"severity\": \"warning\""));
    }

    #[test]
    fn validate_reports_heading_hierarchy_problems() {
        let root = temp_dir("prose-heading-hierarchy");
        write_book_with_chapter_contents(
            &root,
            "## Missing Title\n\n#### Too Deep\n",
            r#"validation:
  strict: true
  epubcheck: true
  accessibility: error"#,
        );

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("chapter file does not begin with a level-1 heading"));
        assert!(report.contains("heading hierarchy skips levels"));
    }

    #[test]
    fn validate_can_disable_accessibility_heading_checks() {
        let root = temp_dir("prose-accessibility-off");
        write_book_with_chapter_contents(
            &root,
            "## Missing Title\n\n#### Too Deep\n",
            r#"validation:
  strict: true
  epubcheck: true
  accessibility: off"#,
        );

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(!report.contains("chapter file does not begin with a level-1 heading"));
        assert!(!report.contains("heading hierarchy skips levels"));
    }
}
