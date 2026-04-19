use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use pulldown_cmark::{Event, Options as MarkdownOptions, Parser, Tag, TagEnd};
use regex::Regex;
use serde::Serialize;

use super::build_book::{BuildBookError, build_book_with_toolchain};
use crate::{
    cli_api::CommandContext,
    config,
    diagnostics::{IssueLocation, Severity, ValidationIssue},
    domain::{ProjectType, RepoPath},
    editorial,
    fs::join_repo_path,
    manga, markdown, pipeline,
    repo::{self, RepoError},
    toolchain::{self, ToolStatus},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ValidateBookResult {
    pub summary: String,
    pub plan: Option<pipeline::ValidatePlan>,
    pub report_path: PathBuf,
    pub manuscript_stats: Option<ManuscriptStats>,
    pub issues: Vec<ValidationIssue>,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManuscriptStats {
    pub total_characters: usize,
    pub frontmatter_characters: usize,
    pub chapter_characters: usize,
    pub backmatter_characters: usize,
    pub files: Vec<ManuscriptFileStats>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManuscriptFileStats {
    pub path: String,
    pub role: ManuscriptFileRole,
    pub characters: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManuscriptFileRole {
    Frontmatter,
    Chapter,
    Backmatter,
}

#[derive(Debug, Error)]
pub enum ValidateBookError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Editorial(#[from] editorial::EditorialError),
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

pub(crate) fn validate_book_with_toolchain(
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
        let editorial = if project_type.is_prose() {
            Some(editorial::load_bundle(&resolved)?)
        } else {
            None
        };
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
        let mut validator_runs = Vec::new();
        let mut manuscript_stats = None;
        if let Some(plan) = &plan {
            issues.extend(issues_from_checks(plan));
            issues.extend(schema_warning_issues(&resolved));
            issues.extend(match project_type {
                ProjectType::Manga => manga_validation_issues(&resolved, plan),
                _ => prose_validation_issues(&resolved, plan, editorial.as_ref()),
            });
            if project_type.is_prose() {
                manuscript_stats = Some(compute_manuscript_stats(&resolved, plan));
            }
            let external_validation = run_external_validators(command, &resolved, toolchain, plan);
            issues.extend(external_validation.issues);
            validator_runs = external_validation.runs;
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
            validator_runs,
            manuscript_stats: manuscript_stats.clone(),
            issues: issues.clone(),
        };
        write_report(&report_path, &report)?;
        let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);
        return Ok(ValidateBookResult {
            summary: format!(
                "validation completed for {} with outputs: {}, issues: {}, {}report: {}",
                book.id,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                },
                issues.len(),
                manuscript_stats
                    .as_ref()
                    .map(|stats| format!("manuscript characters: {}, ", stats.total_characters))
                    .unwrap_or_default(),
                report_path.display()
            ),
            plan,
            report_path,
            manuscript_stats,
            issues: issues.clone(),
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
    #[serde(rename = "validators")]
    validator_runs: Vec<ValidatorRunReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manuscript_stats: Option<ManuscriptStats>,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationCheckReport {
    name: String,
    target: String,
    tool: Option<String>,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
struct ValidatorRunReport {
    name: String,
    target: String,
    tool: String,
    status: String,
    summary: String,
    artifact: Option<String>,
    log_path: Option<String>,
}

fn compute_manuscript_stats(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::ValidatePlan,
) -> ManuscriptStats {
    let Some(manuscript) = resolved.effective.manuscript.as_ref() else {
        return ManuscriptStats {
            total_characters: 0,
            frontmatter_characters: 0,
            chapter_characters: 0,
            backmatter_characters: 0,
            files: Vec::new(),
        };
    };

    let frontmatter_paths = manuscript
        .frontmatter
        .iter()
        .map(|path| path.as_str())
        .collect::<BTreeSet<_>>();
    let chapter_paths = manuscript
        .chapters
        .iter()
        .map(|path| path.as_str())
        .collect::<BTreeSet<_>>();
    let backmatter_paths = manuscript
        .backmatter
        .iter()
        .map(|path| path.as_str())
        .collect::<BTreeSet<_>>();

    let mut stats = ManuscriptStats {
        total_characters: 0,
        frontmatter_characters: 0,
        chapter_characters: 0,
        backmatter_characters: 0,
        files: Vec::new(),
    };

    for file_path in &plan.manuscript_files {
        let repo_relative = relative_display(&resolved.repo.repo_root, file_path);
        let role = if frontmatter_paths.contains(repo_relative.as_str()) {
            ManuscriptFileRole::Frontmatter
        } else if chapter_paths.contains(repo_relative.as_str()) {
            ManuscriptFileRole::Chapter
        } else if backmatter_paths.contains(repo_relative.as_str()) {
            ManuscriptFileRole::Backmatter
        } else {
            continue;
        };

        let characters = fs::read_to_string(file_path)
            .ok()
            .map(|contents| markdown_character_count(&contents))
            .unwrap_or(0);

        stats.total_characters += characters;
        match role {
            ManuscriptFileRole::Frontmatter => stats.frontmatter_characters += characters,
            ManuscriptFileRole::Chapter => stats.chapter_characters += characters,
            ManuscriptFileRole::Backmatter => stats.backmatter_characters += characters,
        }
        stats.files.push(ManuscriptFileStats {
            path: repo_relative,
            role,
            characters,
        });
    }

    stats
}

fn markdown_character_count(contents: &str) -> usize {
    let body = match markdown::parse_frontmatter(contents) {
        Ok(Some(_)) => markdown::body_without_frontmatter(contents).unwrap_or(contents),
        Ok(None) | Err(_) => contents,
    };
    let mut image_depth = 0usize;
    let mut plain_text = String::new();

    for event in Parser::new_ext(body, MarkdownOptions::all()) {
        match event {
            Event::Start(Tag::Image { .. }) => image_depth += 1,
            Event::End(TagEnd::Image) => image_depth = image_depth.saturating_sub(1),
            Event::Text(text) | Event::Code(text) if image_depth == 0 => {
                plain_text.push_str(&text);
            }
            _ => {}
        }
    }

    plain_text.chars().count()
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

#[derive(Debug, Default)]
struct ExternalValidationResult {
    runs: Vec<ValidatorRunReport>,
    issues: Vec<ValidationIssue>,
}

fn run_external_validators(
    command: &CommandContext,
    resolved: &config::ResolvedBookConfig,
    toolchain: &toolchain::ToolchainReport,
    plan: &pipeline::ValidatePlan,
) -> ExternalValidationResult {
    let mut result = ExternalValidationResult::default();

    if should_run_epubcheck(command, resolved) {
        let (run, issue) = run_epubcheck_validation(command, resolved, toolchain, plan);
        result.runs.push(run);
        if let Some(issue) = issue {
            result.issues.push(issue);
        }
    }

    result
}

fn should_run_epubcheck(command: &CommandContext, resolved: &config::ResolvedBookConfig) -> bool {
    (command.output_target.is_none() || command.output_target.as_deref() == Some("kindle"))
        && resolved.effective.outputs.kindle.is_some()
        && resolved.effective.validation.epubcheck
}

fn run_epubcheck_validation(
    command: &CommandContext,
    resolved: &config::ResolvedBookConfig,
    toolchain: &toolchain::ToolchainReport,
    plan: &pipeline::ValidatePlan,
) -> (ValidatorRunReport, Option<ValidationIssue>) {
    let book = resolved
        .repo
        .book
        .as_ref()
        .expect("book context must exist for validation");
    let repo_root = &resolved.repo.repo_root;
    let tool_name = "epubcheck".to_string();
    let initial = ValidatorRunReport {
        name: "epubcheck".to_string(),
        target: "kindle".to_string(),
        tool: tool_name.clone(),
        status: "skipped".to_string(),
        summary: "epubcheck was not run".to_string(),
        artifact: None,
        log_path: None,
    };

    let Some(executable) = available_tool_path(toolchain, "epubcheck") else {
        return (
            ValidatorRunReport {
                summary: "epubcheck executable is unavailable".to_string(),
                ..initial
            },
            None,
        );
    };

    if plan
        .checks
        .iter()
        .any(|check| check.target == "kindle" && check.tool_status == ToolStatus::Missing)
    {
        return (
            ValidatorRunReport {
                summary: "kindle validation prerequisites are missing".to_string(),
                ..initial
            },
            None,
        );
    }

    let build_command = CommandContext::new(
        command.start_path.clone(),
        command.book_id.clone(),
        Some("kindle".to_string()),
    );
    let build_result = match build_book_with_toolchain(&build_command, toolchain) {
        Ok(result) => result,
        Err(BuildBookError::RequiredToolMissing { tool, .. }) => {
            return (
                ValidatorRunReport {
                    summary: format!("skipped because required build tool `{tool}` is missing"),
                    ..initial
                },
                None,
            );
        }
        Err(BuildBookError::ExecutionFailed { log_path, .. }) => {
            let issue = ValidationIssue::error(
                "kindle",
                "kindle validation prerequisite build failed".to_string(),
                format!(
                    "生成した EPUB を検証できませんでした。build log を確認してください: {}",
                    relative_display(repo_root, &log_path)
                ),
            );
            return (
                ValidatorRunReport {
                    status: "failed".to_string(),
                    summary: "kindle build failed before epubcheck".to_string(),
                    log_path: Some(relative_display(repo_root, &log_path)),
                    ..initial
                },
                Some(issue),
            );
        }
        Err(error) => {
            let issue = ValidationIssue::error(
                "kindle",
                format!("kindle validation prerequisite build failed: {error}"),
                "kindle build を通してから validate を再実行してください。".to_string(),
            );
            return (
                ValidatorRunReport {
                    status: "failed".to_string(),
                    summary: format!("kindle build failed before epubcheck: {error}"),
                    ..initial
                },
                Some(issue),
            );
        }
    };

    let Some(artifact) = build_result.artifacts.first() else {
        let issue = ValidationIssue::error(
            "kindle",
            "epubcheck input artifact was not generated".to_string(),
            "kindle build の成果物生成を確認してください。".to_string(),
        );
        return (
            ValidatorRunReport {
                status: "failed".to_string(),
                summary: "kindle build did not produce an EPUB artifact".to_string(),
                ..initial
            },
            Some(issue),
        );
    };

    let log_path = validator_log_path(repo_root, &book.id, "epubcheck");
    match toolchain::run_epubcheck(executable, artifact) {
        Ok(output) => {
            let _ = write_validator_log(&log_path, "epubcheck", artifact, &output);
            if output.status.success() {
                (
                    ValidatorRunReport {
                        status: "passed".to_string(),
                        summary: "epubcheck passed on the generated EPUB".to_string(),
                        artifact: Some(relative_display(repo_root, artifact)),
                        log_path: Some(relative_display(repo_root, &log_path)),
                        ..initial
                    },
                    None,
                )
            } else {
                let issue = ValidationIssue::error(
                    "kindle",
                    "epubcheck failed for the generated EPUB".to_string(),
                    format!(
                        "epubcheck log を確認してください: {}",
                        relative_display(repo_root, &log_path)
                    ),
                );
                (
                    ValidatorRunReport {
                        status: "failed".to_string(),
                        summary: "epubcheck reported validation errors".to_string(),
                        artifact: Some(relative_display(repo_root, artifact)),
                        log_path: Some(relative_display(repo_root, &log_path)),
                        ..initial
                    },
                    Some(issue),
                )
            }
        }
        Err(error) => {
            let issue = ValidationIssue::error(
                "kindle",
                format!("failed to start epubcheck: {error}"),
                "epubcheck が起動できることを確認してください。".to_string(),
            );
            (
                ValidatorRunReport {
                    status: "failed".to_string(),
                    summary: format!("failed to start epubcheck: {error}"),
                    artifact: Some(relative_display(repo_root, artifact)),
                    log_path: Some(relative_display(repo_root, &log_path)),
                    ..initial
                },
                Some(issue),
            )
        }
    }
}

fn available_tool_path<'a>(
    toolchain: &'a toolchain::ToolchainReport,
    key: &str,
) -> Option<&'a Path> {
    toolchain.tool(key).and_then(|tool| match tool.status {
        ToolStatus::Available => tool.resolved_path.as_deref(),
        _ => None,
    })
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
            .at(config_path.clone()),
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
            .at(config_path.clone()),
        );
    }

    if project_type.is_prose()
        && resolved.effective.outputs.print.is_some()
        && resolved.effective.book.writing_mode == config::WritingMode::VerticalRl
        && resolved
            .effective
            .pdf
            .as_ref()
            .map(|settings| settings.engine == config::PdfEngine::Weasyprint)
            .unwrap_or(false)
    {
        issues.push(
            ValidationIssue::error(
                "print",
                "pdf.engine = weasyprint does not support vertical-rl prose print".to_string(),
                "縦組み prose の print build では pdf.engine を chromium にしてください。",
            )
            .at(config_path.clone()),
        );
    }

    if project_type == ProjectType::Manga
        && resolved.effective.outputs.kindle.as_deref() == Some("kindle-ja")
    {
        issues.push(
            ValidationIssue::warning(
                "kindle",
                "manga projects usually expect outputs.kindle.target = kindle-comic".to_string(),
                "manga では outputs.kindle.target を kindle-comic にしてください。kindle-ja は v0.1 では将来互換扱いです。",
            )
            .at(config_path.clone()),
        );
    }

    if project_type == ProjectType::Manga
        && resolved.effective.outputs.print.as_deref() != Some("print-manga")
        && resolved.effective.outputs.print.is_some()
    {
        issues.push(
            ValidationIssue::warning(
                "print",
                "manga projects usually expect outputs.print.target = print-manga".to_string(),
                "manga では outputs.print.target を print-manga にしてください。印刷向け PDF はページ画像ベースの本文経路を前提にしています。",
            )
            .at(config_path.clone()),
        );
    }

    if let (Some(print_target), Some(print)) = (
        resolved.effective.outputs.print.as_deref(),
        resolved.effective.print.as_ref(),
    ) {
        let expected_pdf_standard = match print_target {
            "print-jp-pdfx1a" => Some(config::PrintPdfStandard::Pdfx1a),
            "print-jp-pdfx4" => Some(config::PrintPdfStandard::Pdfx4),
            _ => None,
        };
        if let Some(expected_pdf_standard) = expected_pdf_standard
            && print.pdf_standard != expected_pdf_standard
        {
            issues.push(
                ValidationIssue::warning(
                    "print",
                    format!(
                        "print target {print_target} usually expects print.pdf_standard = {}",
                        expected_pdf_standard.as_str()
                    ),
                    format!(
                        "{print_target} を使う場合は print.pdf_standard を {} にしてください。",
                        expected_pdf_standard.as_str()
                    ),
                )
                .at(config_path.clone()),
            );
        }
    }

    if project_type.is_prose()
        && resolved.effective.outputs.print.is_some()
        && let Some(pdf) = resolved.effective.pdf.as_ref()
        && matches!(
            pdf.engine,
            config::PdfEngine::Typst | config::PdfEngine::Lualatex
        )
    {
        issues.push(
            ValidationIssue::warning(
                "print",
                format!(
                    "pdf.engine = {} is accepted but less validated in v0.1",
                    pdf.engine.as_str()
                ),
                format!(
                    "{} を使う場合は handoff 前に preview / print build / proof を追加確認してください。",
                    pdf.engine.as_str()
                ),
            )
            .at(config_path.clone()),
        );
    }

    if resolved.effective.book.profile == "conference-preprint" {
        let pdf = resolved.effective.pdf.as_ref();
        let print = resolved.effective.print.as_ref();
        let mut push_preprint_warning = |summary: &str, detail: &str| {
            issues.push(
                ValidationIssue::warning("print", summary.to_string(), detail.to_string())
                    .at(config_path.clone()),
            );
        };

        if resolved.effective.outputs.print.is_none() {
            push_preprint_warning(
                "conference-preprint profile usually expects print output",
                "conference-preprint では outputs.print.enabled を true にしてください。",
            );
        } else if resolved.effective.outputs.print.as_deref() != Some("print-jp-pdfx4") {
            push_preprint_warning(
                "conference-preprint profile usually expects outputs.print.target = print-jp-pdfx4",
                "conference-preprint では outputs.print.target を print-jp-pdfx4 にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.engine != config::PdfEngine::Weasyprint)
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.engine = weasyprint",
                "conference-preprint では pdf.engine を weasyprint にしてください。",
            );
        }

        if pdf.map(|settings| settings.toc).unwrap_or(true) {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.toc = false",
                "conference-preprint では pdf.toc を false にしてください。",
            );
        }

        if pdf.map(|settings| settings.page_number).unwrap_or(true) {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.page_number = false",
                "conference-preprint では pdf.page_number を false にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.running_header != config::PdfRunningHeader::None)
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.running_header = none",
                "conference-preprint では pdf.running_header を none にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.column_count != 2)
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.column_count = 2",
                "conference-preprint では pdf.column_count を 2 にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.column_gap != "10mm")
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.column_gap = 10mm",
                "conference-preprint では pdf.column_gap を 10mm にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.base_font_size != "9pt")
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.base_font_size = 9pt",
                "conference-preprint では pdf.base_font_size を 9pt にしてください。",
            );
        }

        if pdf
            .map(|settings| settings.line_height != "14pt")
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects pdf.line_height = 14pt",
                "conference-preprint では pdf.line_height を 14pt にしてください。",
            );
        }

        if print
            .map(|settings| settings.trim_size != config::PrintTrimSize::A4)
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects print.trim_size = A4",
                "conference-preprint では print.trim_size を A4 にしてください。",
            );
        }

        if print
            .map(|settings| settings.bleed != "0mm")
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects print.bleed = 0mm",
                "conference-preprint では print.bleed を 0mm にしてください。",
            );
        }

        if print.map(|settings| settings.crop_marks).unwrap_or(true) {
            push_preprint_warning(
                "conference-preprint profile usually expects print.crop_marks = false",
                "conference-preprint では print.crop_marks を false にしてください。",
            );
        }

        if print
            .and_then(|settings| settings.page_margin.as_ref())
            .map(|margin| {
                margin.top == "20mm"
                    && margin.bottom == "20mm"
                    && margin.left == "15mm"
                    && margin.right == "15mm"
            })
            != Some(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects print.page_margin = {top: 20mm, bottom: 20mm, left: 15mm, right: 15mm}",
                "conference-preprint では print.page_margin を top/bottom=20mm, left/right=15mm にしてください。",
            );
        }

        if print
            .map(|settings| settings.sides != config::PrintSides::Duplex)
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects print.sides = duplex",
                "conference-preprint では print.sides を duplex にしてください。",
            );
        }

        if print
            .map(|settings| settings.max_pages != Some(2))
            .unwrap_or(true)
        {
            push_preprint_warning(
                "conference-preprint profile usually expects print.max_pages = 2",
                "conference-preprint では print.max_pages を 2 にしてください。",
            );
        }
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
    editorial_bundle: Option<&editorial::EditorialBundle>,
) -> Vec<ValidationIssue> {
    let Some(manuscript) = resolved.effective.manuscript.as_ref() else {
        return Vec::new();
    };

    let chapter_paths = manuscript
        .chapters
        .iter()
        .map(|path| join_repo_path(&resolved.repo.repo_root, path))
        .collect::<Vec<_>>();
    let manuscript_repo_paths = resolved
        .manuscript_files()
        .into_iter()
        .map(|path| path.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let mut issues = Vec::new();
    let mut referenced_images = BTreeMap::<String, IssueLocation>::new();

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
                issues.push(issue_from_severity_at_location(
                    resolved.effective.validation.missing_alt,
                    if resolved.effective.outputs.kindle.is_some() {
                        "kindle"
                    } else {
                        "common"
                    },
                    format!("image is missing alt text: {}", image.destination),
                    "画像参照に代替テキストを追加してください。",
                    Some(IssueLocation::with_line(
                        file_path.to_path_buf(),
                        image.line,
                    )),
                ));
            }
            if !image.is_external
                && !resolved_path_exists(file_path, &resolved.repo.repo_root, &image.destination)
            {
                issues.push(issue_from_severity_at_location(
                    resolved.effective.validation.missing_image,
                    "common",
                    format!("image reference target not found: {}", image.destination),
                    "画像パスを修正するか、対象ファイルを追加してください。",
                    Some(IssueLocation::with_line(
                        file_path.to_path_buf(),
                        image.line,
                    )),
                ));
            }
        }

        for link in &analysis.links {
            if !link.is_external
                && !resolved_path_exists(file_path, &resolved.repo.repo_root, &link.destination)
            {
                issues.push(issue_from_severity_at_location(
                    resolved.effective.validation.broken_link,
                    "common",
                    format!("link target not found: {}", link.destination),
                    "リンク先パスを修正するか、対象ファイルを追加してください。",
                    Some(IssueLocation::with_line(file_path.to_path_buf(), link.line)),
                ));
            }
        }

        if let Some(editorial_bundle) = editorial_bundle {
            issues.extend(style_validation_issues(
                editorial_bundle.style.as_ref(),
                &analysis.contents,
                file_path,
            ));

            for image in &analysis.images {
                if !image.is_external
                    && let Some(repo_path) = resolve_destination_to_repo_path(
                        file_path,
                        &resolved.repo.repo_root,
                        &image.destination,
                    )
                {
                    referenced_images.entry(repo_path).or_insert_with(|| {
                        IssueLocation::with_line(file_path.to_path_buf(), image.line)
                    });
                }
            }
        }

        if chapter_paths.iter().any(|chapter| chapter == file_path) {
            if analysis.headings.first().map(|heading| heading.level) != Some(1)
                && let Some(issue) = accessibility_issue_at_location(
                    resolved,
                    "common",
                    "chapter file does not begin with a level-1 heading".to_string(),
                    "各 chapter ファイルの先頭に `#` 見出しを置き、navigation の導出元を明確にしてください。",
                    analysis
                        .headings
                        .first()
                        .map(|heading| {
                            IssueLocation::with_line(file_path.to_path_buf(), heading.line)
                        })
                        .unwrap_or_else(|| file_path.to_path_buf().into()),
                )
            {
                issues.push(issue);
            }
            for (previous, current) in analysis.headings.windows(2).filter_map(|levels| {
                if levels[1].level > levels[0].level + 1 {
                    Some((&levels[0], &levels[1]))
                } else {
                    None
                }
            }) {
                if let Some(issue) = accessibility_issue_at_location(
                    resolved,
                    "common",
                    format!(
                        "heading hierarchy skips levels: h{} -> h{}",
                        previous.level, current.level
                    ),
                    "見出しレベルを段階的に増やしてください。例: h1 の次は h2 を使います。",
                    IssueLocation::with_line(file_path.to_path_buf(), current.line),
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

    if let Some(editorial_bundle) = editorial_bundle {
        issues.extend(claim_validation_issues(
            editorial_bundle.claims.as_ref(),
            &manuscript_repo_paths,
        ));
        issues.extend(figure_validation_issues(
            resolved,
            editorial_bundle.figures.as_ref(),
            &referenced_images,
        ));
        issues.extend(freshness_validation_issues(editorial_bundle));
    }

    issues
}

#[derive(Debug, Clone)]
struct MarkdownLink {
    destination: String,
    alt: String,
    is_external: bool,
    line: usize,
}

#[derive(Debug, Clone)]
struct MarkdownHeading {
    level: u32,
    line: usize,
}

#[derive(Debug, Default)]
struct MarkdownAnalysis {
    contents: String,
    headings: Vec<MarkdownHeading>,
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
        contents: contents.clone(),
        headings: heading_regex
            .captures_iter(&contents)
            .map(|capture| MarkdownHeading {
                level: capture[1].len() as u32,
                line: capture
                    .get(0)
                    .map(|matched| line_number_from_offset(&contents, matched.start()))
                    .unwrap_or(1),
            })
            .collect(),
        images: image_regex
            .captures_iter(&contents)
            .map(|capture| {
                let destination = normalize_markdown_destination(&capture["dest"]);
                MarkdownLink {
                    is_external: is_external_destination(&destination),
                    destination,
                    alt: capture["alt"].to_string(),
                    line: capture
                        .get(0)
                        .map(|matched| line_number_from_offset(&contents, matched.start()))
                        .unwrap_or(1),
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
                    line: capture
                        .get(0)
                        .map(|matched| line_number_from_offset(&contents, matched.start()))
                        .unwrap_or(1),
                }
            })
            .collect(),
    })
}

fn style_validation_issues(
    style: Option<&editorial::LoadedStyleGuide>,
    contents: &str,
    file_path: &Path,
) -> Vec<ValidationIssue> {
    let Some(style) = style else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for rule in &style.data.preferred_terms {
        for alias in &rule.aliases {
            if !alias.is_empty() && alias != &rule.preferred && contents.contains(alias) {
                issues.push(
                    issue_from_rule_severity(
                        rule.severity,
                        "common",
                        format!(
                            "preferred term `{}` should replace `{}`",
                            rule.preferred, alias
                        ),
                        "style.yml の推奨表記に合わせて本文の表記を統一してください。",
                    )
                    .at_location(location_for_substring(file_path, contents, alias)),
                );
            }
        }
    }
    for rule in &style.data.banned_terms {
        if !rule.term.is_empty() && contents.contains(&rule.term) {
            let remedy = match rule.reason.as_deref() {
                Some(reason) if !reason.is_empty() => {
                    format!("禁止語を置き換えてください。理由: {reason}")
                }
                _ => "禁止語を style.yml の方針に沿って置き換えてください。".to_string(),
            };
            issues.push(
                issue_from_rule_severity(
                    rule.severity,
                    "common",
                    format!("banned term found: {}", rule.term),
                    remedy,
                )
                .at_location(location_for_substring(file_path, contents, &rule.term)),
            );
        }
    }
    issues
}

fn claim_validation_issues(
    claims: Option<&editorial::LoadedClaimLedger>,
    manuscript_repo_paths: &BTreeSet<String>,
) -> Vec<ValidationIssue> {
    let Some(claims) = claims else {
        return Vec::new();
    };

    let contents = fs::read_to_string(&claims.path).ok();
    let mut seen = BTreeSet::new();
    let mut issues = Vec::new();
    for claim in &claims.data.claims {
        let claim_location =
            yaml_field_location(&claims.path, contents.as_deref(), "id", &claim.id);
        if !seen.insert(claim.id.clone()) {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!("duplicate claim id in claim ledger: {}", claim.id),
                    "claims.yml の id を一意にしてください。",
                )
                .at_location(claim_location.clone()),
            );
        }
        match RepoPath::parse(claim.section.clone()) {
            Ok(section) => {
                if !manuscript_repo_paths.contains(section.as_str()) {
                    issues.push(
                        ValidationIssue::error(
                            "common",
                            format!("claim references a section that is not in manuscript: {}", claim.section),
                            "claims.yml の section を修正するか、対応する manuscript file を追加してください。",
                        )
                        .at_location(yaml_field_location(
                            &claims.path,
                            contents.as_deref(),
                            "section",
                            &claim.section,
                        )),
                    );
                }
            }
            Err(_) => {
                issues.push(
                    ValidationIssue::error(
                        "common",
                        format!(
                            "claim section is not a valid repo-relative path: {}",
                            claim.section
                        ),
                        "claims.yml の section は repo-relative かつ `/` 区切りにしてください。",
                    )
                    .at_location(yaml_field_location(
                        &claims.path,
                        contents.as_deref(),
                        "section",
                        &claim.section,
                    )),
                );
            }
        }
        if claim.sources.is_empty() {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!("claim is missing sources: {}", claim.id),
                    "claims.yml の sources に根拠 URL や資料識別子を追加してください。",
                )
                .at_location(claim_location),
            );
        }
    }
    issues
}

fn figure_validation_issues(
    resolved: &config::ResolvedBookConfig,
    figures: Option<&editorial::LoadedFigureLedger>,
    referenced_images: &BTreeMap<String, IssueLocation>,
) -> Vec<ValidationIssue> {
    let Some(figures) = figures else {
        return Vec::new();
    };

    let contents = fs::read_to_string(&figures.path).ok();
    let mut seen_ids = BTreeSet::new();
    let mut tracked_paths = BTreeSet::new();
    let mut issues = Vec::new();

    for figure in &figures.data.figures {
        let figure_id_location =
            yaml_field_location(&figures.path, contents.as_deref(), "id", &figure.id);
        if !seen_ids.insert(figure.id.clone()) {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!("duplicate figure id in figure ledger: {}", figure.id),
                    "figures.yml の id を一意にしてください。",
                )
                .at_location(figure_id_location.clone()),
            );
        }
        match RepoPath::parse(figure.path.clone()) {
            Ok(path) => {
                tracked_paths.insert(path.as_str().to_string());
                if !join_repo_path(&resolved.repo.repo_root, &path).is_file() {
                    issues.push(issue_from_severity_at_location(
                        resolved.effective.validation.missing_image,
                        "common",
                        format!("figure asset not found: {}", figure.path),
                        "figures.yml の path を修正するか、対応する asset を追加してください。",
                        Some(path_location_for_yaml_field(
                            &figures.path,
                            contents.as_deref(),
                            "path",
                            &figure.path,
                        )),
                    ));
                }
                if !referenced_images.contains_key(path.as_str()) {
                    issues.push(
                        ValidationIssue::warning(
                            "common",
                            format!(
                                "figure ledger entry is not referenced from manuscript: {}",
                                path.as_str()
                            ),
                            "未使用の図表 entry を削除するか、対応する画像参照を manuscript に追加してください。",
                        )
                        .at_location(figure_id_location.clone()),
                    );
                }
            }
            Err(_) => {
                issues.push(
                    ValidationIssue::error(
                        "common",
                        format!(
                            "figure path is not a valid repo-relative path: {}",
                            figure.path
                        ),
                        "figures.yml の path は repo-relative かつ `/` 区切りにしてください。",
                    )
                    .at_location(yaml_field_location(
                        &figures.path,
                        contents.as_deref(),
                        "path",
                        &figure.path,
                    )),
                );
            }
        }
        if figure
            .source
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            issues.push(
                ValidationIssue::warning(
                    "common",
                    format!("figure is missing source metadata: {}", figure.id),
                    "figures.yml の source を埋めて、図表の出典を明示してください。",
                )
                .at_location(figure_id_location),
            );
        }
    }

    for (path, source_location) in referenced_images {
        if !tracked_paths.contains(path) {
            issues.push(
                ValidationIssue::warning(
                    "common",
                    format!("manuscript image is not tracked in figure ledger: {}", path),
                    "figures.yml に図表 entry を追加するか、tracking 不要な画像利用方針を見直してください。",
                )
                .at_location(source_location.clone()),
            );
        }
    }

    issues
}

fn freshness_validation_issues(
    editorial_bundle: &editorial::EditorialBundle,
) -> Vec<ValidationIssue> {
    let Some(freshness) = editorial_bundle.freshness.as_ref() else {
        return Vec::new();
    };

    let contents = fs::read_to_string(&freshness.path).ok();
    let claim_ids = editorial_bundle
        .claims
        .as_ref()
        .map(|claims| {
            claims
                .data
                .claims
                .iter()
                .map(|claim| claim.id.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let figure_ids = editorial_bundle
        .figures
        .as_ref()
        .map(|figures| {
            figures
                .data
                .figures
                .iter()
                .map(|figure| figure.id.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let today = editorial::today_local();
    let mut issues = Vec::new();

    for item in &freshness.data.tracked {
        let item_id_location =
            yaml_field_location(&freshness.path, contents.as_deref(), "id", &item.id);
        let id_exists = match item.kind {
            editorial::FreshnessKind::Claim => claim_ids.contains(item.id.as_str()),
            editorial::FreshnessKind::Figure => figure_ids.contains(item.id.as_str()),
        };
        if !id_exists {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!(
                        "freshness entry references unknown {} id: {}",
                        item.kind.as_str(),
                        item.id
                    ),
                    "freshness.yml の id を修正するか、対応する claim / figure を追加してください。",
                )
                .at_location(item_id_location.clone()),
            );
        }

        let last_verified = match editorial::parse_iso_date(&item.last_verified) {
            Some(value) => value,
            None => {
                issues.push(
                    ValidationIssue::error(
                        "common",
                        format!(
                            "freshness last_verified must use YYYY-MM-DD: {}",
                            item.last_verified
                        ),
                        "freshness.yml の last_verified を YYYY-MM-DD 形式にしてください。",
                    )
                    .at_location(yaml_field_location(
                        &freshness.path,
                        contents.as_deref(),
                        "last_verified",
                        &item.last_verified,
                    )),
                );
                continue;
            }
        };
        let review_due_on = match editorial::parse_iso_date(&item.review_due_on) {
            Some(value) => value,
            None => {
                issues.push(
                    ValidationIssue::error(
                        "common",
                        format!(
                            "freshness review_due_on must use YYYY-MM-DD: {}",
                            item.review_due_on
                        ),
                        "freshness.yml の review_due_on を YYYY-MM-DD 形式にしてください。",
                    )
                    .at_location(yaml_field_location(
                        &freshness.path,
                        contents.as_deref(),
                        "review_due_on",
                        &item.review_due_on,
                    )),
                );
                continue;
            }
        };
        if review_due_on < last_verified {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!(
                        "freshness review_due_on is earlier than last_verified for {} {}",
                        item.kind.as_str(),
                        item.id
                    ),
                    "freshness.yml の日付順を見直してください。",
                )
                .at_location(yaml_field_location(
                    &freshness.path,
                    contents.as_deref(),
                    "review_due_on",
                    &item.review_due_on,
                )),
            );
        } else if review_due_on < today {
            issues.push(
                ValidationIssue::warning(
                    "common",
                    format!(
                        "freshness review is overdue for {} {}",
                        item.kind.as_str(),
                        item.id
                    ),
                    "release 前に根拠や図表の鮮度を再確認してください。",
                )
                .at_location(yaml_field_location(
                    &freshness.path,
                    contents.as_deref(),
                    "review_due_on",
                    &item.review_due_on,
                )),
            );
        }
    }

    issues
}

fn issue_from_rule_severity(
    severity: editorial::RuleSeverity,
    target: impl Into<String>,
    cause: impl Into<String>,
    remedy: impl Into<String>,
) -> ValidationIssue {
    match severity {
        editorial::RuleSeverity::Warn => ValidationIssue::warning(target, cause, remedy),
        editorial::RuleSeverity::Error => ValidationIssue::error(target, cause, remedy),
    }
}

fn location_for_substring(path: &Path, contents: &str, needle: &str) -> IssueLocation {
    line_number_of_substring(contents, needle)
        .map(|line| IssueLocation::with_line(path.to_path_buf(), line))
        .unwrap_or_else(|| path.to_path_buf().into())
}

fn yaml_field_location(
    path: &Path,
    contents: Option<&str>,
    field: &str,
    value: &str,
) -> IssueLocation {
    let patterns = [
        format!("{field}: {value}"),
        format!("{field}: \"{value}\""),
        format!("{field}: '{value}'"),
    ];
    location_for_patterns(path, contents, &patterns)
}

fn path_location_for_yaml_field(
    path: &Path,
    contents: Option<&str>,
    field: &str,
    value: &str,
) -> IssueLocation {
    yaml_field_location(path, contents, field, value)
}

fn location_for_patterns(
    path: &Path,
    contents: Option<&str>,
    patterns: &[String],
) -> IssueLocation {
    if let Some(contents) = contents {
        for pattern in patterns {
            if let Some(line) = line_number_of_substring(contents, pattern) {
                return IssueLocation::with_line(path.to_path_buf(), line);
            }
        }
    }
    path.to_path_buf().into()
}

fn line_number_of_substring(contents: &str, needle: &str) -> Option<usize> {
    contents
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
}

fn line_number_from_offset(contents: &str, offset: usize) -> usize {
    contents[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
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

fn resolve_destination_to_repo_path(
    source_path: &Path,
    repo_root: &Path,
    destination: &str,
) -> Option<String> {
    let base_destination = destination.split('#').next()?;
    if base_destination.is_empty() || is_external_destination(base_destination) {
        return None;
    }

    let mut normalized = PathBuf::new();

    if !base_destination.starts_with('/') {
        let source_parent = source_path.parent().unwrap_or(repo_root);
        let source_relative = source_parent.strip_prefix(repo_root).ok()?;
        for component in source_relative.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => normalized.push(part),
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
            }
        }
    }

    for component in Path::new(base_destination.trim_start_matches('/')).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if normalized.as_os_str().is_empty() {
        return None;
    }

    Some(normalized.to_string_lossy().replace('\\', "/"))
}

fn resolved_path_exists(source_path: &Path, repo_root: &Path, destination: &str) -> bool {
    let Some(base_destination) = destination.split('#').next() else {
        return true;
    };
    if base_destination.is_empty() {
        return true;
    }

    if let Some(repo_relative) =
        resolve_destination_to_repo_path(source_path, repo_root, destination)
        && let Ok(path) = RepoPath::parse(repo_relative)
    {
        return join_repo_path(repo_root, &path).exists();
    }

    false
}

fn accessibility_issue_at_location(
    resolved: &config::ResolvedBookConfig,
    target: impl Into<String>,
    cause: impl Into<String>,
    remedy: impl Into<String>,
    location: IssueLocation,
) -> Option<ValidationIssue> {
    match resolved.effective.validation.accessibility {
        config::ValidationLevel::Off => None,
        config::ValidationLevel::Warn => {
            Some(ValidationIssue::warning(target, cause, remedy).at_location(location))
        }
        config::ValidationLevel::Error => {
            Some(ValidationIssue::error(target, cause, remedy).at_location(location))
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

fn issue_from_severity_at_location(
    severity: config::ValidationSeverity,
    target: impl Into<String>,
    cause: impl Into<String>,
    remedy: impl Into<String>,
    location: Option<IssueLocation>,
) -> ValidationIssue {
    let issue = match severity {
        config::ValidationSeverity::Warn => ValidationIssue::warning(target, cause, remedy),
        config::ValidationSeverity::Error => ValidationIssue::error(target, cause, remedy),
    };
    match location {
        Some(location) => issue.at_location(location),
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

fn validator_log_path(repo_root: &Path, book_id: &str, validator: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("logs")
        .join(format!("{book_id}-{validator}-validate.log"))
}

fn write_validator_log(
    path: &Path,
    tool_name: &str,
    artifact: &Path,
    run_output: &toolchain::ToolRunOutput,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = format!(
        "tool: {tool_name}\nartifact: {}\nstatus: {}\nstdout:\n{}\n\nstderr:\n{}\n",
        artifact.display(),
        run_output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "signal".to_string()),
        run_output.stdout,
        run_output.stderr
    );
    fs::write(path, contents)
}

fn relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
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
        fake_toolchain_with_engines(
            epubcheck,
            ToolStatus::Available,
            ToolStatus::Available,
            ToolStatus::Available,
        )
    }

    fn fake_toolchain_with_engines(
        epubcheck: ToolStatus,
        weasyprint: ToolStatus,
        chromium: ToolStatus,
        typst: ToolStatus,
    ) -> ToolchainReport {
        ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: ToolStatus::Available,
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install pandoc and ensure it is available on PATH.".to_string(),
                },
                ToolRecord {
                    key: "epubcheck",
                    display_name: "epubcheck",
                    status: epubcheck,
                    detected_as: Some("epubcheck".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install epubcheck and ensure the launcher is available on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "weasyprint",
                    display_name: "weasyprint",
                    status: weasyprint,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install weasyprint and ensure the launcher is on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "chromium",
                    display_name: "Chromium PDF",
                    status: chromium,
                    detected_as: Some("chromium".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install a Chromium-based browser and ensure its executable is available."
                            .to_string(),
                },
                ToolRecord {
                    key: "typst",
                    display_name: "typst",
                    status: typst,
                    detected_as: Some("typst".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install typst and ensure the launcher is on PATH.".to_string(),
                },
                ToolRecord {
                    key: "pdf-engine",
                    display_name: "PDF engine",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install one supported PDF engine such as weasyprint, Chromium, typst, or lualatex."
                            .to_string(),
                },
            ],
        }
    }

    #[cfg(unix)]
    fn fake_toolchain_with_paths(
        pandoc: Option<PathBuf>,
        epubcheck: Option<PathBuf>,
        epubcheck_status: ToolStatus,
    ) -> ToolchainReport {
        ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: if pandoc.is_some() {
                        ToolStatus::Available
                    } else {
                        ToolStatus::Missing
                    },
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: pandoc,
                    version: None,
                    install_hint: "Install pandoc and ensure it is available on PATH.".to_string(),
                },
                ToolRecord {
                    key: "epubcheck",
                    display_name: "epubcheck",
                    status: epubcheck_status,
                    detected_as: Some("epubcheck".to_string()),
                    resolved_path: epubcheck,
                    version: None,
                    install_hint: "Install epubcheck and ensure the launcher is available on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "weasyprint",
                    display_name: "weasyprint",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install weasyprint and ensure the launcher is on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "chromium",
                    display_name: "Chromium PDF",
                    status: ToolStatus::Available,
                    detected_as: Some("chromium".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install a Chromium-based browser and ensure its executable is available."
                            .to_string(),
                },
                ToolRecord {
                    key: "typst",
                    display_name: "typst",
                    status: ToolStatus::Available,
                    detected_as: Some("typst".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install typst and ensure the launcher is on PATH.".to_string(),
                },
                ToolRecord {
                    key: "pdf-engine",
                    display_name: "PDF engine",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install one supported PDF engine such as weasyprint, Chromium, typst, or lualatex."
                            .to_string(),
                },
            ],
        }
    }

    #[cfg(unix)]
    fn write_fake_pandoc(root: &std::path::Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let pandoc = root.join("pandoc");
        fs::write(
            &pandoc,
            r#"#!/bin/sh
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf 'fake epub' > "$out"
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&pandoc, permissions).unwrap();
        pandoc
    }

    #[cfg(unix)]
    fn write_fake_epubcheck(
        root: &std::path::Path,
        exit_code: i32,
        args_path: &std::path::Path,
    ) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let epubcheck = root.join("epubcheck");
        fs::write(
            &epubcheck,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{}"
echo "fake epubcheck"
exit {}
"#,
                args_path.display(),
                exit_code
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&epubcheck).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&epubcheck, permissions).unwrap();
        epubcheck
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

    #[test]
    fn markdown_character_count_ignores_frontmatter_and_markdown_syntax() {
        let count = markdown_character_count(
            "---\nstatus: draft\n---\n# 章題\n本文と[リンク](notes.md)\n![図](assets/figure.png)\n",
        );

        assert_eq!(count, "章題本文とリンク".chars().count());
    }

    #[test]
    fn markdown_character_count_keeps_leading_non_yaml_blocks() {
        let count = markdown_character_count("---\n# Interlude\n---\n本文\n");

        assert!(count > markdown_character_count("本文\n"));
    }

    #[test]
    fn validate_includes_manuscript_stats_for_prose_books() {
        let root = temp_dir("manuscript-stats");
        write_book_with_chapter_contents(
            &root,
            "---\nstatus: draft\n---\n# 章題\n本文と[リンク](notes.md)\n",
            "validation:\n  strict: true\n  epubcheck: false\n",
        );

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Missing),
        )
        .unwrap();

        let stats = result.manuscript_stats.as_ref().expect("manuscript stats");
        assert_eq!(stats.total_characters, "章題本文とリンク".chars().count());
        assert_eq!(stats.chapter_characters, "章題本文とリンク".chars().count());
        assert_eq!(stats.files.len(), 1);
        assert_eq!(stats.files[0].path, "manuscript/01.md");
        assert_eq!(stats.files[0].role, ManuscriptFileRole::Chapter);
        assert!(result.summary.contains("manuscript characters: 8"));

        let report: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(result.report_path).unwrap()).unwrap();
        assert_eq!(report["manuscript_stats"]["total_characters"], 8);
        assert_eq!(report["manuscript_stats"]["chapter_characters"], 8);
        assert_eq!(
            report["manuscript_stats"]["files"][0]["path"],
            "manuscript/01.md"
        );
        assert_eq!(report["manuscript_stats"]["files"][0]["role"], "chapter");
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

    fn write_book_with_editorial(root: &std::path::Path) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::create_dir_all(root.join("editorial")).unwrap();
        fs::create_dir_all(root.join("assets/images")).unwrap();
        fs::write(
            root.join("manuscript/01.md"),
            "# Chapter 1\nUse git in the workflow.\n![Architecture](../assets/images/architecture.png)\n",
        )
        .unwrap();
        fs::write(root.join("assets/images/architecture.png"), tiny_png()).unwrap();
        fs::write(
            root.join("editorial/style.yml"),
            r#"
preferred_terms:
  - preferred: "Git"
    aliases:
      - "git"
    severity: warn
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/claims.yml"),
            r#"
claims:
  - id: claim-1
    summary: "Git を使う"
    section: manuscript/01.md
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/figures.yml"),
            r#"
figures:
  - id: fig-architecture
    path: assets/images/architecture.png
    caption: "Architecture"
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/freshness.yml"),
            r#"
tracked:
  - kind: claim
    id: claim-1
    last_verified: 1999-01-01
    review_due_on: 2000-01-01
"#,
        )
        .unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: business
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: ltr
layout:
  binding: left
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
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
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

    #[cfg(unix)]
    #[test]
    fn validate_runs_epubcheck_against_generated_kindle_artifact() {
        let root = temp_dir("epubcheck-success");
        write_book_with_cover(&root, "error", true);
        let pandoc = write_fake_pandoc(&root);
        let args_path = root.join("epubcheck-args.txt");
        let epubcheck = write_fake_epubcheck(&root, 0, &args_path);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_paths(Some(pandoc), Some(epubcheck), ToolStatus::Available),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(&result.report_path).unwrap();
        assert!(report.contains("\"validators\""));
        assert!(report.contains("\"status\": \"passed\""));
        assert!(report.contains("\"artifact\":"));
        assert!(report.contains(".epub"));
        let args = fs::read_to_string(args_path).unwrap();
        assert!(args.contains(".epub"));
    }

    #[cfg(unix)]
    #[test]
    fn validate_reports_epubcheck_failures() {
        let root = temp_dir("epubcheck-failure");
        write_book_with_cover(&root, "error", true);
        let pandoc = write_fake_pandoc(&root);
        let args_path = root.join("epubcheck-args.txt");
        let epubcheck = write_fake_epubcheck(&root, 2, &args_path);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_paths(Some(pandoc), Some(epubcheck), ToolStatus::Available),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(&result.report_path).unwrap();
        assert!(report.contains("epubcheck failed for the generated EPUB"));
        assert!(report.contains("\"status\": \"failed\""));
        assert!(
            root.join("dist/logs/default-epubcheck-validate.log")
                .is_file()
        );
        let args = fs::read_to_string(args_path).unwrap();
        assert!(args.contains(".epub"));
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
            &fake_toolchain(ToolStatus::Available),
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
    fn validate_warns_when_conference_preprint_preset_is_incomplete() {
        let root = temp_dir("conference-preprint-warnings");
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::create_dir_all(root.join("assets/cover")).unwrap();
        fs::write(root.join("manuscript/01-main.md"), "# Main\n").unwrap();
        fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: paper
  vcs: git
book:
  title: "Sample Preprint"
  authors:
    - "Author"
  profile: conference-preprint
  reading_direction: ltr
manuscript:
  chapters:
    - manuscript/01-main.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
  print:
    enabled: false
cover:
  ebook_image: assets/cover/front.png
pdf:
  engine: typst
  column_count: 1
print:
  trim_size: B6
  max_pages: 3
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(!result.has_errors, "{report}");
        assert!(report.contains("conference-preprint profile usually expects print output"));
        assert!(
            report.contains("conference-preprint profile usually expects pdf.engine = weasyprint")
        );
        assert!(report.contains("conference-preprint profile usually expects pdf.toc = false"));
        assert!(
            report.contains("conference-preprint profile usually expects pdf.page_number = false")
        );
        assert!(
            report
                .contains("conference-preprint profile usually expects pdf.running_header = none")
        );
        assert!(
            report.contains("conference-preprint profile usually expects pdf.column_count = 2")
        );
        assert!(
            report.contains("conference-preprint profile usually expects pdf.column_gap = 10mm")
        );
        assert!(
            report.contains("conference-preprint profile usually expects pdf.base_font_size = 9pt")
        );
        assert!(
            report.contains("conference-preprint profile usually expects pdf.line_height = 14pt")
        );
        assert!(
            report.contains("conference-preprint profile usually expects print.trim_size = A4")
        );
        assert!(report.contains("conference-preprint profile usually expects print.bleed = 0mm"));
        assert!(
            report.contains("conference-preprint profile usually expects print.crop_marks = false")
        );
        assert!(report.contains(
            "conference-preprint profile usually expects print.page_margin = {top: 20mm, bottom: 20mm, left: 15mm, right: 15mm}"
        ));
        assert!(
            report.contains("conference-preprint profile usually expects print.sides = duplex")
        );
        assert!(report.contains("conference-preprint profile usually expects print.max_pages = 2"));
    }

    #[test]
    fn validate_warns_when_conference_preprint_uses_non_pdfx4_target() {
        let root = temp_dir("conference-preprint-target-warning");
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01-main.md"), "# Main\n").unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: paper
  vcs: git
book:
  title: "Sample Preprint"
  authors:
    - "Author"
  profile: conference-preprint
  reading_direction: ltr
layout:
  binding: left
manuscript:
  chapters:
    - manuscript/01-main.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: weasyprint
  toc: false
  page_number: false
  running_header: none
  column_count: 2
  column_gap: 10mm
  base_font_size: 9pt
  line_height: 14pt
print:
  trim_size: A4
  bleed: 0mm
  crop_marks: false
  page_margin:
    top: 20mm
    bottom: 20mm
    left: 15mm
    right: 15mm
  sides: duplex
  max_pages: 2
  pdf_standard: pdfx1a
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains(
            "conference-preprint profile usually expects outputs.print.target = print-jp-pdfx4"
        ));
    }

    #[test]
    fn validate_warns_when_print_target_and_pdf_standard_do_not_match() {
        let root = temp_dir("print-target-pdf-standard-mismatch");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: chromium
print:
  pdf_standard: pdfx4
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(
            report.contains(
                "print target print-jp-pdfx1a usually expects print.pdf_standard = pdfx1a"
            )
        );
    }

    #[test]
    fn validate_reports_missing_configured_pdf_engine() {
        let root = temp_dir("missing-configured-pdf-engine");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: typst
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_engines(
                ToolStatus::Missing,
                ToolStatus::Available,
                ToolStatus::Available,
                ToolStatus::Missing,
            ),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("required validation tool is missing: typst"));
    }

    #[test]
    fn validate_rejects_vertical_weasyprint_print() {
        let root = temp_dir("vertical-weasyprint");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: weasyprint
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(
            report.contains("pdf.engine = weasyprint does not support vertical-rl prose print")
        );
        assert!(report.contains("pdf.engine を chromium にしてください"));
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
    fn validate_warns_when_manga_uses_legacy_kindle_target() {
        let root = temp_dir("manga-legacy-kindle-target");
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
outputs:
  kindle:
    enabled: true
    target: kindle-ja
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
        assert!(
            report.contains("manga projects usually expect outputs.kindle.target = kindle-comic")
        );
    }

    #[test]
    fn validate_warns_when_manga_uses_prose_print_target() {
        let root = temp_dir("manga-prose-print-target");
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
        assert!(
            report.contains("manga projects usually expect outputs.print.target = print-manga")
        );
    }

    #[test]
    fn validate_warns_when_prose_uses_less_validated_pdf_engine() {
        let root = temp_dir("prose-typst-warning");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: typst
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        assert!(!result.has_errors);
        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("pdf.engine = typst is accepted but less validated in v0.1"));
        assert!(report.contains("handoff 前に preview / print build / proof を追加確認"));
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
        assert!(report.contains("\"line\": 3"));
        assert!(report.contains("\"line\": 5"));

        let missing_alt = result
            .issues
            .iter()
            .find(|issue| issue.cause.contains("image is missing alt text"))
            .unwrap();
        assert_eq!(
            missing_alt.location.as_ref().map(|location| location.line),
            Some(Some(3))
        );

        let missing_image = result
            .issues
            .iter()
            .find(|issue| issue.cause.contains("image reference target not found"))
            .unwrap();
        assert_eq!(
            missing_image
                .location
                .as_ref()
                .map(|location| location.line),
            Some(Some(3))
        );

        let broken_link = result
            .issues
            .iter()
            .find(|issue| issue.cause.contains("link target not found"))
            .unwrap();
        assert_eq!(
            broken_link.location.as_ref().map(|location| location.line),
            Some(Some(5))
        );
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
        assert!(report.contains("\"line\": 1"));
        assert!(report.contains("\"line\": 3"));

        let missing_h1 = result
            .issues
            .iter()
            .find(|issue| {
                issue
                    .cause
                    .contains("chapter file does not begin with a level-1 heading")
            })
            .unwrap();
        assert_eq!(
            missing_h1.location.as_ref().map(|location| location.line),
            Some(Some(1))
        );

        let skipped_heading = result
            .issues
            .iter()
            .find(|issue| issue.cause.contains("heading hierarchy skips levels"))
            .unwrap();
        assert_eq!(
            skipped_heading
                .location
                .as_ref()
                .map(|location| location.line),
            Some(Some(3))
        );
    }

    #[test]
    fn validate_reports_editorial_issues_for_prose_books() {
        let root = temp_dir("prose-editorial");
        write_book_with_editorial(&root);

        let result = validate_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(ToolStatus::Available),
        )
        .unwrap();

        let report = fs::read_to_string(result.report_path).unwrap();
        assert!(report.contains("preferred term `Git` should replace `git`"));
        assert!(report.contains("claim is missing sources: claim-1"));
        assert!(report.contains("figure is missing source metadata: fig-architecture"));
        assert!(report.contains("freshness review is overdue for claim claim-1"));
        assert!(report.contains("\"line\": 2"));
        assert!(report.contains("\"line\": 3"));
        assert!(report.contains("\"line\": 6"));
        assert!(
            !report.contains("image reference target not found: ../assets/images/architecture.png")
        );
        assert!(!report.contains(
            "figure ledger entry is not referenced from manuscript: assets/images/architecture.png"
        ));
        assert!(!report.contains(
            "manuscript image is not tracked in figure ledger: assets/images/architecture.png"
        ));

        let style_issue = result
            .issues
            .iter()
            .find(|issue| {
                issue
                    .cause
                    .contains("preferred term `Git` should replace `git`")
            })
            .unwrap();
        assert_eq!(
            style_issue.location.as_ref().map(|location| location.line),
            Some(Some(2))
        );

        let claim_issue = result
            .issues
            .iter()
            .find(|issue| issue.cause.contains("claim is missing sources: claim-1"))
            .unwrap();
        assert_eq!(
            claim_issue.location.as_ref().map(|location| location.line),
            Some(Some(3))
        );

        let figure_issue = result
            .issues
            .iter()
            .find(|issue| {
                issue
                    .cause
                    .contains("figure is missing source metadata: fig-architecture")
            })
            .unwrap();
        assert_eq!(
            figure_issue.location.as_ref().map(|location| location.line),
            Some(Some(3))
        );

        let freshness_issue = result
            .issues
            .iter()
            .find(|issue| {
                issue
                    .cause
                    .contains("freshness review is overdue for claim claim-1")
            })
            .unwrap();
        assert_eq!(
            freshness_issue
                .location
                .as_ref()
                .map(|location| location.line),
            Some(Some(6))
        );
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
