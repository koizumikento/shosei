use std::path::Path;

use crate::{
    config::{self, ResolvedBookConfig},
    domain::RepoMode,
    repo,
    toolchain::{self, HostOs, ToolStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorToolCategory {
    Required,
    Optional,
}

impl DoctorToolCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
        }
    }
}

const DOCTOR_TOOL_MATRIX: &[(&str, DoctorToolCategory)] = &[
    ("git", DoctorToolCategory::Required),
    ("pandoc", DoctorToolCategory::Required),
    ("weasyprint", DoctorToolCategory::Required),
    ("chromium", DoctorToolCategory::Required),
    ("typst", DoctorToolCategory::Optional),
    ("lualatex", DoctorToolCategory::Optional),
    ("epubcheck", DoctorToolCategory::Optional),
    ("qpdf", DoctorToolCategory::Optional),
    ("git-lfs", DoctorToolCategory::Optional),
    ("kindle-previewer", DoctorToolCategory::Optional),
];

#[derive(Debug, Clone)]
pub struct DoctorResult {
    pub summary: String,
    pub report: toolchain::ToolchainReport,
    pub snapshot: DoctorSnapshot,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorSnapshot {
    pub host_os: String,
    pub available: usize,
    pub missing: usize,
    pub pending: usize,
    pub required_available: usize,
    pub required_missing: usize,
    pub required_pending: usize,
    pub optional_available: usize,
    pub optional_missing: usize,
    pub optional_pending: usize,
    pub tools: Vec<DoctorSnapshotTool>,
    pub detected_project: Option<DoctorDetectedProject>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorSnapshotTool {
    pub category: String,
    pub key: String,
    pub display_name: String,
    pub status: String,
    pub detected_as: Option<String>,
    pub resolved_path: Option<String>,
    pub version: Option<String>,
    pub install_hint: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorDetectedProject {
    pub repo_mode: String,
    pub book_id: Option<String>,
    pub project_type: Option<String>,
    pub enabled_outputs: Vec<String>,
    pub focused_required_tools: Vec<String>,
    pub focused_optional_tools: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct DoctorDisplayTool<'a> {
    category: DoctorToolCategory,
    record: &'a toolchain::ToolRecord,
}

#[derive(Debug, Clone, Copy)]
struct DoctorCounts {
    available: usize,
    missing: usize,
    pending: usize,
    required_available: usize,
    required_missing: usize,
    required_pending: usize,
    optional_available: usize,
    optional_missing: usize,
    optional_pending: usize,
}

pub fn doctor() -> DoctorResult {
    let report = toolchain::inspect_default_toolchain();
    let cwd = std::env::current_dir().ok();
    doctor_with_report_and_path(report, cwd.as_deref())
}

fn doctor_with_report_and_path(
    report: toolchain::ToolchainReport,
    start_path: Option<&Path>,
) -> DoctorResult {
    let host_os = HostOs::detect();
    let tools = display_tools(&report);
    let detected_project = detect_project(start_path);
    let (required_available, required_missing, required_pending) =
        category_counts(&tools, DoctorToolCategory::Required);
    let (optional_available, optional_missing, optional_pending) =
        category_counts(&tools, DoctorToolCategory::Optional);
    let counts = DoctorCounts {
        available: required_available + optional_available,
        missing: required_missing + optional_missing,
        pending: required_pending + optional_pending,
        required_available,
        required_missing,
        required_pending,
        optional_available,
        optional_missing,
        optional_pending,
    };
    let required_lines = render_tools(
        tools
            .iter()
            .copied()
            .filter(|tool| tool.category == DoctorToolCategory::Required),
    );
    let optional_lines = render_tools(
        tools
            .iter()
            .copied()
            .filter(|tool| tool.category == DoctorToolCategory::Optional),
    );
    let next_steps = tools
        .iter()
        .filter(|tool| tool.record.status == ToolStatus::Missing)
        .map(|tool| {
            format!(
                "- {} {}: {}",
                tool.category.as_str(),
                tool.record.display_name,
                tool.record.install_hint
            )
        })
        .collect::<Vec<_>>();
    let snapshot = build_snapshot(host_os, counts, &tools, detected_project.clone());
    let project_section = render_detected_project(&detected_project);

    DoctorResult {
        summary: format!(
            "doctor summary for {}: required {} available, {} missing, {} pending; optional {} available, {} missing, {} pending\n\nrequired tools:\n{}\n\noptional tools:\n{}\n{}\n\nnext steps:\n{}",
            host_os.as_str(),
            counts.required_available,
            counts.required_missing,
            counts.required_pending,
            counts.optional_available,
            counts.optional_missing,
            counts.optional_pending,
            if required_lines.is_empty() {
                "- none".to_string()
            } else {
                required_lines.join("\n")
            },
            if optional_lines.is_empty() {
                "- none".to_string()
            } else {
                optional_lines.join("\n")
            },
            project_section,
            if next_steps.is_empty() {
                "- no immediate action required".to_string()
            } else {
                next_steps.join("\n")
            }
        ),
        report,
        snapshot,
    }
}

fn build_snapshot(
    host_os: HostOs,
    counts: DoctorCounts,
    tools: &[DoctorDisplayTool<'_>],
    detected_project: Option<DoctorDetectedProject>,
) -> DoctorSnapshot {
    DoctorSnapshot {
        host_os: host_os.as_str().to_string(),
        available: counts.available,
        missing: counts.missing,
        pending: counts.pending,
        required_available: counts.required_available,
        required_missing: counts.required_missing,
        required_pending: counts.required_pending,
        optional_available: counts.optional_available,
        optional_missing: counts.optional_missing,
        optional_pending: counts.optional_pending,
        detected_project,
        tools: tools
            .iter()
            .map(|tool| DoctorSnapshotTool {
                category: tool.category.as_str().to_string(),
                key: tool.record.key.to_string(),
                display_name: tool.record.display_name.to_string(),
                status: tool.record.status.to_string(),
                detected_as: tool.record.detected_as.clone(),
                resolved_path: tool
                    .record
                    .resolved_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
                version: tool.record.version.clone(),
                install_hint: tool.record.install_hint.clone(),
            })
            .collect(),
    }
}

fn detect_project(start_path: Option<&Path>) -> Option<DoctorDetectedProject> {
    let start_path = start_path?;
    let context = repo::discover(start_path, None).ok()?;
    let mut detected = DoctorDetectedProject {
        repo_mode: match context.mode {
            RepoMode::SingleBook => "single-book".to_string(),
            RepoMode::Series => "series".to_string(),
        },
        book_id: context.book.as_ref().map(|book| book.id.clone()),
        project_type: None,
        enabled_outputs: Vec::new(),
        focused_required_tools: vec!["git".to_string()],
        focused_optional_tools: Vec::new(),
        notes: Vec::new(),
    };

    if context.mode == RepoMode::Series && context.book.is_none() {
        detected.notes.push(
            "series repo root detected; run doctor inside books/<book-id>/... for book-specific tool requirements".to_string(),
        );
        return Some(detected);
    }

    match config::resolve_book_config(&context) {
        Ok(resolved) => apply_resolved_book_context(&mut detected, &resolved),
        Err(error) => detected.notes.push(format!(
            "detected repo but could not resolve current book config: {error}"
        )),
    }

    Some(detected)
}

fn apply_resolved_book_context(
    detected: &mut DoctorDetectedProject,
    resolved: &ResolvedBookConfig,
) {
    let invalid_vertical_weasyprint = resolved.effective.project.project_type.is_prose()
        && resolved.effective.outputs.print.is_some()
        && resolved.effective.book.writing_mode == config::WritingMode::VerticalRl
        && resolved
            .effective
            .pdf
            .as_ref()
            .is_some_and(|pdf| pdf.engine == config::PdfEngine::Weasyprint);

    detected.project_type = Some(resolved.effective.project.project_type.as_str().to_string());
    if resolved.effective.outputs.kindle.is_some() {
        detected.enabled_outputs.push("kindle".to_string());
    }
    if resolved.effective.outputs.print.is_some() {
        detected.enabled_outputs.push("print".to_string());
    }

    if resolved.effective.project.project_type.is_prose()
        && !detected
            .focused_required_tools
            .iter()
            .any(|tool| tool == "pandoc")
    {
        detected.focused_required_tools.push("pandoc".to_string());
    }
    if resolved.effective.outputs.print.is_some()
        && resolved.effective.project.project_type.is_prose()
        && let Some(pdf) = resolved.effective.pdf.as_ref()
    {
        let engine = if invalid_vertical_weasyprint {
            "chromium".to_string()
        } else {
            pdf.engine.as_str().to_string()
        };
        if !detected
            .focused_required_tools
            .iter()
            .any(|tool| tool == &engine)
        {
            detected.focused_required_tools.push(engine);
        }
        if matches!(
            pdf.engine,
            config::PdfEngine::Typst | config::PdfEngine::Lualatex
        ) {
            detected.notes.push(format!(
                "pdf.engine = {} is accepted in v0.1 but is less validated than the default weasyprint/chromium paths; run an extra proof build before handoff",
                pdf.engine.as_str()
            ));
        }
    }
    if resolved.effective.outputs.kindle.is_some() && resolved.effective.validation.epubcheck {
        detected
            .focused_optional_tools
            .push("epubcheck".to_string());
    }
    if resolved.effective.outputs.kindle.is_some() && resolved.effective.validation.kindle_previewer
    {
        detected
            .focused_optional_tools
            .push("kindle-previewer".to_string());
    }
    if resolved.effective.outputs.print.is_some() {
        detected.focused_optional_tools.push("qpdf".to_string());
    }
    if resolved.effective.git.lfs {
        detected.focused_optional_tools.push("git-lfs".to_string());
    }
    detected.focused_optional_tools.sort();
    detected.focused_optional_tools.dedup();

    if invalid_vertical_weasyprint {
        detected.notes.push(
            "vertical-rl prose print requires chromium at build time; current config still points to weasyprint".to_string(),
        );
    }
}

fn render_detected_project(detected_project: &Option<DoctorDetectedProject>) -> String {
    let Some(detected_project) = detected_project else {
        return String::new();
    };

    let outputs = if detected_project.enabled_outputs.is_empty() {
        "none".to_string()
    } else {
        detected_project.enabled_outputs.join(", ")
    };
    let focused_required = if detected_project.focused_required_tools.is_empty() {
        "none".to_string()
    } else {
        detected_project.focused_required_tools.join(", ")
    };
    let focused_optional = if detected_project.focused_optional_tools.is_empty() {
        "none".to_string()
    } else {
        detected_project.focused_optional_tools.join(", ")
    };
    let notes = if detected_project.notes.is_empty() {
        "- none".to_string()
    } else {
        detected_project
            .notes
            .iter()
            .map(|note| format!("- {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "\ndetected project:\n- repo mode: {}\n- book: {}\n- project type: {}\n- enabled outputs: {}\n- focused required tools: {}\n- focused optional tools: {}\n- notes:\n{}",
        detected_project.repo_mode,
        detected_project.book_id.as_deref().unwrap_or("none"),
        detected_project
            .project_type
            .as_deref()
            .unwrap_or("unknown"),
        outputs,
        focused_required,
        focused_optional,
        notes
    )
}

fn display_tools(report: &toolchain::ToolchainReport) -> Vec<DoctorDisplayTool<'_>> {
    DOCTOR_TOOL_MATRIX
        .iter()
        .filter_map(|(key, category)| {
            report.tool(key).map(|record| DoctorDisplayTool {
                category: *category,
                record,
            })
        })
        .collect()
}

fn category_counts(
    tools: &[DoctorDisplayTool<'_>],
    category: DoctorToolCategory,
) -> (usize, usize, usize) {
    let mut available = 0;
    let mut missing = 0;
    let mut pending = 0;

    for tool in tools.iter().filter(|tool| tool.category == category) {
        match tool.record.status {
            ToolStatus::Available => available += 1,
            ToolStatus::Missing => missing += 1,
            ToolStatus::NotYetImplemented => pending += 1,
            ToolStatus::Planned => {}
        }
    }

    (available, missing, pending)
}

fn render_tools<'a>(tools: impl Iterator<Item = DoctorDisplayTool<'a>>) -> Vec<String> {
    tools.map(|tool| render_tool_line(tool.record)).collect()
}

fn render_tool_line(tool: &toolchain::ToolRecord) -> String {
    match (&tool.resolved_path, &tool.version, &tool.detected_as) {
        (Some(path), Some(version), Some(detected_as)) => format!(
            "- {}: {} ({}, {}, detected as {})",
            tool.display_name,
            tool.status,
            path.display(),
            version,
            detected_as
        ),
        (Some(path), Some(version), None) => format!(
            "- {}: {} ({}, {})",
            tool.display_name,
            tool.status,
            path.display(),
            version
        ),
        (Some(path), None, Some(detected_as)) => format!(
            "- {}: {} ({}, detected as {})",
            tool.display_name,
            tool.status,
            path.display(),
            detected_as
        ),
        (Some(path), None, None) => format!(
            "- {}: {} ({})",
            tool.display_name,
            tool.status,
            path.display()
        ),
        (None, _, _) => format!("- {}: {}", tool.display_name, tool.status),
    }
}

#[cfg(test)]
mod tests {
    use crate::toolchain::{ToolRecord, ToolchainReport};

    use super::*;

    #[test]
    fn doctor_summary_groups_required_and_optional_tools() {
        let report = ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: ToolStatus::Available,
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: Some("/tmp/pandoc".into()),
                    version: Some("pandoc 3.0".to_string()),
                    install_hint: "Install pandoc.".to_string(),
                },
                ToolRecord {
                    key: "kindle-previewer",
                    display_name: "Kindle Previewer",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install Kindle Previewer.".to_string(),
                },
            ],
        };

        let tools = display_tools(&report);
        let required_lines = render_tools(
            tools
                .iter()
                .copied()
                .filter(|tool| tool.category == DoctorToolCategory::Required),
        );
        let optional_lines = render_tools(
            tools
                .iter()
                .copied()
                .filter(|tool| tool.category == DoctorToolCategory::Optional),
        );

        assert_eq!(required_lines.len(), 1);
        assert_eq!(optional_lines.len(), 1);
        assert!(required_lines[0].contains("pandoc 3.0"));
        assert_eq!(optional_lines[0], "- Kindle Previewer: missing");
    }

    #[test]
    fn doctor_snapshot_preserves_category_and_tool_status_details() {
        let report = ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: ToolStatus::Available,
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: Some("/tmp/pandoc".into()),
                    version: Some("pandoc 3.0".to_string()),
                    install_hint: "Install pandoc.".to_string(),
                },
                ToolRecord {
                    key: "kindle-previewer",
                    display_name: "Kindle Previewer",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install Kindle Previewer.".to_string(),
                },
            ],
        };

        let tools = display_tools(&report);
        let snapshot = build_snapshot(
            HostOs::Macos,
            DoctorCounts {
                available: 1,
                missing: 1,
                pending: 0,
                required_available: 1,
                required_missing: 0,
                required_pending: 0,
                optional_available: 0,
                optional_missing: 1,
                optional_pending: 0,
            },
            &tools,
            None,
        );

        assert_eq!(snapshot.host_os, "macOS");
        assert_eq!(snapshot.available, 1);
        assert_eq!(snapshot.missing, 1);
        assert_eq!(snapshot.pending, 0);
        assert_eq!(snapshot.required_available, 1);
        assert_eq!(snapshot.optional_missing, 1);
        assert_eq!(snapshot.tools[0].category, "required");
        assert_eq!(snapshot.tools[0].status, "available");
        assert_eq!(
            snapshot.tools[0].resolved_path.as_deref(),
            Some("/tmp/pandoc")
        );
        assert_eq!(snapshot.tools[1].category, "optional");
        assert_eq!(snapshot.tools[1].display_name, "Kindle Previewer");
        assert_eq!(snapshot.tools[1].status, "missing");
    }

    #[test]
    fn doctor_detects_project_specific_tool_focus_for_current_book() {
        let root =
            std::env::temp_dir().join(format!("shosei-doctor-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("manuscript")).unwrap();
        std::fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        std::fs::write(
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
  engine: chromium
validation:
  strict: true
  epubcheck: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let result = doctor_with_report_and_path(ToolchainReport { tools: vec![] }, Some(&root));
        let project = result
            .snapshot
            .detected_project
            .expect("project should be detected");

        assert_eq!(project.repo_mode, "single-book");
        assert_eq!(project.book_id.as_deref(), Some("default"));
        assert_eq!(project.project_type.as_deref(), Some("novel"));
        assert_eq!(project.enabled_outputs, vec!["kindle", "print"]);
        assert_eq!(
            project.focused_required_tools,
            vec!["git", "pandoc", "chromium"]
        );
        assert_eq!(
            project.focused_optional_tools,
            vec!["epubcheck", "git-lfs", "qpdf"]
        );
    }

    #[test]
    fn doctor_notes_series_root_without_selected_book() {
        let root =
            std::env::temp_dir().join(format!("shosei-doctor-series-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("books/vol-01")).unwrap();
        std::fs::write(
            root.join("series.yml"),
            r#"
series:
  id: sample
  title: Sample
  type: novel
books:
  - id: vol-01
    path: books/vol-01
"#,
        )
        .unwrap();

        let result = doctor_with_report_and_path(ToolchainReport { tools: vec![] }, Some(&root));
        let project = result
            .snapshot
            .detected_project
            .expect("project should be detected");

        assert_eq!(project.repo_mode, "series");
        assert_eq!(project.book_id, None);
        assert!(
            project
                .notes
                .iter()
                .any(|note| note.contains("series repo root detected"))
        );
    }

    #[test]
    fn doctor_prefers_chromium_focus_for_vertical_print_even_when_config_is_invalid() {
        let root = std::env::temp_dir().join(format!(
            "shosei-doctor-vertical-weasyprint-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("manuscript")).unwrap();
        std::fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        std::fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
book:
  title: "Sample"
  authors:
    - "Author"
  writing_mode: vertical-rl
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
"#,
        )
        .unwrap();

        let result = doctor_with_report_and_path(ToolchainReport { tools: vec![] }, Some(&root));
        let project = result
            .snapshot
            .detected_project
            .expect("project should be detected");

        assert_eq!(
            project.focused_required_tools,
            vec!["git", "pandoc", "chromium"]
        );
        assert!(
            project
                .notes
                .iter()
                .any(|note| note.contains("requires chromium at build time"))
        );
    }

    #[test]
    fn doctor_keeps_configured_engine_for_vertical_typst_print() {
        let root = std::env::temp_dir().join(format!(
            "shosei-doctor-vertical-typst-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("manuscript")).unwrap();
        std::fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
        std::fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
book:
  title: "Sample"
  authors:
    - "Author"
  writing_mode: vertical-rl
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
"#,
        )
        .unwrap();

        let result = doctor_with_report_and_path(ToolchainReport { tools: vec![] }, Some(&root));
        let project = result
            .snapshot
            .detected_project
            .expect("project should be detected");

        assert_eq!(
            project.focused_required_tools,
            vec!["git", "pandoc", "typst"]
        );
        assert!(
            project
                .notes
                .iter()
                .any(|note| note.contains("less validated than the default"))
        );
    }

    #[test]
    fn doctor_summary_lists_typst_and_lualatex_as_optional_tools() {
        let report = ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "git",
                    display_name: "git",
                    status: ToolStatus::Available,
                    detected_as: Some("git".to_string()),
                    resolved_path: Some("/tmp/git".into()),
                    version: Some("git version 2.0".to_string()),
                    install_hint: "Install git.".to_string(),
                },
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: ToolStatus::Available,
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: Some("/tmp/pandoc".into()),
                    version: Some("pandoc 3.0".to_string()),
                    install_hint: "Install pandoc.".to_string(),
                },
                ToolRecord {
                    key: "weasyprint",
                    display_name: "weasyprint",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install weasyprint.".to_string(),
                },
                ToolRecord {
                    key: "chromium",
                    display_name: "Chromium PDF",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install Chromium.".to_string(),
                },
                ToolRecord {
                    key: "typst",
                    display_name: "typst",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install typst.".to_string(),
                },
                ToolRecord {
                    key: "lualatex",
                    display_name: "lualatex",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install lualatex.".to_string(),
                },
                ToolRecord {
                    key: "epubcheck",
                    display_name: "epubcheck",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install epubcheck.".to_string(),
                },
                ToolRecord {
                    key: "qpdf",
                    display_name: "qpdf",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install qpdf.".to_string(),
                },
                ToolRecord {
                    key: "git-lfs",
                    display_name: "git-lfs",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install git-lfs.".to_string(),
                },
                ToolRecord {
                    key: "kindle-previewer",
                    display_name: "Kindle Previewer",
                    status: ToolStatus::Missing,
                    detected_as: None,
                    resolved_path: None,
                    version: None,
                    install_hint: "Install Kindle Previewer.".to_string(),
                },
            ],
        };

        let result = doctor_with_report_and_path(report, None);

        assert!(result.summary.contains("- typst: missing"));
        assert!(result.summary.contains("- lualatex: missing"));
        assert!(result.summary.contains("- qpdf: missing"));
    }
}
