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

    let error = app::build_book(&CommandContext::new(&root, None, None)).unwrap_err();
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

    let error = app::validate_book(&CommandContext::new(&root, None, None)).unwrap_err();
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

    let error = app::build_book(&CommandContext::new(&root, None, None)).unwrap_err();
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

    let result = app::validate_book(&CommandContext::new(&root, None, None)).unwrap();
    assert!(result.has_errors);
    assert_eq!(result.issue_count, 1);
    assert!(result.report_path.is_file());
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("manuscript file not found"));
    assert!(report.contains("manuscript/01.md"));
}

#[test]
fn build_rejects_disabled_selected_target() {
    let root = temp_dir("build-disabled-target");
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

    let error =
        app::build_book(&CommandContext::new(&root, None, Some("print".to_string()))).unwrap_err();
    assert!(matches!(
        error,
        app::BuildBookError::TargetNotEnabled { target } if target == "print"
    ));
}

#[test]
fn validate_rejects_disabled_selected_target() {
    let root = temp_dir("validate-disabled-target");
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

    let error = app::validate_book(&CommandContext::new(&root, None, Some("print".to_string())))
        .unwrap_err();
    assert!(matches!(
        error,
        app::ValidateBookError::TargetNotEnabled { target } if target == "print"
    ));
}

#[test]
fn explain_shows_single_book_origins() {
    let root = temp_dir("explain-single");
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
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
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("references/entries/market.md"),
        r#"---
title: Market
---

notes
"#,
    )
    .unwrap();

    let result = app::explain_config(&CommandContext::new(&root, None, None)).unwrap();
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
    assert_eq!(result.snapshot.project_type, "novel");
    assert_eq!(result.snapshot.title, "Sample");
    assert_eq!(result.snapshot.outputs, vec!["kindle-ja"]);
    assert_eq!(
        result.snapshot.manuscript.as_ref().unwrap().chapters,
        vec!["manuscript/01.md"]
    );
    assert!(result.summary.contains("reference summary:"));
    assert!(
        result
            .summary
            .contains("- references = 1 entry(s) at references/entries")
    );
    assert!(result.summary.contains("config reference:"));
    assert!(
        result
            .summary
            .contains("https://github.com/koizumikento/shosei/blob/main/docs/config-reference.md")
    );
    assert!(result.snapshot.references.current.initialized);
    assert_eq!(
        result.snapshot.references.current.references_root,
        "references"
    );
    assert_eq!(
        result.snapshot.references.current.entries,
        vec!["references/entries/market.md"]
    );
}

#[test]
fn explain_shows_series_default_origins_and_shared_paths() {
    let root = temp_dir("explain-series");
    fs::create_dir_all(root.join("books/vol-01")).unwrap();
    fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
    fs::write(root.join("books/vol-01/manuscript/01.md"), "# Chapter 1\n").unwrap();
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
    app::reference_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("books/vol-01/references/entries/local.md"),
        "book note\n",
    )
    .unwrap();
    fs::write(
        root.join("shared/metadata/references/entries/shared.md"),
        "shared note\n",
    )
    .unwrap();

    let result = app::explain_config(&CommandContext::new(
        &root,
        Some("vol-01".to_string()),
        None,
    ))
    .unwrap();
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
    assert!(result.summary.contains("config reference:"));
    assert!(result.summary.contains("reference summary:"));
    assert!(
        result
            .summary
            .contains("- book references = 1 entry(s) at books/vol-01/references/entries")
    );
    assert!(
        result
            .summary
            .contains("- shared references = 1 entry(s) at shared/metadata/references/entries")
    );
    assert_eq!(
        result.snapshot.references.current.entries,
        vec!["books/vol-01/references/entries/local.md"]
    );
    assert_eq!(
        result.snapshot.references.shared.as_ref().unwrap().entries,
        vec!["shared/metadata/references/entries/shared.md"]
    );
}

#[test]
fn explain_shows_editorial_summary_when_sidecars_are_configured() {
    let root = temp_dir("explain-editorial");
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::create_dir_all(root.join("editorial")).unwrap();
    fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
    fs::write(
        root.join("editorial/style.yml"),
        r#"
preferred_terms:
  - preferred: "Git"
    aliases:
      - "git"
"#,
    )
    .unwrap();
    fs::write(
        root.join("editorial/claims.yml"),
        r#"
claims:
  - id: claim-1
    summary: "Summary"
    section: manuscript/01.md
    sources:
      - https://example.com/source
"#,
    )
    .unwrap();
    fs::write(
        root.join("editorial/figures.yml"),
        r#"
figures:
  - id: fig-1
    path: assets/images/example.png
    caption: "Example"
    source: "Source"
"#,
    )
    .unwrap();
    fs::write(
        root.join("editorial/freshness.yml"),
        r#"
tracked:
  - kind: claim
    id: claim-1
    last_verified: 2026-04-13
    review_due_on: 2026-05-13
"#,
    )
    .unwrap();
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: business
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
  kindle:
    enabled: true
    target: kindle-ja
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
"#,
    )
    .unwrap();

    let result = app::explain_config(&CommandContext::new(&root, None, None)).unwrap();
    assert!(
        result
            .summary
            .contains("editorial.style = editorial/style.yml [book.yml]")
    );
    assert!(result.summary.contains("editorial summary:"));
    assert!(result.summary.contains("- style rules = 1"));
    assert!(result.summary.contains("- claims = 1"));
    assert!(result.summary.contains("- figures = 1"));
    assert!(result.summary.contains("- freshness items = 1"));
}
