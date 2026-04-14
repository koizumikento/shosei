use crate::toolchain::{self, HostOs, ToolStatus};

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
    ("epubcheck", DoctorToolCategory::Optional),
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
    let host_os = HostOs::detect();
    let tools = display_tools(&report);
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
    let snapshot = build_snapshot(host_os, counts, &tools);

    DoctorResult {
        summary: format!(
            "doctor summary for {}: required {} available, {} missing, {} pending; optional {} available, {} missing, {} pending\n\nrequired tools:\n{}\n\noptional tools:\n{}\n\nnext steps:\n{}",
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
}
