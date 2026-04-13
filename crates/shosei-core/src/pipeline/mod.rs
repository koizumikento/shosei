use std::path::PathBuf;

use thiserror::Error;

use crate::{
    config::ResolvedBookConfig,
    diagnostics::Diagnostic,
    domain::RepoContext,
    fs::join_repo_path,
    manga,
    toolchain::{self, ToolStatus},
};

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub context: RepoContext,
    pub manuscript_files: Vec<PathBuf>,
    pub outputs: Vec<BuildOutputPlan>,
    pub stages: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutputPlan {
    pub channel: &'static str,
    pub target: String,
    pub artifact_path: PathBuf,
    pub primary_tool: &'static str,
    pub tool_status: ToolStatus,
}

#[derive(Debug, Clone)]
pub struct ValidatePlan {
    pub context: RepoContext,
    pub manuscript_files: Vec<PathBuf>,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCheck {
    pub name: &'static str,
    pub target: &'static str,
    pub tool: Option<&'static str>,
    pub tool_status: ToolStatus,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("{command} preflight failed: {summary}")]
    PreflightFailed {
        command: &'static str,
        summary: String,
        diagnostics: Vec<Diagnostic>,
    },
}

impl PipelineError {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::PreflightFailed { diagnostics, .. } => diagnostics,
        }
    }
}

pub fn prose_build_plan(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
) -> Result<BuildPlan, PipelineError> {
    let toolchain = toolchain::inspect_default_toolchain();
    prose_build_plan_with_toolchain(context, resolved, &toolchain)
}

pub fn prose_build_plan_with_toolchain(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
    toolchain: &toolchain::ToolchainReport,
) -> Result<BuildPlan, PipelineError> {
    let manuscript_files = collect_manuscript_files(resolved);
    let diagnostics = manuscript_file_diagnostics(&manuscript_files);
    if !diagnostics.is_empty() {
        return Err(preflight_failed("build", diagnostics));
    }

    Ok(BuildPlan {
        context,
        manuscript_files,
        outputs: build_outputs(resolved, toolchain),
        stages: vec![
            "resolve-config",
            "prepare-manuscript",
            "invoke-pandoc",
            "collect-artifacts",
        ],
    })
}

pub fn prose_validate_plan(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
) -> Result<ValidatePlan, PipelineError> {
    let toolchain = toolchain::inspect_default_toolchain();
    prose_validate_plan_with_toolchain(context, resolved, &toolchain)
}

pub fn prose_validate_plan_with_toolchain(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
    toolchain: &toolchain::ToolchainReport,
) -> Result<ValidatePlan, PipelineError> {
    let manuscript_files = collect_manuscript_files(resolved);
    let diagnostics = manuscript_file_diagnostics(&manuscript_files);
    if !diagnostics.is_empty() {
        return Err(preflight_failed("validate", diagnostics));
    }

    let mut checks = vec![ValidationCheck {
        name: "common-lint",
        target: "common",
        tool: None,
        tool_status: ToolStatus::Planned,
    }];
    if resolved.effective.outputs.kindle.is_some() {
        checks.push(ValidationCheck {
            name: "kindle-target-check",
            target: "kindle",
            tool: None,
            tool_status: ToolStatus::Planned,
        });
        if resolved.effective.validation.epubcheck {
            checks.push(ValidationCheck {
                name: "epubcheck",
                target: "kindle",
                tool: Some("epubcheck"),
                tool_status: toolchain
                    .tool("epubcheck")
                    .map(|tool| tool.status)
                    .unwrap_or(ToolStatus::Missing),
            });
        }
    }
    if resolved.effective.outputs.print.is_some() {
        checks.push(ValidationCheck {
            name: "print-target-check",
            target: "print",
            tool: None,
            tool_status: ToolStatus::Planned,
        });
    }

    Ok(ValidatePlan {
        context,
        manuscript_files,
        checks,
    })
}

pub fn manga_build_plan(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
) -> Result<BuildPlan, PipelineError> {
    let toolchain = toolchain::inspect_default_toolchain();
    manga_build_plan_with_toolchain(context, resolved, &toolchain)
}

pub fn manga_build_plan_with_toolchain(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
    _toolchain: &toolchain::ToolchainReport,
) -> Result<BuildPlan, PipelineError> {
    let page_files = manga_page_files("build", &context)?;
    Ok(BuildPlan {
        context,
        manuscript_files: page_files,
        outputs: manga_build_outputs(resolved),
        stages: vec![
            "resolve-config",
            "resolve-page-manifest",
            "validate-images",
            "package-target",
        ],
    })
}

pub fn manga_validate_plan(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
) -> Result<ValidatePlan, PipelineError> {
    let toolchain = toolchain::inspect_default_toolchain();
    manga_validate_plan_with_toolchain(context, resolved, &toolchain)
}

pub fn manga_validate_plan_with_toolchain(
    context: RepoContext,
    resolved: &ResolvedBookConfig,
    _toolchain: &toolchain::ToolchainReport,
) -> Result<ValidatePlan, PipelineError> {
    let page_files = manga_page_files("validate", &context)?;
    let mut checks = vec![
        ValidationCheck {
            name: "common-lint",
            target: "common",
            tool: None,
            tool_status: ToolStatus::Planned,
        },
        ValidationCheck {
            name: "image-integrity",
            target: "common",
            tool: None,
            tool_status: ToolStatus::Planned,
        },
    ];

    if resolved.effective.outputs.kindle.is_some() {
        checks.push(ValidationCheck {
            name: "kindle-target-check",
            target: "kindle",
            tool: None,
            tool_status: ToolStatus::Planned,
        });
    }
    if resolved.effective.outputs.print.is_some() {
        checks.push(ValidationCheck {
            name: "print-target-check",
            target: "print",
            tool: None,
            tool_status: ToolStatus::Planned,
        });
    }

    Ok(ValidatePlan {
        context,
        manuscript_files: page_files,
        checks,
    })
}

fn collect_manuscript_files(resolved: &ResolvedBookConfig) -> Vec<PathBuf> {
    resolved
        .manuscript_files()
        .into_iter()
        .map(|path| join_repo_path(&resolved.repo.repo_root, &path))
        .collect()
}

fn manuscript_file_diagnostics(paths: &[PathBuf]) -> Vec<Diagnostic> {
    paths
        .iter()
        .filter(|path| !path.is_file())
        .map(|path| {
            Diagnostic::new(
                "missing-manuscript",
                format!("manuscript file not found: {}", path.display()),
            )
            .at(path.clone())
        })
        .collect()
}

fn manga_page_files(
    command: &'static str,
    context: &RepoContext,
) -> Result<Vec<PathBuf>, PipelineError> {
    let book_root = context
        .book
        .as_ref()
        .expect("book context must exist for manga pipeline")
        .root
        .clone();
    manga::discover_page_files(&book_root).map_err(|error| match error {
        manga::MangaRenderError::MissingPageDirectory { path } => preflight_failed(
            command,
            vec![
                Diagnostic::new(
                    "missing-manga-pages",
                    format!("manga page directory not found: {}", path.display()),
                )
                .at(path),
            ],
        ),
        manga::MangaRenderError::NoPageImages { path } => preflight_failed(
            command,
            vec![
                Diagnostic::new(
                    "missing-manga-pages",
                    format!(
                        "no supported page images were found under {}",
                        path.display()
                    ),
                )
                .at(path),
            ],
        ),
        other => preflight_failed(
            command,
            vec![Diagnostic::new("manga-preflight", other.to_string()).at(book_root)],
        ),
    })
}

fn build_outputs(
    resolved: &ResolvedBookConfig,
    toolchain: &toolchain::ToolchainReport,
) -> Vec<BuildOutputPlan> {
    let mut outputs = Vec::new();

    if let Some(target) = &resolved.effective.outputs.kindle {
        outputs.push(BuildOutputPlan {
            channel: "kindle",
            target: target.clone(),
            artifact_path: artifact_path(resolved, target),
            primary_tool: "pandoc",
            tool_status: toolchain
                .tool("pandoc")
                .map(|tool| tool.status)
                .unwrap_or(ToolStatus::Missing),
        });
    }
    if let Some(target) = &resolved.effective.outputs.print {
        outputs.push(BuildOutputPlan {
            channel: "print",
            target: target.clone(),
            artifact_path: artifact_path(resolved, target),
            primary_tool: "pandoc",
            tool_status: toolchain
                .tool("pandoc")
                .map(|tool| tool.status)
                .unwrap_or(ToolStatus::Missing),
        });
    }

    outputs
}

fn manga_build_outputs(resolved: &ResolvedBookConfig) -> Vec<BuildOutputPlan> {
    let mut outputs = Vec::new();

    if let Some(target) = &resolved.effective.outputs.kindle {
        outputs.push(BuildOutputPlan {
            channel: "kindle",
            target: target.clone(),
            artifact_path: artifact_path(resolved, target),
            primary_tool: "shosei-fxl-epub",
            tool_status: ToolStatus::Available,
        });
    }
    if let Some(target) = &resolved.effective.outputs.print {
        outputs.push(BuildOutputPlan {
            channel: "print",
            target: target.clone(),
            artifact_path: artifact_path(resolved, target),
            primary_tool: "shosei-image-pdf",
            tool_status: ToolStatus::Available,
        });
    }

    outputs
}

fn artifact_path(resolved: &ResolvedBookConfig, target: &str) -> PathBuf {
    let book_id = resolved
        .repo
        .book
        .as_ref()
        .map(|book| book.id.as_str())
        .unwrap_or("default");
    let extension = match target {
        "kindle-ja" | "kindle-comic" => "epub",
        "print-jp-pdfx1a" | "print-jp-pdfx4" | "print-manga" => "pdf",
        _ => "artifact",
    };
    resolved
        .repo
        .repo_root
        .join("dist")
        .join(format!("{book_id}-{target}.{extension}"))
}

fn preflight_failed(command: &'static str, diagnostics: Vec<Diagnostic>) -> PipelineError {
    let summary = diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    PipelineError::PreflightFailed {
        command,
        summary,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{config, repo};

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-pipeline-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prose_validate_plan_includes_target_checks() {
        let root = temp_dir("validate-plan");
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
validation:
  strict: true
  epubcheck: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = config::resolve_book_config(&context).unwrap();
        let plan = prose_validate_plan(context, &resolved).unwrap();

        assert_eq!(plan.checks[0].name, "common-lint");
        assert!(plan.checks.iter().any(|check| check.name == "epubcheck"));
        assert!(
            plan.checks
                .iter()
                .any(|check| check.name == "print-target-check")
        );
    }
}
