use std::fs;

use shosei_core::{app, cli_api::CommandContext, config::ConfigError, pipeline::PipelineError};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shosei-book-commands-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn build_reports_config_errors_without_collapsing_them() {
    let root = temp_dir("invalid-config");
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
outputs:
  kindle:
    enabled: true
validation:
  strict: true
git:
  lfs: true
"#,
    )
    .unwrap();

    let error = app::build_book(&CommandContext::new(&root, None)).unwrap_err();
    assert!(matches!(
        error,
        app::BuildBookError::Config(ConfigError::MissingField { field, .. }) if field == "manuscript.chapters"
    ));
}

#[test]
fn validate_reports_config_errors_without_collapsing_them() {
    let root = temp_dir("invalid-validate");
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
validation:
  strict: true
git:
  lfs: true
"#,
    )
    .unwrap();

    let error = app::validate_book(&CommandContext::new(&root, None)).unwrap_err();
    assert!(matches!(
        error,
        app::ValidateBookError::Config(ConfigError::NoEnabledOutputs { .. })
    ));
}

#[test]
fn build_fails_when_manuscript_file_is_missing() {
    let root = temp_dir("missing-manuscript-build");
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

    let error = app::build_book(&CommandContext::new(&root, None)).unwrap_err();
    match error {
        app::BuildBookError::Pipeline(PipelineError::PreflightFailed { diagnostics, .. }) => {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code, "missing-manuscript");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn validate_reports_missing_manuscript_in_report() {
    let root = temp_dir("missing-manuscript-validate");
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

    let result = app::validate_book(&CommandContext::new(&root, None)).unwrap();
    assert!(result.has_errors);
    assert_eq!(result.issue_count, 1);
    assert!(result.report_path.is_file());
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("manuscript file not found"));
    assert!(report.contains("manuscript/01.md"));
}
