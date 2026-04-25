use std::io::{self, Write};

use shosei_core::diagnostics::{Severity, ValidationIssue};

const ISSUE_PREVIEW_LIMIT: usize = 5;

pub fn print_line(message: &str) {
    println!("{message}");
    let _ = io::stdout().flush();
}

pub fn format_issue_preview(issues: &[ValidationIssue]) -> Option<String> {
    if issues.is_empty() {
        return None;
    }

    let mut lines = vec!["issues:".to_string()];
    for issue in issues.iter().take(ISSUE_PREVIEW_LIMIT) {
        lines.push(format!(
            "- [{}] {}",
            match issue.severity {
                Severity::Warning => "warn",
                Severity::Error => "error",
            },
            issue.cause
        ));
        if let Some(location) = &issue.location {
            lines.push(format!("  location: {location}"));
        }
        lines.push(format!("  remedy: {}", issue.remedy));
    }

    if issues.len() > ISSUE_PREVIEW_LIMIT {
        lines.push(format!(
            "- ... and {} more",
            issues.len() - ISSUE_PREVIEW_LIMIT
        ));
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use shosei_core::diagnostics::{IssueLocation, ValidationIssue};
    use std::path::PathBuf;

    use super::format_issue_preview;

    #[test]
    fn formats_issue_preview_with_location_and_remedy() {
        let preview = format_issue_preview(&[ValidationIssue::error(
            "common",
            "link target not found: missing.md",
            "リンク先パスを修正してください。",
        )
        .at_location(IssueLocation::with_line(
            PathBuf::from("manuscript/01.md"),
            5,
        ))])
        .unwrap();

        assert!(preview.contains("issues:"));
        assert!(preview.contains("[error] link target not found: missing.md"));
        assert!(preview.contains("location: manuscript/01.md:5"));
        assert!(preview.contains("remedy: リンク先パスを修正してください。"));
    }

    #[test]
    fn formats_issue_preview_with_truncation_notice() {
        let issues = (1..=7)
            .map(|index| {
                ValidationIssue::error("common", format!("issue {index}"), "修正してください。")
            })
            .collect::<Vec<_>>();

        let preview = format_issue_preview(&issues).unwrap();

        assert_eq!(
            preview
                .lines()
                .filter(|line| line.starts_with("- [error] issue"))
                .count(),
            5
        );
        assert!(preview.contains("- ... and 2 more"));
    }
}
