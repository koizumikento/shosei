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
                .map(
                    |tool| match (&tool.resolved_path, &tool.version, &tool.detected_as) {
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
                        (None, _, _) => format!(
                            "- {}: {} ({})",
                            tool.display_name, tool.status, tool.install_hint
                        ),
                    }
                )
                .collect::<Vec<_>>()
                .join("\n")
        ),
        report,
    }
}
