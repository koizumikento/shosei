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

#[test]
fn explain_shows_single_book_origins() {
    let root = temp_dir("explain-single");
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: novel
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
cover:
  ebook_image: assets/cover/front.jpg
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
"#,
    )
    .unwrap();

    let result = app::explain_config(&CommandContext::new(&root, None)).unwrap();
    assert!(result.summary.contains("explain for default"));
    assert!(result.summary.contains("book.title = Sample [book.yml]"));
    assert!(
        result
            .summary
            .contains("cover.ebook_image = assets/cover/front.jpg [book.yml]")
    );
    assert!(result.summary.contains("pdf.toc = true [built-in default]"));
    assert!(
        result
            .summary
            .contains("book.language = ja [built-in default]")
    );
}

#[test]
fn explain_shows_series_default_origins_and_shared_paths() {
    let root = temp_dir("explain-series");
    fs::create_dir_all(root.join("books/vol-01")).unwrap();
    fs::write(
        root.join("series.yml"),
        r#"
series:
  id: sample
  title: Sample
  type: novel
defaults:
  book:
    language: ja
    writing_mode: vertical-rl
    reading_direction: rtl
  outputs:
    kindle:
      enabled: true
      target: kindle-ja
shared:
  assets:
    - shared/assets
books:
  - id: vol-01
    path: books/vol-01
"#,
    )
    .unwrap();
    fs::write(
        root.join("books/vol-01/book.yml"),
        r#"
project:
  type: novel
book:
  title: "Vol 1"
  authors:
    - "Author"
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
"#,
    )
    .unwrap();

    let result =
        app::explain_config(&CommandContext::new(&root, Some("vol-01".to_string()))).unwrap();
    assert!(
        result
            .summary
            .contains("book.language = ja [series defaults]")
    );
    assert!(
        result
            .summary
            .contains("outputs.kindle.target = kindle-ja [series defaults]")
    );
    assert!(result.summary.contains("shared search paths:"));
    assert!(result.summary.contains("assets = shared/assets"));
}
