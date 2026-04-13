use crate::toolchain::{self, ToolStatus};

#[derive(Debug, Clone)]
pub struct DoctorResult {
    pub summary: String,
    pub report: toolchain::ToolchainReport,
}

pub fn doctor() -> DoctorResult {
    let report = toolchain::inspect_default_toolchain();
    let available = report
        .tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::Available)
        .count();
    let missing = report
        .tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::Missing)
        .count();
    let pending = report
        .tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::NotYetImplemented)
        .count();

    DoctorResult {
        summary: format!(
            "doctor summary: {available} available, {missing} missing, {pending} pending\n{}",
            report
                .tools
                .iter()
                .map(|tool| match (&tool.resolved_path, &tool.version) {
                    (Some(path), Some(version)) => format!(
                        "- {}: {} ({}, {})",
                        tool.display_name,
                        tool.status,
                        path.display(),
                        version
                    ),
                    (Some(path), None) => {
                        format!(
                            "- {}: {} ({})",
                            tool.display_name,
                            tool.status,
                            path.display()
                        )
                    }
                    (None, _) => format!("- {}: {}", tool.display_name, tool.status),
                })
                .collect::<Vec<_>>()
                .join("\n")
        ),
        report,
    }
}
