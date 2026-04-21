use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cli_api::CommandContext,
    config::{self},
    domain::ProjectType,
    fs::join_repo_path,
    manga, pipeline,
    repo::{self, RepoError},
    toolchain::{self, ToolStatus},
};
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BuildBookResult {
    pub summary: String,
    pub plan: pipeline::BuildPlan,
    pub artifacts: Vec<PathBuf>,
    artifact_details: Vec<Value>,
}

impl BuildBookResult {
    pub fn artifact_details(&self) -> &[Value] {
        &self.artifact_details
    }

    pub fn artifact_metadata(&self, channel: &str, target: &str) -> Option<&Value> {
        self.artifact_details.iter().find_map(|detail| {
            let object = detail.as_object()?;
            let detail_channel = object.get("channel")?.as_str()?;
            let detail_target = object.get("target")?.as_str()?;
            (detail_channel == channel && detail_target == target)
                .then(|| object.get("artifact_metadata"))
                .flatten()
        })
    }
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
    #[error("failed to write generated print stylesheet to {path}: {source}")]
    WriteGeneratedStylesheet {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "pdf.engine `weasyprint` does not support vertical-rl prose print for target `{target}`; set `pdf.engine: chromium`"
    )]
    UnsupportedVerticalWeasyprint { target: String },
}

pub fn build_book(command: &CommandContext) -> Result<BuildBookResult, BuildBookError> {
    let toolchain = toolchain::inspect_default_toolchain();
    build_book_with_toolchain(command, &toolchain)
}

pub(crate) fn build_book_with_toolchain(
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
        let tool_summary = plan
            .outputs
            .iter()
            .map(|output| format!("{} via {}", output.target, output.primary_tool))
            .collect::<Vec<_>>();
        let source_count = plan.manuscript_files.len();
        let artifacts = match project_type {
            ProjectType::Manga => execute_manga_build_outputs(&resolved, &plan)?,
            _ => execute_build_outputs(&resolved, &plan, toolchain)?,
        };
        let artifact_details = build_artifact_details(&resolved, &plan, &artifacts)?;
        return Ok(BuildBookResult {
            summary: format!(
                "build completed for {} with {} input file(s), outputs: {}, tools: {}, stages: {}, artifacts: {}",
                book.id,
                source_count,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                },
                if tool_summary.is_empty() {
                    "none".to_string()
                } else {
                    tool_summary.join(", ")
                },
                plan.stages.join(", "),
                artifacts
                    .iter()
                    .map(|path| relative_display(&resolved.repo.repo_root, path))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            plan,
            artifacts,
            artifact_details,
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}

fn build_artifact_details(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::BuildPlan,
    artifacts: &[PathBuf],
) -> Result<Vec<Value>, BuildBookError> {
    let source_file_count = plan.manuscript_files.len();
    match resolved.effective.project.project_type {
        ProjectType::Manga => {
            build_manga_artifact_details(resolved, plan, artifacts, source_file_count)
        }
        _ => Ok(build_prose_artifact_details(
            resolved,
            plan,
            artifacts,
            source_file_count,
        )),
    }
}

fn build_prose_artifact_details(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::BuildPlan,
    artifacts: &[PathBuf],
    source_file_count: usize,
) -> Vec<Value> {
    plan.outputs
        .iter()
        .zip(artifacts.iter())
        .map(|(output, artifact_path)| {
            let metadata = match output.channel {
                "kindle" => prose_kindle_metadata(resolved, source_file_count),
                "print" => prose_print_metadata(resolved, source_file_count, artifact_path),
                _ => json!({}),
            };
            json!({
                "channel": output.channel,
                "target": output.target,
                "path": relative_display(&resolved.repo.repo_root, &output.artifact_path),
                "primary_tool": output.primary_tool,
                "target_profile": resolved.effective.book.profile,
                "artifact_metadata": metadata,
            })
        })
        .collect()
}

fn build_manga_artifact_details(
    resolved: &config::ResolvedBookConfig,
    plan: &pipeline::BuildPlan,
    artifacts: &[PathBuf],
    source_file_count: usize,
) -> Result<Vec<Value>, BuildBookError> {
    let manga_settings = resolved
        .effective
        .manga
        .as_ref()
        .expect("manga settings must exist for manga build");

    plan.outputs
        .iter()
        .zip(artifacts.iter())
        .map(|(output, artifact_path)| {
            let render_summary = match output.channel {
                "kindle" => manga::summarize_fixed_layout_render(
                    &plan.manuscript_files,
                    manga::FixedLayoutOptions {
                        reading_direction: manga_settings.reading_direction,
                        default_page_side: manga_settings.default_page_side,
                        spread_policy_for_kindle: manga_settings.spread_policy_for_kindle,
                    },
                ),
                "print" => manga::summarize_print_render(&plan.manuscript_files),
                _ => Ok(manga::MangaRenderSummary {
                    source_page_count: 0,
                    rendered_page_count: 0,
                    spread_candidate_count: 0,
                    split_source_page_count: 0,
                    skipped_source_page_count: 0,
                    color_page_count: 0,
                    unique_page_dimensions: Vec::new(),
                }),
            }
            .map_err(|error| {
                execution_failed_with_message(
                    &resolved.repo.repo_root,
                    &resolved
                        .repo
                        .book
                        .as_ref()
                        .expect("book context must exist for build")
                        .id,
                    &output.target,
                    error.to_string(),
                )
            })?;

            let metadata = match output.channel {
                "kindle" => manga_kindle_metadata(
                    resolved,
                    source_file_count,
                    manga_settings,
                    &render_summary,
                ),
                "print" => manga_print_metadata(
                    resolved,
                    source_file_count,
                    manga_settings,
                    &render_summary,
                    artifact_path,
                ),
                _ => json!({}),
            };

            Ok(json!({
                "channel": output.channel,
                "target": output.target,
                "path": relative_display(&resolved.repo.repo_root, &output.artifact_path),
                "primary_tool": output.primary_tool,
                "target_profile": resolved.effective.book.profile,
                "artifact_metadata": metadata,
            }))
        })
        .collect()
}

fn prose_kindle_metadata(resolved: &config::ResolvedBookConfig, source_file_count: usize) -> Value {
    json!({
        "format": "epub",
        "book_profile": resolved.effective.book.profile,
        "source_file_count": source_file_count,
        "kindle": {
            "fixed_layout": false,
            "reading_direction": resolved.effective.book.reading_direction.as_str(),
            "cover_ebook_image": resolved.effective.cover.ebook_image.as_ref().map(|path| path.as_str()),
        }
    })
}

fn prose_print_metadata(
    resolved: &config::ResolvedBookConfig,
    source_file_count: usize,
    artifact_path: &Path,
) -> Value {
    let pdf = resolved
        .effective
        .pdf
        .as_ref()
        .expect("pdf settings must exist for prose print build");
    let print = resolved.effective.print.as_ref();
    let pdf_inspection = inspect_pdf_artifact(artifact_path);

    json!({
        "format": "pdf",
        "source_file_count": source_file_count,
        "print": {
            "pdf_engine": pdf.engine.as_str(),
            "toc": pdf.toc,
            "page_numbering": pdf.page_number,
            "running_header": pdf.running_header.as_str(),
            "column_count": pdf.column_count,
            "column_gap": pdf.column_gap,
            "base_font_size": pdf.base_font_size,
            "line_height": pdf.line_height,
            "trim_size": print.map(|settings| settings.trim_size.as_str()),
            "bleed": print.map(|settings| settings.bleed.as_str()),
            "crop_marks": print.map(|settings| settings.crop_marks),
            "page_margin": print.and_then(|settings| settings.page_margin.as_ref()).map(|margin| json!({
                "top": margin.top,
                "right": margin.right,
                "bottom": margin.bottom,
                "left": margin.left,
            })),
            "sides": print.map(|settings| settings.sides.as_str()),
            "max_pages": print.and_then(|settings| settings.max_pages),
            "body_pdf": print.map(|settings| settings.body_pdf),
            "cover_pdf": print.map(|settings| settings.cover_pdf),
            "pdf_standard": print.map(|settings| settings.pdf_standard.as_str()),
            "body_mode": print.map(|settings| settings.body_mode.as_str()),
            "page_count": pdf_inspection.as_ref().map(|inspection| inspection.page_count),
            "fonts_embedded": pdf_inspection.as_ref().map(|inspection| inspection.fonts_embedded),
        }
    })
}

fn manga_kindle_metadata(
    resolved: &config::ResolvedBookConfig,
    source_file_count: usize,
    manga_settings: &config::MangaSettings,
    render_summary: &manga::MangaRenderSummary,
) -> Value {
    json!({
        "format": "epub",
        "book_profile": resolved.effective.book.profile,
        "source_file_count": source_file_count,
        "kindle": {
            "fixed_layout": true,
            "reading_direction": manga_settings.reading_direction.as_str(),
            "cover_ebook_image": resolved.effective.cover.ebook_image.as_ref().map(|path| path.as_str()),
        },
        "manga": manga_delivery_metadata(manga_settings, render_summary),
    })
}

fn manga_print_metadata(
    resolved: &config::ResolvedBookConfig,
    source_file_count: usize,
    manga_settings: &config::MangaSettings,
    render_summary: &manga::MangaRenderSummary,
    artifact_path: &Path,
) -> Value {
    let print = resolved.effective.print.as_ref();
    let pdf_inspection = inspect_pdf_artifact(artifact_path);
    json!({
        "format": "pdf",
        "source_file_count": source_file_count,
        "print": {
            "trim_size": print.map(|settings| settings.trim_size.as_str()),
            "bleed": print.map(|settings| settings.bleed.as_str()),
            "crop_marks": print.map(|settings| settings.crop_marks),
            "sides": print.map(|settings| settings.sides.as_str()),
            "max_pages": print.and_then(|settings| settings.max_pages),
            "body_pdf": print.map(|settings| settings.body_pdf),
            "cover_pdf": print.map(|settings| settings.cover_pdf),
            "pdf_standard": print.map(|settings| settings.pdf_standard.as_str()),
            "body_mode": print.map(|settings| settings.body_mode.as_str()),
            "page_count": pdf_inspection
                .as_ref()
                .map(|inspection| inspection.page_count)
                .or(Some(render_summary.rendered_page_count)),
            "fonts_embedded": pdf_inspection.as_ref().map(|inspection| inspection.fonts_embedded),
        },
        "manga": manga_delivery_metadata(manga_settings, render_summary),
    })
}

struct PdfArtifactInspection {
    page_count: usize,
    fonts_embedded: bool,
}

fn inspect_pdf_artifact(path: &Path) -> Option<PdfArtifactInspection> {
    let bytes = fs::read(path).ok()?;
    let page_count = pdf_page_object_count(&bytes);
    let fonts_embedded = bytes
        .windows(b"/FontFile".len())
        .any(|window| window == b"/FontFile")
        || bytes
            .windows(b"/FontFile2".len())
            .any(|window| window == b"/FontFile2")
        || bytes
            .windows(b"/FontFile3".len())
            .any(|window| window == b"/FontFile3");

    if page_count == 0 && !fonts_embedded {
        return None;
    }

    Some(PdfArtifactInspection {
        page_count,
        fonts_embedded,
    })
}

fn pdf_page_object_count(bytes: &[u8]) -> usize {
    const PAGE_TYPE: &[u8] = b"/Type /Page";

    bytes
        .windows(PAGE_TYPE.len())
        .enumerate()
        .filter(|(index, window)| {
            *window == PAGE_TYPE
                && bytes
                    .get(index + PAGE_TYPE.len())
                    .is_none_or(|byte| pdf_name_delimiter(*byte))
        })
        .count()
}

fn pdf_name_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'\0'
            | b'\t'
            | b'\n'
            | b'\x0c'
            | b'\r'
            | b' '
            | b'('
            | b')'
            | b'<'
            | b'>'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'/'
            | b'%'
    )
}

fn manga_delivery_metadata(
    manga_settings: &config::MangaSettings,
    render_summary: &manga::MangaRenderSummary,
) -> Value {
    json!({
        "reading_direction": manga_settings.reading_direction.as_str(),
        "default_page_side": manga_page_side_label(manga_settings.default_page_side),
        "spread_policy_for_kindle": spread_policy_label(manga_settings.spread_policy_for_kindle),
        "front_color_pages": manga_settings.front_color_pages,
        "body_mode": manga_body_mode_label(manga_settings.body_mode),
        "source_page_count": render_summary.source_page_count,
        "rendered_page_count": render_summary.rendered_page_count,
        "spread_candidate_count": render_summary.spread_candidate_count,
        "split_source_page_count": render_summary.split_source_page_count,
        "skipped_source_page_count": render_summary.skipped_source_page_count,
        "color_page_count": render_summary.color_page_count,
        "unique_page_dimensions": render_summary.unique_page_dimensions.iter().map(|page| {
            json!({
                "width_px": page.width_px,
                "height_px": page.height_px,
            })
        }).collect::<Vec<_>>(),
    })
}

fn manga_page_side_label(side: config::MangaPageSide) -> &'static str {
    match side {
        config::MangaPageSide::Left => "left",
        config::MangaPageSide::Right => "right",
    }
}

fn spread_policy_label(policy: config::SpreadPolicyForKindle) -> &'static str {
    match policy {
        config::SpreadPolicyForKindle::Split => "split",
        config::SpreadPolicyForKindle::SinglePage => "single-page",
        config::SpreadPolicyForKindle::Skip => "skip",
    }
}

fn manga_body_mode_label(mode: config::MangaBodyMode) -> &'static str {
    match mode {
        config::MangaBodyMode::Monochrome => "monochrome",
        config::MangaBodyMode::Color => "color",
        config::MangaBodyMode::Mixed => "mixed",
    }
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
                let epub_stylesheets = epub_stylesheets(resolved);
                let run_output = toolchain::run_pandoc_epub(
                    pandoc,
                    &plan.manuscript_files,
                    &toolchain::PandocEpubOptions {
                        working_dir: &book.root,
                        output: &output.artifact_path,
                        title: &resolved.effective.book.title,
                        language: &resolved.effective.book.language,
                        stylesheets: &epub_stylesheets,
                        cover_image: resolved_cover_image_path(resolved).as_deref(),
                    },
                )
                .map_err(|error| {
                    execution_failed_with_message(
                        &resolved.repo.repo_root,
                        &book.id,
                        &output.target,
                        format!("failed to start pandoc for {}: {error}", output.target),
                    )
                })?;
                ensure_path_written(
                    &resolved.repo.repo_root,
                    &book.id,
                    &output.target,
                    "pandoc",
                    &output.artifact_path,
                    run_output,
                )?;
                artifacts.push(output.artifact_path.clone());
            }
            "print" => {
                let pandoc = available_tool_path(toolchain, "pandoc").ok_or_else(|| {
                    BuildBookError::RequiredToolMissing {
                        tool: "pandoc",
                        target: output.target.clone(),
                    }
                })?;
                let pdf = resolved
                    .effective
                    .pdf
                    .as_ref()
                    .expect("pdf settings must exist for prose print build");
                prepare_artifact_dir(
                    &output.artifact_path,
                    &resolved.repo.repo_root,
                    &book.id,
                    &output.target,
                )?;
                match pdf.engine {
                    config::PdfEngine::Chromium => {
                        let chromium =
                            available_tool_path(toolchain, "chromium").ok_or_else(|| {
                                BuildBookError::RequiredToolMissing {
                                    tool: "chromium",
                                    target: output.target.clone(),
                                }
                            })?;
                        let stylesheets = generated_print_stylesheets(resolved, output)?;
                        let html_path = generated_print_html_path(&output.artifact_path);
                        let pandoc_output = toolchain::run_pandoc_html(
                            pandoc,
                            &plan.manuscript_files,
                            &toolchain::PandocHtmlOptions {
                                working_dir: &book.root,
                                output: &html_path,
                                title: &resolved.effective.book.title,
                                language: &resolved.effective.book.language,
                                stylesheets: &stylesheets,
                                table_of_contents: pdf.toc,
                            },
                        )
                        .map_err(|error| {
                            execution_failed_with_message(
                                &resolved.repo.repo_root,
                                &book.id,
                                &output.target,
                                format!("failed to start pandoc for {}: {error}", output.target),
                            )
                        })?;
                        ensure_path_written(
                            &resolved.repo.repo_root,
                            &book.id,
                            &output.target,
                            "pandoc",
                            &html_path,
                            pandoc_output,
                        )?;

                        let chromium_output = toolchain::run_chromium_pdf(
                            chromium,
                            &html_path,
                            &output.artifact_path,
                        )
                        .map_err(|error| {
                            execution_failed_with_message(
                                &resolved.repo.repo_root,
                                &book.id,
                                &output.target,
                                format!("failed to start chromium for {}: {error}", output.target),
                            )
                        })?;
                        ensure_path_written(
                            &resolved.repo.repo_root,
                            &book.id,
                            &output.target,
                            "chromium",
                            &output.artifact_path,
                            chromium_output,
                        )?;
                    }
                    config::PdfEngine::Weasyprint
                        if resolved.effective.book.writing_mode
                            == config::WritingMode::VerticalRl =>
                    {
                        return Err(BuildBookError::UnsupportedVerticalWeasyprint {
                            target: output.target.clone(),
                        });
                    }
                    _ => {
                        let pdf_options = build_pandoc_pdf_options(resolved, output)?;
                        let run_output = toolchain::run_pandoc_pdf(
                            pandoc,
                            &book.root,
                            &plan.manuscript_files,
                            &output.artifact_path,
                            &resolved.effective.book.title,
                            &resolved.effective.book.language,
                            &pdf_options,
                        )
                        .map_err(|error| {
                            execution_failed_with_message(
                                &resolved.repo.repo_root,
                                &book.id,
                                &output.target,
                                format!("failed to start pandoc for {}: {error}", output.target),
                            )
                        })?;
                        ensure_path_written(
                            &resolved.repo.repo_root,
                            &book.id,
                            &output.target,
                            "pandoc",
                            &output.artifact_path,
                            run_output,
                        )?;
                    }
                }
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

fn build_pandoc_pdf_options(
    resolved: &config::ResolvedBookConfig,
    output: &pipeline::BuildOutputPlan,
) -> Result<toolchain::PandocPdfOptions, BuildBookError> {
    let pdf = resolved
        .effective
        .pdf
        .as_ref()
        .expect("pdf settings must exist for prose print build");
    let print = resolved.effective.print.as_ref();
    let mut options = toolchain::PandocPdfOptions {
        pdf_engine: pdf.engine,
        table_of_contents: pdf.toc,
        stylesheets: Vec::new(),
        variables: Vec::new(),
        variable_json: Vec::new(),
    };

    match pdf.engine {
        config::PdfEngine::Weasyprint => {
            options.stylesheets = generated_print_stylesheets(resolved, output)?;
        }
        config::PdfEngine::Chromium => {}
        config::PdfEngine::Typst => {
            apply_typst_print_variables(&mut options, pdf, print);
        }
        config::PdfEngine::Lualatex => {
            apply_lualatex_print_variables(&mut options, pdf, print);
        }
    }

    Ok(options)
}

fn epub_stylesheets(resolved: &config::ResolvedBookConfig) -> Vec<PathBuf> {
    prose_stylesheets(resolved, &["base.css", "epub.css"])
}

fn print_stylesheets(resolved: &config::ResolvedBookConfig) -> Vec<PathBuf> {
    prose_stylesheets(resolved, &["base.css", "print.css"])
}

fn prose_stylesheets(resolved: &config::ResolvedBookConfig, file_names: &[&str]) -> Vec<PathBuf> {
    let book = resolved
        .repo
        .book
        .as_ref()
        .expect("book context must exist for build");
    let mut stylesheets = Vec::new();

    push_stylesheet_candidates(&mut stylesheets, &book.root.join("styles"), file_names);
    for path in &resolved.shared.styles {
        push_stylesheet_candidates(
            &mut stylesheets,
            &join_repo_path(&resolved.repo.repo_root, path),
            file_names,
        );
    }

    stylesheets
}

fn push_stylesheet_candidates(stylesheets: &mut Vec<PathBuf>, path: &Path, file_names: &[&str]) {
    if path.is_dir() {
        for name in file_names {
            let candidate = path.join(name);
            if candidate.is_file() && !stylesheets.iter().any(|existing| existing == &candidate) {
                stylesheets.push(candidate);
            }
        }
    } else if path.is_file()
        && path.extension().and_then(|ext| ext.to_str()) == Some("css")
        && !stylesheets.iter().any(|existing| existing == path)
    {
        stylesheets.push(path.to_path_buf());
    }
}

fn generated_print_stylesheet_path(output: &Path) -> PathBuf {
    output.with_extension("layout.css")
}

fn relative_display(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|relative| relative.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string().replace('\\', "/"))
}

fn generated_print_html_path(output: &Path) -> PathBuf {
    output.with_extension("print.html")
}

fn generated_print_stylesheets(
    resolved: &config::ResolvedBookConfig,
    output: &pipeline::BuildOutputPlan,
) -> Result<Vec<PathBuf>, BuildBookError> {
    let pdf = resolved
        .effective
        .pdf
        .as_ref()
        .expect("pdf settings must exist for prose print build");
    let print = resolved.effective.print.as_ref();
    let mut stylesheets = print_stylesheets(resolved);
    let generated = generated_print_stylesheet_path(&output.artifact_path);
    fs::write(
        &generated,
        render_generated_print_stylesheet(
            &resolved.effective.book.title,
            &resolved.effective.book.profile,
            resolved.effective.book.writing_mode,
            pdf,
            print,
        ),
    )
    .map_err(|source| BuildBookError::WriteGeneratedStylesheet {
        path: generated.clone(),
        source,
    })?;
    stylesheets.push(generated);
    Ok(stylesheets)
}

fn render_generated_print_stylesheet(
    title: &str,
    profile: &str,
    writing_mode: config::WritingMode,
    pdf: &config::PdfSettings,
    print: Option<&config::PrintSettings>,
) -> String {
    let mut css = Vec::new();
    css.push("html {".to_string());
    if pdf.base_font_size != "auto" {
        css.push(format!("  font-size: {};", pdf.base_font_size));
    }
    if pdf.line_height != "auto" {
        css.push(format!("  line-height: {};", pdf.line_height));
    }
    css.push("}".to_string());
    css.push("body {".to_string());
    css.push("  margin: 0;".to_string());
    if pdf.column_count > 1 {
        css.push(format!("  column-count: {};", pdf.column_count));
        if pdf.column_gap != "auto" {
            css.push(format!("  column-gap: {};", pdf.column_gap));
        }
        css.push("  column-fill: balance;".to_string());
    }
    css.push("}".to_string());
    css.push("h1, h2, h3 { break-after: avoid; }".to_string());
    css.push("figure, table, pre, blockquote { break-inside: avoid; }".to_string());
    css.push("h1 { string-set: shosei-heading content(text); }".to_string());
    if writing_mode == config::WritingMode::VerticalRl {
        if pdf.toc {
            css.push(
                "header#title-block-header { break-after: avoid; page-break-after: avoid; }"
                    .to_string(),
            );
            css.push("nav#TOC { break-after: page; page-break-after: always; }".to_string());
        } else {
            css.push(
                "header#title-block-header { break-after: page; page-break-after: always; }"
                    .to_string(),
            );
        }
    }

    let mut page_lines = Vec::new();
    if let Some(print) = print {
        if let Some(size) = css_page_size(print.trim_size) {
            page_lines.push(format!("  size: {};", size));
        }
        if let Some(margin) = print.page_margin.as_ref() {
            page_lines.push(format!(
                "  margin: {} {} {} {};",
                margin.top, margin.right, margin.bottom, margin.left
            ));
        }
        page_lines.push(format!("  bleed: {};", print.bleed));
        if print.crop_marks {
            page_lines.push("  marks: crop;".to_string());
        }
    }
    push_page_rule(&mut css, None, page_lines);

    let running_header = match pdf.running_header {
        config::PdfRunningHeader::None => None,
        config::PdfRunningHeader::Title => Some(format!("\"{}\"", escape_css_string(title))),
        config::PdfRunningHeader::Chapter | config::PdfRunningHeader::Auto => {
            Some("string(shosei-heading)".to_string())
        }
    };
    let has_page_style_content = pdf.page_number || running_header.is_some();
    if has_page_style_content {
        let mut left_page_lines = Vec::new();
        let mut right_page_lines = Vec::new();
        if writing_mode == config::WritingMode::VerticalRl {
            // Chromium clips right-side corner margin boxes under vertical-rl, so keep
            // vertical prose page styles in center margin boxes for stable output.
            if pdf.page_number {
                left_page_lines.push("  @bottom-center { content: counter(page); }".to_string());
                right_page_lines.push("  @bottom-center { content: counter(page); }".to_string());
            }
            if let Some(content) = &running_header {
                left_page_lines.push(format!("  @top-center {{ content: {content}; }}"));
                right_page_lines.push(format!("  @top-center {{ content: {content}; }}"));
            }
        } else {
            if pdf.page_number {
                left_page_lines.push("  @bottom-left { content: counter(page); }".to_string());
                right_page_lines.push("  @bottom-right { content: counter(page); }".to_string());
            }
            if let Some(content) = &running_header {
                left_page_lines.push(format!("  @top-left {{ content: {content}; }}"));
                right_page_lines.push(format!("  @top-right {{ content: {content}; }}"));
            }
        }
        push_page_rule(&mut css, Some(":left"), left_page_lines);
        push_page_rule(&mut css, Some(":right"), right_page_lines);

        if writing_mode == config::WritingMode::VerticalRl {
            if pdf.toc {
                css.push(
                    "header#title-block-header, nav#TOC { page: shosei-frontmatter; }".to_string(),
                );
            } else {
                css.push("header#title-block-header { page: shosei-frontmatter; }".to_string());
            }
            let suppressed_page_style = suppressed_page_style_lines();
            push_page_rule(
                &mut css,
                Some("shosei-frontmatter"),
                suppressed_page_style.clone(),
            );
            push_page_rule(
                &mut css,
                Some("shosei-frontmatter:first"),
                suppressed_page_style.clone(),
            );
            push_page_rule(
                &mut css,
                Some("shosei-frontmatter:left"),
                suppressed_page_style.clone(),
            );
            push_page_rule(
                &mut css,
                Some("shosei-frontmatter:right"),
                suppressed_page_style,
            );
        }
    }

    if profile == "conference-preprint" && pdf.column_count > 1 {
        css.push(".abstract, .title, .subtitle { column-span: all; }".to_string());
    }

    css.join("\n") + "\n"
}

fn css_page_size(trim_size: config::PrintTrimSize) -> Option<&'static str> {
    match trim_size {
        config::PrintTrimSize::A4 => Some("210mm 297mm"),
        config::PrintTrimSize::A5 => Some("148mm 210mm"),
        config::PrintTrimSize::B6 => Some("128mm 182mm"),
        config::PrintTrimSize::Bunko => Some("105mm 148mm"),
        config::PrintTrimSize::Custom => None,
    }
}

fn escape_css_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\A ")
}

fn push_page_rule(css: &mut Vec<String>, selector: Option<&str>, lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }

    let mut rule = "@page".to_string();
    if let Some(selector) = selector {
        rule.push(' ');
        rule.push_str(selector);
    }
    css.push(format!("{rule} {{"));
    css.extend(lines);
    css.push("}".to_string());
}

fn suppressed_page_style_lines() -> Vec<String> {
    vec![
        "  @top-left { content: none; }".to_string(),
        "  @top-center { content: none; }".to_string(),
        "  @top-right { content: none; }".to_string(),
        "  @bottom-left { content: none; }".to_string(),
        "  @bottom-center { content: none; }".to_string(),
        "  @bottom-right { content: none; }".to_string(),
    ]
}

fn apply_typst_print_variables(
    options: &mut toolchain::PandocPdfOptions,
    pdf: &config::PdfSettings,
    print: Option<&config::PrintSettings>,
) {
    if let Some(print) = print {
        if let Some(papersize) = typst_papersize(print.trim_size) {
            options
                .variables
                .push(("papersize".to_string(), papersize.to_string()));
        }
        if let Some(margin) = print.page_margin.as_ref() {
            options.variable_json.push((
                "margin".to_string(),
                json!({
                    "top": margin.top,
                    "bottom": margin.bottom,
                    "left": margin.left,
                    "right": margin.right
                })
                .to_string(),
            ));
        }
    }
    if pdf.column_count > 1 {
        options
            .variables
            .push(("columns".to_string(), pdf.column_count.to_string()));
    }
    if pdf.base_font_size != "auto" {
        options
            .variables
            .push(("fontsize".to_string(), pdf.base_font_size.clone()));
    }
    if let Some(linestretch) = line_stretch_ratio(pdf) {
        options
            .variables
            .push(("linestretch".to_string(), linestretch));
    }
    if !pdf.page_number {
        options
            .variables
            .push(("page-numbering".to_string(), String::new()));
    }
}

fn typst_papersize(trim_size: config::PrintTrimSize) -> Option<&'static str> {
    match trim_size {
        config::PrintTrimSize::A4 => Some("a4"),
        config::PrintTrimSize::A5 => Some("a5"),
        config::PrintTrimSize::B6 => Some("b6"),
        config::PrintTrimSize::Bunko | config::PrintTrimSize::Custom => None,
    }
}

fn apply_lualatex_print_variables(
    options: &mut toolchain::PandocPdfOptions,
    pdf: &config::PdfSettings,
    print: Option<&config::PrintSettings>,
) {
    if pdf.column_count > 1 {
        options
            .variables
            .push(("classoption".to_string(), "twocolumn".to_string()));
    }
    if let Some(print) = print {
        match print.trim_size {
            config::PrintTrimSize::A4 => options
                .variables
                .push(("papersize".to_string(), "a4".to_string())),
            config::PrintTrimSize::A5 => options
                .variables
                .push(("papersize".to_string(), "a5".to_string())),
            _ => {}
        }
        if let Some((width, height)) = latex_paper_dimensions(print.trim_size) {
            options
                .variables
                .push(("geometry".to_string(), format!("paperwidth={width}")));
            options
                .variables
                .push(("geometry".to_string(), format!("paperheight={height}")));
        }
        if let Some(margin) = print.page_margin.as_ref() {
            for (side, value) in [
                ("top", &margin.top),
                ("bottom", &margin.bottom),
                ("left", &margin.left),
                ("right", &margin.right),
            ] {
                options
                    .variables
                    .push(("geometry".to_string(), format!("{side}={value}")));
            }
        }
    }
    if matches!(pdf.base_font_size.as_str(), "10pt" | "11pt" | "12pt") {
        options
            .variables
            .push(("fontsize".to_string(), pdf.base_font_size.clone()));
    }
    if let Some(linestretch) = line_stretch_ratio(pdf) {
        options
            .variables
            .push(("linestretch".to_string(), linestretch));
    }
    if !pdf.page_number && pdf.running_header == config::PdfRunningHeader::None {
        options
            .variables
            .push(("pagestyle".to_string(), "empty".to_string()));
    }
}

fn latex_paper_dimensions(
    trim_size: config::PrintTrimSize,
) -> Option<(&'static str, &'static str)> {
    match trim_size {
        config::PrintTrimSize::A4 => Some(("210mm", "297mm")),
        config::PrintTrimSize::A5 => Some(("148mm", "210mm")),
        config::PrintTrimSize::B6 => Some(("128mm", "182mm")),
        config::PrintTrimSize::Bunko => Some(("105mm", "148mm")),
        config::PrintTrimSize::Custom => None,
    }
}

fn line_stretch_ratio(pdf: &config::PdfSettings) -> Option<String> {
    let font = parse_numeric_length(&pdf.base_font_size)?;
    let line = parse_numeric_length(&pdf.line_height)?;
    if font.1 != line.1 || font.0 <= 0.0 {
        return None;
    }
    Some(format!("{:.4}", line.0 / font.0))
}

fn parse_numeric_length(value: &str) -> Option<(f64, &str)> {
    if value == "auto" {
        return None;
    }
    let boundary = value
        .find(|char: char| !(char.is_ascii_digit() || char == '.'))
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(boundary);
    if number.is_empty() || unit.is_empty() {
        return None;
    }
    Some((number.parse().ok()?, unit))
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

fn ensure_path_written(
    repo_root: &std::path::Path,
    book_id: &str,
    target: &str,
    tool_name: &str,
    expected_path: &std::path::Path,
    run_output: toolchain::ToolRunOutput,
) -> Result<(), BuildBookError> {
    if run_output.status.success() && expected_path.is_file() {
        return Ok(());
    }

    let log_path = build_log_path(repo_root, book_id, target);
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
    let log_contents = log_contents.replacen("tool: pandoc", &format!("tool: {tool_name}"), 1);
    let _ = write_build_log(&log_path, &log_contents);
    Err(BuildBookError::ExecutionFailed {
        target: target.to_string(),
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

    use crate::app::init_project::{InitProjectOptions, init_project};
    use crate::toolchain::{ToolRecord, ToolchainReport};

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-build-book-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn pdf_page_object_count_does_not_count_page_tree_nodes() {
        let pdf = br#"
1 0 obj
<< /Type /Pages /Count 2 /Kids [2 0 R 3 0 R] >>
endobj
2 0 obj
<< /Type /Page /Parent 1 0 R >>
endobj
3 0 obj
<< /Type /Page
   /Parent 1 0 R >>
endobj
"#;

        assert_eq!(pdf_page_object_count(pdf), 2);
    }

    fn fake_toolchain(pandoc_path: Option<PathBuf>) -> ToolchainReport {
        fake_toolchain_with_chromium(pandoc_path, None)
    }

    fn fake_toolchain_with_chromium(
        pandoc_path: Option<PathBuf>,
        chromium_path: Option<PathBuf>,
    ) -> ToolchainReport {
        ToolchainReport {
            tools: vec![
                ToolRecord {
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
                },
                ToolRecord {
                    key: "chromium",
                    display_name: "Chromium PDF",
                    status: if chromium_path.is_some() {
                        ToolStatus::Available
                    } else {
                        ToolStatus::Missing
                    },
                    detected_as: Some("chromium".to_string()),
                    resolved_path: chromium_path,
                    version: None,
                    install_hint:
                        "Install a Chromium-based browser and ensure its executable is available."
                            .to_string(),
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
git:
  lfs: true
"#,
        )
        .unwrap();
    }

    fn write_series_book(root: &std::path::Path) {
        init_project(InitProjectOptions {
            root: root.to_path_buf(),
            non_interactive: true,
            force: false,
            config_template: Some("business".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            initial_series_book_id: None,
            title: Some("Series Sample".to_string()),
            author: Some("Author".to_string()),
            language: Some("ja".to_string()),
            output_preset: Some("kindle".to_string()),
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            include_introduction: None,
            include_afterword: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
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
        write_print_book_with_pdf(
            root,
            "pdf:\n  engine: weasyprint\n  toc: true\n  page_number: true\n  running_header: auto\n",
        );
    }

    fn write_print_book_with_pdf(root: &std::path::Path, pdf_block: &str) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        fs::write(
            root.join("book.yml"),
            format!(
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

    fn write_vertical_print_book_with_pdf(root: &std::path::Path, pdf_block: &str) {
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

    fn write_chromium_print_book(root: &std::path::Path) {
        write_vertical_print_book_with_pdf(
            root,
            "pdf:\n  engine: chromium\n  toc: true\n  page_number: true\n  running_header: auto\n",
        );
    }

    fn write_chromium_print_book_without_toc(root: &std::path::Path) {
        write_vertical_print_book_with_pdf(
            root,
            "pdf:\n  engine: chromium\n  toc: false\n  page_number: true\n  running_header: auto\n",
        );
    }

    fn write_conference_preprint_book(root: &std::path::Path, engine: &str) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::create_dir_all(root.join("styles")).unwrap();
        fs::write(root.join("manuscript/01-main.md"), "# Main\n\n## Intro\n").unwrap();
        fs::write(root.join("styles/base.css"), "body { color: black; }\n").unwrap();
        fs::write(root.join("styles/print.css"), "body { widows: 2; }\n").unwrap();
        fs::write(
            root.join("book.yml"),
            format!(
                r#"
project:
  type: paper
  vcs: git
book:
  title: "Sample Preprint"
  authors:
    - "Author"
  reading_direction: ltr
  profile: conference-preprint
layout:
  binding: left
manuscript:
  chapters:
    - manuscript/01-main.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx4
pdf:
  engine: {engine}
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
  pdf_standard: pdfx4
validation:
  strict: true
git:
  lfs: true
"#
            ),
        )
        .unwrap();
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
    fn build_passes_base_and_epub_stylesheets_to_pandoc_epub() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-epub-css");
        write_book(&root);
        fs::create_dir_all(root.join("styles")).unwrap();
        fs::write(root.join("styles/base.css"), "body { color: black; }\n").unwrap();
        fs::write(
            root.join("styles/epub.css"),
            "body { background: white; }\n",
        )
        .unwrap();
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
printf 'fake epub' > "$out"
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
        let css_args = args
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|window| (window[0] == "--css").then_some(window[1]))
            .collect::<Vec<_>>();
        assert!(css_args.iter().any(|arg| arg.ends_with("/styles/base.css")));
        assert!(css_args.iter().any(|arg| arg.ends_with("/styles/epub.css")));
    }

    #[test]
    fn build_passes_shared_base_and_epub_stylesheets_to_pandoc_epub_for_series() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-series-epub-css");
        write_series_book(&root);
        fs::write(
            root.join("shared/styles/base.css"),
            "body { color: black; }\n",
        )
        .unwrap();
        fs::write(
            root.join("shared/styles/epub.css"),
            "body { background: white; }\n",
        )
        .unwrap();
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
printf 'fake epub' > "$out"
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
            &CommandContext::new(&root, Some("vol-01".to_string()), None),
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        let args = fs::read_to_string(args_path).unwrap();
        let css_args = args
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|window| (window[0] == "--css").then_some(window[1]))
            .collect::<Vec<_>>();
        assert!(
            css_args
                .iter()
                .any(|arg| arg.ends_with("/shared/styles/base.css"))
        );
        assert!(
            css_args
                .iter()
                .any(|arg| arg.ends_with("/shared/styles/epub.css"))
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
        assert_eq!(result.artifact_details()[0]["target_profile"], "business");
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["format"],
            "pdf"
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["print"]["pdf_engine"],
            "weasyprint"
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["print"]["page_numbering"],
            true
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
        assert!(args.lines().any(|arg| arg == "--pdf-engine"));
        assert!(args.lines().any(|arg| arg == "weasyprint"));
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
        assert!(args.lines().any(|arg| arg == "--pdf-engine"));
        assert!(args.lines().any(|arg| arg == "weasyprint"));
    }

    #[test]
    fn build_passes_pdf_engine_to_pandoc_pdf() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("fake-pandoc-print-engine");
        write_print_book_with_pdf(
            &root,
            "pdf:\n  engine: typst\n  toc: false\n  page_number: true\n  running_header: auto\n",
        );
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
        assert!(args.lines().any(|arg| arg == "--pdf-engine"));
        assert!(args.lines().any(|arg| arg == "typst"));
    }

    #[test]
    fn build_requires_chromium_for_vertical_print() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("missing-chromium");
        write_chromium_print_book(&root);
        let pandoc = root.join("pandoc");
        fs::write(&pandoc, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let error = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_chromium(Some(pandoc), None),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BuildBookError::RequiredToolMissing { tool, target }
                if tool == "chromium" && target == "print-jp-pdfx1a"
        ));
    }

    #[test]
    fn build_rejects_vertical_weasyprint_print() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("vertical-weasyprint");
        write_vertical_print_book_with_pdf(
            &root,
            "pdf:\n  engine: weasyprint\n  toc: true\n  page_number: true\n  running_header: auto\n",
        );
        let pandoc = root.join("pandoc");
        fs::write(&pandoc, "#!/bin/sh\nexit 0\n").unwrap();
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

        assert!(matches!(
            error,
            BuildBookError::UnsupportedVerticalWeasyprint { target }
                if target == "print-jp-pdfx1a"
        ));
    }

    #[test]
    fn build_passes_chromium_vertical_print_through_html_and_browser() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("chromium-print");
        write_chromium_print_book(&root);
        fs::create_dir_all(root.join("styles")).unwrap();
        fs::write(
            root.join("styles/base.css"),
            "body { writing-mode: vertical-rl; }\n",
        )
        .unwrap();
        fs::write(root.join("styles/print.css"), "body { color: black; }\n").unwrap();

        let pandoc = root.join("pandoc");
        let pandoc_args_path = root.join("pandoc-args.txt");
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
printf '<!doctype html><html><body>fake</body></html>' > "$out"
"#,
                pandoc_args_path.display()
            ),
        )
        .unwrap();

        let chromium = root.join("chromium");
        let chromium_args_path = root.join("chromium-args.txt");
        fs::write(
            &chromium,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{}"
out=""
for arg in "$@"; do
  case "$arg" in
    --print-to-pdf=*)
      out="${{arg#--print-to-pdf=}}"
      ;;
  esac
done
mkdir -p "$(dirname "$out")"
printf 'fake pdf' > "$out"
"#,
                chromium_args_path.display()
            ),
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for tool in [&pandoc, &chromium] {
                let mut permissions = fs::metadata(tool).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(tool, permissions).unwrap();
            }
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_chromium(Some(pandoc), Some(chromium)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        let pandoc_args = fs::read_to_string(pandoc_args_path).unwrap();
        assert!(pandoc_args.lines().any(|arg| arg == "html5"));
        assert!(pandoc_args.lines().any(|arg| arg == "--embed-resources"));
        let css_args = pandoc_args
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|window| (window[0] == "--css").then_some(window[1]))
            .collect::<Vec<_>>();
        assert!(css_args.iter().any(|arg| arg.ends_with("/styles/base.css")));
        assert!(
            css_args
                .iter()
                .any(|arg| arg.ends_with("/styles/print.css"))
        );
        assert!(css_args.iter().any(|arg| arg.ends_with(".layout.css")));
        let generated = css_args
            .iter()
            .find(|arg| arg.ends_with(".layout.css"))
            .expect("generated layout stylesheet must be passed");
        let generated_css = fs::read_to_string(generated).unwrap();
        assert!(generated_css.contains("header#title-block-header { break-after: avoid;"));
        assert!(generated_css.contains("nav#TOC { break-after: page;"));
        assert!(generated_css.contains("@page :left {"));
        assert!(generated_css.contains("@bottom-center { content: counter(page); }"));
        assert!(generated_css.contains("@top-center { content: string(shosei-heading); }"));
        assert!(generated_css.contains("@page :right {"));
        assert!(!generated_css.contains("@bottom-left { content: counter(page); }"));
        assert!(!generated_css.contains("@bottom-right { content: counter(page); }"));
        assert!(!generated_css.contains("@top-left { content: string(shosei-heading); }"));
        assert!(!generated_css.contains("@top-right { content: string(shosei-heading); }"));
        assert!(
            generated_css
                .contains("header#title-block-header, nav#TOC { page: shosei-frontmatter; }")
        );
        assert!(generated_css.contains("@page shosei-frontmatter:left {"));
        assert!(generated_css.contains("@page shosei-frontmatter:right {"));
        assert!(generated_css.contains("@bottom-left { content: none; }"));
        assert!(generated_css.contains("@bottom-right { content: none; }"));

        let chromium_args = fs::read_to_string(chromium_args_path).unwrap();
        assert!(chromium_args.lines().any(|arg| arg == "--headless=new"));
        assert!(
            chromium_args
                .lines()
                .any(|arg| arg == "--no-pdf-header-footer")
        );
        assert!(chromium_args.contains("--print-to-pdf="));
        assert!(chromium_args.contains("file://"));
        assert!(result.artifacts[0].with_extension("print.html").is_file());
    }

    #[test]
    fn build_breaks_after_title_when_vertical_print_disables_toc() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("chromium-print-no-toc");
        write_chromium_print_book_without_toc(&root);
        fs::create_dir_all(root.join("styles")).unwrap();
        fs::write(
            root.join("styles/base.css"),
            "body { writing-mode: vertical-rl; }\n",
        )
        .unwrap();
        fs::write(root.join("styles/print.css"), "body { color: black; }\n").unwrap();

        let pandoc = root.join("pandoc");
        let pandoc_args_path = root.join("pandoc-args.txt");
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
printf '<!doctype html><html><body>fake</body></html>' > "$out"
"#,
                pandoc_args_path.display()
            ),
        )
        .unwrap();

        let chromium = root.join("chromium");
        fs::write(
            &chromium,
            r#"#!/bin/sh
out=""
for arg in "$@"; do
  case "$arg" in
    --print-to-pdf=*)
      out="${arg#--print-to-pdf=}"
      ;;
  esac
done
mkdir -p "$(dirname "$out")"
printf 'fake pdf' > "$out"
"#,
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for tool in [&pandoc, &chromium] {
                let mut permissions = fs::metadata(tool).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(tool, permissions).unwrap();
            }
        }

        let result = build_book_with_toolchain(
            &CommandContext::new(&root, None, None),
            &fake_toolchain_with_chromium(Some(pandoc), Some(chromium)),
        )
        .unwrap();

        assert!(result.artifacts[0].is_file());
        let pandoc_args = fs::read_to_string(pandoc_args_path).unwrap();
        assert!(!pandoc_args.lines().any(|arg| arg == "--toc"));
        let generated = result.artifacts[0].with_extension("layout.css");
        let generated_css = fs::read_to_string(generated).unwrap();
        assert!(generated_css.contains("header#title-block-header { break-after: page;"));
        assert!(!generated_css.contains("nav#TOC { break-after: page;"));
        assert!(generated_css.contains("@page :left {"));
        assert!(generated_css.contains("@bottom-center { content: counter(page); }"));
        assert!(generated_css.contains("@page :right {"));
        assert!(!generated_css.contains("@bottom-left { content: counter(page); }"));
        assert!(!generated_css.contains("@bottom-right { content: counter(page); }"));
        assert!(generated_css.contains("header#title-block-header { page: shosei-frontmatter; }"));
        assert!(generated_css.contains("@page shosei-frontmatter:first {"));
    }

    #[test]
    fn build_writes_generated_weasyprint_layout_css_for_preprint() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("preprint-weasyprint-layout");
        write_conference_preprint_book(&root, "weasyprint");
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
        let css_args = args
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|window| (window[0] == "--css").then_some(window[1]))
            .collect::<Vec<_>>();
        assert!(css_args.iter().any(|arg| arg.ends_with("/styles/base.css")));
        assert!(
            css_args
                .iter()
                .any(|arg| arg.ends_with("/styles/print.css"))
        );
        let generated = css_args
            .iter()
            .find(|arg| arg.ends_with(".layout.css"))
            .expect("generated layout stylesheet must be passed");
        let css = fs::read_to_string(generated).unwrap();
        assert!(css.contains("size: 210mm 297mm;"));
        assert!(css.contains("margin: 20mm 15mm 20mm 15mm;"));
        assert!(css.contains("column-count: 2;"));
        assert!(css.contains("column-gap: 10mm;"));
        assert!(css.contains("font-size: 9pt;"));
        assert!(css.contains("line-height: 14pt;"));
        assert!(!css.contains("title-block-header"));
    }

    #[test]
    fn build_passes_typst_print_variables_for_preprint() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("preprint-typst-variables");
        write_conference_preprint_book(&root, "typst");
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
        assert!(args.contains("--variable\ncolumns=2"));
        assert!(args.contains("--variable\npapersize=a4"));
        assert!(args.contains("--variable\nfontsize=9pt"));
        assert!(args.contains("--variable\nlinestretch=1.5556"));
        assert!(args.contains("--variable-json\nmargin="));
        assert!(args.contains("\"top\":\"20mm\""));
        assert!(args.contains("\"bottom\":\"20mm\""));
        assert!(args.contains("\"left\":\"15mm\""));
        assert!(args.contains("\"right\":\"15mm\""));
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
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["kindle"]["fixed_layout"],
            true
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["manga"]["source_page_count"],
            1
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["manga"]["rendered_page_count"],
            2
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["manga"]["split_source_page_count"],
            1
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["manga"]["unique_page_dimensions"][0]
                ["width_px"],
            1
        );
        assert_eq!(
            result.artifact_details()[0]["artifact_metadata"]["manga"]["unique_page_dimensions"][0]
                ["height_px"],
            1
        );
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
