use std::{fs, path::PathBuf};

use serde::Serialize;

use crate::{
    cli_api::CommandContext,
    config,
    diagnostics::{Severity, ValidationIssue},
    domain::ProjectType,
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
        let (plan, mut issues) = match match project_type {
            ProjectType::Manga => {
                pipeline::manga_validate_plan_with_toolchain(context, &resolved, toolchain)
            }
            _ => pipeline::prose_validate_plan_with_toolchain(context, &resolved, toolchain),
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
        if let Some(plan) = &plan {
            issues.extend(issues_from_checks(plan));
            issues.extend(schema_warning_issues(&resolved));
            if project_type == ProjectType::Manga {
                issues.extend(manga_validation_issues(&resolved, plan));
            }
        }
        let outputs = resolved.outputs();
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

    issues
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
            tools: vec![ToolRecord {
                key: "epubcheck",
                display_name: "epubcheck",
                status: epubcheck,
                resolved_path: None,
                version: None,
            }],
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
            &CommandContext::new(&root, None),
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
    fn validate_manga_reports_missing_page_directory() {
        let root = temp_dir("manga-missing-pages");
        write_manga_book(&root);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
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
            &CommandContext::new(&root, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("project.type is manga but manuscript.chapters is also present"));
    }
}
