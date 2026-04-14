use std::{fs, path::PathBuf};

use crate::{
    cli_api::CommandContext,
    config,
    domain::ProjectType,
    fs::join_repo_path,
    manga, pipeline,
    repo::{self, RepoError},
    toolchain::{self, ToolStatus},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BuildBookResult {
    pub summary: String,
    pub plan: pipeline::BuildPlan,
    pub artifacts: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum BuildBookError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Pipeline(#[from] pipeline::PipelineError),
    #[error("required tool `{tool}` is missing for target `{target}`; run `shosei doctor`")]
    RequiredToolMissing { tool: &'static str, target: String },
    #[error("build target `{target}` is not implemented yet")]
    UnsupportedTarget { target: String },
    #[error("build for `{target}` failed; details saved to {log_path}")]
    ExecutionFailed { target: String, log_path: PathBuf },
    #[error("build planning is not implemented yet for {project_type}")]
    UnsupportedProjectType { project_type: ProjectType },
    #[error("requested target `{target}` is not enabled for this book")]
    TargetNotEnabled { target: String },
}

pub fn build_book(command: &CommandContext) -> Result<BuildBookResult, BuildBookError> {
    let toolchain = toolchain::inspect_default_toolchain();
    build_book_with_toolchain(command, &toolchain)
}

fn build_book_with_toolchain(
    command: &CommandContext,
    toolchain: &toolchain::ToolchainReport,
) -> Result<BuildBookResult, BuildBookError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    if let Some(book) = context.book.clone() {
        let resolved = config::resolve_book_config(&context)?;
        let project_type = resolved.effective.project.project_type;
        let selected_channel = pipeline::selected_output_channel(command);
        let plan = match project_type {
            ProjectType::Manga => pipeline::manga_build_plan_with_toolchain(
                context,
                &resolved,
                toolchain,
                selected_channel,
            )?,
            _ => pipeline::prose_build_plan_with_toolchain(
                context,
                &resolved,
                toolchain,
                selected_channel,
            )?,
        };
        if plan.outputs.is_empty() {
            return Err(BuildBookError::TargetNotEnabled {
                target: command
                    .output_target
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            });
        }
        let outputs = plan
            .outputs
            .iter()
            .map(|output| output.target.clone())
            .collect::<Vec<_>>();
        let source_count = plan.manuscript_files.len();
        let artifacts = match project_type {
            ProjectType::Manga => execute_manga_build_outputs(&resolved, &plan)?,
            _ => execute_build_outputs(&resolved, &plan, toolchain)?,
        };
        return Ok(BuildBookResult {
            summary: format!(
                "build completed for {} with {} input file(s), outputs: {}, stages: {}, artifacts: {}",
                book.id,
                source_count,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                },
                plan.stages.join(", "),
                artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            plan,
            artifacts,
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}

fn execute_build_outputs(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::BuildPlan,
    toolchain: &toolchain::ToolchainReport,
) -> Result<Vec<PathBuf>, BuildBookError> {
    let mut artifacts = Vec::new();
    let book = resolved
        .repo
        .book
        .as_ref()
        .expect("book context must exist for build");

    for output in &plan.outputs {
        match output.channel {
            "kindle" => {
                let pandoc = available_tool_path(toolchain, "pandoc").ok_or_else(|| {
                    BuildBookError::RequiredToolMissing {
                        tool: "pandoc",
                        target: output.target.clone(),
                    }
                })?;
                prepare_artifact_dir(
                    &output.artifact_path,
                    &resolved.repo.repo_root,
                    &book.id,
                    &output.target,
                )?;
                let run_output = toolchain::run_pandoc_epub(
                    pandoc,
                    &plan.manuscript_files,
                    &output.artifact_path,
                    &resolved.effective.book.title,
                    &resolved.effective.book.language,
                    resolved_cover_image_path(resolved).as_deref(),
                )
                .map_err(|error| {
                    execution_failed_with_message(
                        &resolved.repo.repo_root,
                        &book.id,
                        &output.target,
                        format!("failed to start pandoc for {}: {error}", output.target),
                    )
                })?;
                ensure_artifact_written(&resolved.repo.repo_root, &book.id, output, run_output)?;
                artifacts.push(output.artifact_path.clone());
            }
            "print" => {
                let pandoc = available_tool_path(toolchain, "pandoc").ok_or_else(|| {
                    BuildBookError::RequiredToolMissing {
                        tool: "pandoc",
                        target: output.target.clone(),
                    }
                })?;
                prepare_artifact_dir(
                    &output.artifact_path,
                    &resolved.repo.repo_root,
                    &book.id,
                    &output.target,
                )?;
                let run_output = toolchain::run_pandoc_pdf(
                    pandoc,
                    &plan.manuscript_files,
                    &output.artifact_path,
                    &resolved.effective.book.title,
                    &resolved.effective.book.language,
                    resolved
                        .effective
                        .pdf
                        .as_ref()
                        .map(|pdf| pdf.toc)
                        .unwrap_or(true),
                )
                .map_err(|error| {
                    execution_failed_with_message(
                        &resolved.repo.repo_root,
                        &book.id,
                        &output.target,
                        format!("failed to start pandoc for {}: {error}", output.target),
                    )
                })?;
                ensure_artifact_written(&resolved.repo.repo_root, &book.id, output, run_output)?;
                artifacts.push(output.artifact_path.clone());
            }
            _ => {
                return Err(BuildBookError::UnsupportedTarget {
                    target: output.target.clone(),
                });
            }
        }
    }

    Ok(artifacts)
}

fn available_tool_path<'a>(
    toolchain: &'a toolchain::ToolchainReport,
    key: &str,
) -> Option<&'a std::path::Path> {
    toolchain.tool(key).and_then(|tool| match tool.status {
        ToolStatus::Available => tool.resolved_path.as_deref(),
        _ => None,
    })
}

fn resolved_cover_image_path(resolved: &config::ResolvedBookConfig) -> Option<PathBuf> {
    resolved
        .effective
        .cover
        .ebook_image
        .as_ref()
        .map(|path| join_repo_path(&resolved.repo.repo_root, path))
}

fn prepare_artifact_dir(
    artifact_path: &std::path::Path,
    repo_root: &std::path::Path,
    book_id: &str,
    target: &str,
) -> Result<(), BuildBookError> {
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).map_err(|_| BuildBookError::ExecutionFailed {
            target: target.to_string(),
            log_path: build_log_path(repo_root, book_id, target),
        })?;
    }
    Ok(())
}

fn execution_failed_with_message(
    repo_root: &std::path::Path,
    book_id: &str,
    target: &str,
    message: String,
) -> BuildBookError {
    let log_path = build_log_path(repo_root, book_id, target);
    let _ = write_build_log(&log_path, &message);
    BuildBookError::ExecutionFailed {
        target: target.to_string(),
        log_path,
    }
}

fn ensure_artifact_written(
    repo_root: &std::path::Path,
    book_id: &str,
    output: &pipeline::BuildOutputPlan,
    run_output: toolchain::ToolRunOutput,
) -> Result<(), BuildBookError> {
    if run_output.status.success() && output.artifact_path.is_file() {
        return Ok(());
    }

    let log_path = build_log_path(repo_root, book_id, &output.target);
    let log_contents = format!(
        "tool: pandoc\nstatus: {}\nstdout:\n{}\n\nstderr:\n{}\n",
        run_output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "signal".to_string()),
        run_output.stdout,
        run_output.stderr
    );
    let _ = write_build_log(&log_path, &log_contents);
    Err(BuildBookError::ExecutionFailed {
        target: output.target.clone(),
        log_path,
    })
}

fn build_log_path(repo_root: &std::path::Path, book_id: &str, target: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("logs")
        .join(format!("{book_id}-{target}-build.log"))
}

fn write_build_log(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn execute_manga_build_outputs(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::BuildPlan,
) -> Result<Vec<PathBuf>, BuildBookError> {
    let mut artifacts = Vec::new();
    let book = resolved
        .repo
        .book
        .as_ref()
        .expect("book context must exist for build");
    let manga_settings = resolved
        .effective
        .manga
        .as_ref()
        .expect("manga settings must exist for manga build");

    for output in &plan.outputs {
        prepare_artifact_dir(
            &output.artifact_path,
            &resolved.repo.repo_root,
            &book.id,
            &output.target,
        )?;

        let result = match output.channel {
            "kindle" => manga::write_fixed_layout_epub(
                &book.id,
                &resolved.effective.book.title,
                &resolved.effective.book.language,
                &plan.manuscript_files,
                &output.artifact_path,
                resolved_cover_image_path(resolved).as_deref(),
                manga::FixedLayoutOptions {
                    reading_direction: manga_settings.reading_direction,
                    default_page_side: manga_settings.default_page_side,
                    spread_policy_for_kindle: manga_settings.spread_policy_for_kindle,
                },
            ),
            "print" => manga::write_image_pdf(
                &resolved.effective.book.title,
                &plan.manuscript_files,
                &output.artifact_path,
            ),
            _ => {
                return Err(BuildBookError::UnsupportedTarget {
                    target: output.target.clone(),
                });
            }
        };

        result.map_err(|error| {
            execution_failed_with_message(
                &resolved.repo.repo_root,
                &book.id,
                &output.target,
                error.to_string(),
            )
        })?;

        if !output.artifact_path.is_file() {
            return Err(execution_failed_with_message(
                &resolved.repo.repo_root,
                &book.id,
                &output.target,
                format!(
                    "manga build did not create an artifact for {}",
                    output.target
                ),
            ));
        }

        artifacts.push(output.artifact_path.clone());
    }

    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read, path::PathBuf};

    use crate::toolchain::{ToolRecord, ToolchainReport};

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-build-book-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_toolchain(pandoc_path: Option<PathBuf>) -> ToolchainReport {
        ToolchainReport {
            tools: vec![ToolRecord {
                key: "pandoc",
                display_name: "pandoc",
                status: if pandoc_path.is_some() {
                    ToolStatus::Available
                } else {
                    ToolStatus::Missing
                },
                detected_as: Some("pandoc".to_string()),
                resolved_path: pandoc_path,
                version: None,
                install_hint: "Install pandoc and ensure it is available on PATH.".to_string(),
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
git:
  lfs: true
"#,
        )
        .unwrap();
    }

    fn write_book_with_cover(root: &std::path::Path) {
        write_book(root);
        fs::create_dir_all(root.join("assets/cover")).unwrap();
        fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
        let book_yml = fs::read_to_string(root.join("book.yml")).unwrap();
        fs::write(
            root.join("book.yml"),
            format!("{book_yml}cover:\n  ebook_image: assets/cover/front.png\n"),
        )
        .unwrap();
    }

    fn write_print_book(root: &std::path::Path) {
        write_print_book_with_pdf(root, "");
    }

    fn write_print_book_with_pdf(root: &std::path::Path, pdf_block: &str) {
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
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx1a
{pdf_block}validation:
  strict: true
git:
  lfs: true
"#
            ),
        )
        .unwrap();
    }

    fn write_print_book_without_toc(root: &std::path::Path) {
        write_print_book_with_pdf(
            root,
            "pdf:\n  engine: weasyprint\n  toc: false\n  page_number: true\n  running_header: auto\n",
        );
    }

    fn write_print_book_with_toc(root: &std::path::Path) {
        write_print_book_with_pdf(
            root,
            "pdf:\n  engine: weasyprint\n  toc: true\n  page_number: true\n  running_header: auto\n",
        );
    }

    fn write_manga_book(root: &std::path::Path, output_block: &str, spread_policy: &str) {
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), tiny_png()).unwrap();
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
{output_block}validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: {spread_policy}
  front_color_pages: 0
  body_mode: monochrome
"#
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

    fn read_epub_entry(epub_path: &std::path::Path, entry_name: &str) -> String {
        let file = fs::File::open(epub_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut contents = String::new();
        archive
            .by_name(entry_name)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        contents
    }

    #[test]
    fn build_reports_missing_pandoc() {
        let root = temp_dir("missing-pandoc");
        write_book(&root);

        let error = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            BuildBookError::RequiredToolMissing { tool, target }
                if tool == "pandoc" && target == "kindle-ja"
        ));
    }

    #[test]
    fn build_executes_fake_pandoc_and_writes_artifact() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-success");
        write_book(&root);
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert!(result.artifacts[0].is_file());
    }

    #[test]
    fn build_passes_cover_image_to_pandoc_epub() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-cover");
        write_book_with_cover(&root);
        let pandoc = root.join("pandoc");
        let cover_arg = root.join("cover-arg.txt");
        fs::write(
            &pandoc,
            format!(
                r#"#!/bin/sh
out=""
cover=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  if [ "$prev" = "--epub-cover-image" ]; then
    cover="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf '%s' "$cover" > "{}"
printf 'fake epub' > "$out"
"#,
                cover_arg.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        assert_eq!(
            fs::read_to_string(cover_arg).unwrap(),
            root.join("assets/cover/front.png").display().to_string()
        );
    }

    #[test]
    fn build_writes_log_when_pandoc_fails() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-failure");
        write_book(&root);
        let pandoc = root.join("pandoc");
        fs::write(
            &pandoc,
            r#"#!/bin/sh
echo "pandoc failed" >&2
exit 42
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let error = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap_err();

        match error {
            BuildBookError::ExecutionFailed { log_path, .. } => {
                assert!(log_path.is_file());
                let log = fs::read_to_string(log_path).unwrap();
                assert!(log.contains("pandoc failed"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn build_executes_fake_pandoc_and_writes_print_artifact() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-print-success");
        write_print_book(&root);
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
printf 'fake pdf' > "$out"
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert!(result.artifacts[0].is_file());
        assert_eq!(
            result.artifacts[0].extension().and_then(|ext| ext.to_str()),
            Some("pdf")
        );
    }

    #[test]
    fn build_passes_toc_flag_to_pandoc_pdf_when_enabled() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-print-toc-enabled");
        write_print_book_with_toc(&root);
        let pandoc = root.join("pandoc");
        let args_path = root.join("pandoc-args.txt");
        fs::write(
            &pandoc,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{}"
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf 'fake pdf' > "$out"
"#,
                args_path.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        let args = fs::read_to_string(args_path).unwrap();
        assert!(args.lines().any(|arg| arg == "--toc"));
    }

    #[test]
    fn build_omits_toc_flag_to_pandoc_pdf_when_disabled() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-print-toc-disabled");
        write_print_book_without_toc(&root);
        let pandoc = root.join("pandoc");
        let args_path = root.join("pandoc-args.txt");
        fs::write(
            &pandoc,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{}"
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf 'fake pdf' > "$out"
"#,
                args_path.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        let args = fs::read_to_string(args_path).unwrap();
        assert!(!args.lines().any(|arg| arg == "--toc"));
    }

    #[test]
    fn build_writes_manga_epub_artifact() {
        let root = temp_dir("manga-kindle");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            "split",
        );
        fs::write(root.join("manga/pages/002.png"), tiny_png()).unwrap();

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(
            result.artifacts[0].extension().and_then(|ext| ext.to_str()),
            Some("epub")
        );
        assert!(result.artifacts[0].is_file());

        let package = read_epub_entry(&result.artifacts[0], "OEBPS/package.opf");
        assert!(package.contains("page-progression-direction=\"rtl\""));
        assert!(package.contains("page-spread-right"));
        assert!(package.contains("page-spread-left"));
    }

    #[test]
    fn build_includes_cover_image_in_manga_epub() {
        let root = temp_dir("manga-kindle-cover");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            "split",
        );
        fs::create_dir_all(root.join("assets/cover")).unwrap();
        fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
        let book_yml = fs::read_to_string(root.join("book.yml")).unwrap();
        fs::write(
            root.join("book.yml"),
            format!("{book_yml}cover:\n  ebook_image: assets/cover/front.png\n"),
        )
        .unwrap();

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        let package = read_epub_entry(&result.artifacts[0], "OEBPS/package.opf");
        assert!(package.contains("properties=\"cover-image\""));
        let file = fs::File::open(&result.artifacts[0]).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("OEBPS/cover/front.png").is_ok());
    }

    #[test]
    fn build_splits_wide_manga_page_for_kindle_rtl() {
        let root = temp_dir("manga-kindle-split");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            "split",
        );
        fs::write(root.join("manga/pages/001.png"), wide_png()).unwrap();

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        let first_page = read_epub_entry(&result.artifacts[0], "OEBPS/pages/page-0001.xhtml");
        let second_page = read_epub_entry(&result.artifacts[0], "OEBPS/pages/page-0002.xhtml");
        assert!(first_page.contains("001-right.png"));
        assert!(second_page.contains("001-left.png"));
    }

    #[test]
    fn build_keeps_wide_manga_page_as_single_page_for_kindle() {
        let root = temp_dir("manga-kindle-single-page");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            "single-page",
        );
        fs::write(root.join("manga/pages/001.png"), wide_png()).unwrap();

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        let first_page = read_epub_entry(&result.artifacts[0], "OEBPS/pages/page-0001.xhtml");
        assert!(first_page.contains("001.png"));
    }

    #[test]
    fn build_skips_wide_manga_page_for_kindle() {
        let root = temp_dir("manga-kindle-skip");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            "skip",
        );
        fs::write(root.join("manga/pages/002.png"), wide_png()).unwrap();

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        let package = read_epub_entry(&result.artifacts[0], "OEBPS/package.opf");
        let first_page = read_epub_entry(&result.artifacts[0], "OEBPS/pages/page-0001.xhtml");
        assert!(package.contains("page-0001"));
        assert!(!package.contains("page-0002"));
        assert!(first_page.contains("001.png"));
    }

    #[test]
    fn build_writes_manga_pdf_artifact() {
        let root = temp_dir("manga-print");
        write_manga_book(
            &root,
            "outputs:\n  print:\n    enabled: true\n    target: print-manga\n",
            "split",
        );

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain(None),
        )
        .unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(
            result.artifacts[0].extension().and_then(|ext| ext.to_str()),
            Some("pdf")
        );
        assert!(result.artifacts[0].is_file());
    }
}
