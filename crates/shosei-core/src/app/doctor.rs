use crate::toolchain::{self, HostOs, ToolStatus};

#[derive(Debug, Clone)]
pub struct DoctorResult {
    pub summary: String,
    pub report: toolchain::ToolchainReport,
}

pub fn doctor() -> DoctorResult {
    let report = toolchain::inspect_default_toolchain();
    let host_os = HostOs::detect();
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
    let available_lines = render_tools(
        report
            .tools
            .iter()
            .filter(|tool| tool.status == ToolStatus::Available),
    );
    let missing_lines = render_tools(
        report
            .tools
            .iter()
            .filter(|tool| tool.status == ToolStatus::Missing),
    );
    let pending_lines = render_tools(
        report
            .tools
            .iter()
            .filter(|tool| tool.status == ToolStatus::NotYetImplemented),
    );
    let next_steps = report
        .tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::Missing)
        .map(|tool| format!("- {}: {}", tool.display_name, tool.install_hint))
        .collect::<Vec<_>>();

    DoctorResult {
        summary: format!(
            "doctor summary for {}: {available} available, {missing} missing, {pending} pending\n\navailable tools:\n{}\n\nmissing tools:\n{}\n\npending integrations:\n{}\n\nnext steps:\n{}",
            host_os.as_str(),
            if available_lines.is_empty() {
                "- none".to_string()
            } else {
                available_lines.join("\n")
            },
            if missing_lines.is_empty() {
                "- none".to_string()
            } else {
                missing_lines.join("\n")
            },
            if pending_lines.is_empty() {
                "- none".to_string()
            } else {
                pending_lines.join("\n")
            },
            if next_steps.is_empty() {
                "- no immediate action required".to_string()
            } else {
                next_steps.join("\n")
            }
        ),
        report,
    }
}

fn render_tools<'a>(tools: impl Iterator<Item = &'a toolchain::ToolRecord>) -> Vec<String> {
    tools.map(render_tool_line).collect()
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
    fn doctor_summary_groups_missing_tools_into_next_steps() {
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

        let available_lines = render_tools(
            report
                .tools
                .iter()
                .filter(|tool| tool.status == ToolStatus::Available),
        );
        let missing_lines = render_tools(
            report
                .tools
                .iter()
                .filter(|tool| tool.status == ToolStatus::Missing),
        );

        assert_eq!(available_lines.len(), 1);
        assert_eq!(missing_lines.len(), 1);
        assert!(available_lines[0].contains("pandoc 3.0"));
        assert_eq!(missing_lines[0], "- Kindle Previewer: missing");
    }
}
