use std::fs;

use shosei_core::{app, cli_api::CommandContext, repo::RepoError};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shosei-reference-commands-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_single_book(root: &std::path::Path) {
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
"#,
    )
    .unwrap();
}

fn write_series_repo(root: &std::path::Path) {
    fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
    fs::write(root.join("books/vol-01/manuscript/01.md"), "# Chapter 1\n").unwrap();
    fs::write(
        root.join("series.yml"),
        r#"
series:
  id: sample
  title: Sample Series
  type: novel
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
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
"#,
    )
    .unwrap();
}

fn write_reference_entry(root: &std::path::Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_claims(root: &std::path::Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn reference_scaffold_creates_single_book_workspace() {
    let root = temp_dir("single-book");
    write_single_book(&root);

    let result = app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("single-book reference workspace"));
    assert!(root.join("references/README.md").is_file());
    assert!(root.join("references/entries/README.md").is_file());
}

#[test]
fn reference_scaffold_creates_series_book_workspace() {
    let root = temp_dir("series-book");
    write_series_repo(&root);

    let result = app::reference_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("reference workspace for vol-01"));
    assert!(root.join("books/vol-01/references/README.md").is_file());
    assert!(
        root.join("books/vol-01/references/entries/README.md")
            .is_file()
    );
}

#[test]
fn reference_scaffold_creates_shared_series_workspace() {
    let root = temp_dir("series-shared");
    write_series_repo(&root);

    let result = app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("shared series reference workspace"));
    assert!(root.join("shared/metadata/references/README.md").is_file());
    assert!(
        root.join("shared/metadata/references/entries/README.md")
            .is_file()
    );
}

#[test]
fn reference_scaffold_requires_book_for_series_book_scope() {
    let root = temp_dir("series-book-required");
    write_series_repo(&root);

    let error = app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceScaffoldError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn reference_scaffold_rejects_shared_scope_in_single_book_repo() {
    let root = temp_dir("single-shared");
    write_single_book(&root);

    let error = app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceScaffoldError::SharedRequiresSeries
    ));
}

#[test]
fn reference_scaffold_rejects_explicit_book_with_shared_scope() {
    let root = temp_dir("series-conflicting");
    write_series_repo(&root);

    let error = app::reference_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceScaffoldError::ConflictingScope
    ));
}

#[test]
fn reference_map_writes_report_for_single_book_workspace() {
    let root = temp_dir("map-single");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "references/entries/market.md",
        r#"---
id: market-report
title: Market Report
links:
  - https://example.com/report
status: reading
---

notes
"#,
    );
    write_reference_entry(&root, "references/entries/interview.md", "notes only\n");

    let result = app::reference_map(
        &CommandContext::new(&root, None, None),
        app::ReferenceMapOptions { shared: false },
    )
    .unwrap();

    assert!(result.summary.contains("reference map: 2 entry(s)"));
    assert!(result.summary.contains("market-report"));
    assert!(result.summary.contains("interview"));
    assert!(result.report_path.is_file());
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("\"scope\": \"single-book\""));
    assert!(report.contains("\"entry_count\": 2"));
}

#[test]
fn reference_map_reads_shared_series_workspace() {
    let root = temp_dir("map-shared");
    write_series_repo(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/canon.md",
        r#"---
title: Shared Note
links:
  - https://example.com/shared
---
"#,
    );

    let result = app::reference_map(
        &CommandContext::new(&root, None, None),
        app::ReferenceMapOptions { shared: true },
    )
    .unwrap();

    assert!(result.summary.contains("reference map: 1 entry(s)"));
    assert!(result.summary.contains("canon"));
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("\"scope\": \"shared-series\""));
}

#[test]
fn reference_map_requires_book_for_series_book_scope() {
    let root = temp_dir("map-series-book-required");
    write_series_repo(&root);

    let error = app::reference_map(
        &CommandContext::new(&root, None, None),
        app::ReferenceMapOptions { shared: false },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceMapError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn reference_map_reports_invalid_frontmatter() {
    let root = temp_dir("map-invalid-frontmatter");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "references/entries/bad.md",
        r#"---
- invalid
---
"#,
    );

    let error = app::reference_map(
        &CommandContext::new(&root, None, None),
        app::ReferenceMapOptions { shared: false },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceMapError::InvalidEntryFrontmatter { .. }
    ));
}

#[test]
fn reference_check_reports_duplicate_ids_and_path_issues() {
    let root = temp_dir("check-issues");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::write(root.join("manuscript/01.md"), "# Chapter\n").unwrap();
    write_reference_entry(
        &root,
        "references/entries/source-a.md",
        r#"---
id: duplicate-source
links:
  - manuscript/01.md#intro
  - missing.md
related_sections:
  - ../outside.md
---
"#,
    );
    write_reference_entry(
        &root,
        "references/entries/source-b.md",
        r#"---
id: duplicate-source
---
"#,
    );

    let result = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("reference check completed for default")
    );
    assert_eq!(result.issue_count, 3);
    assert!(result.has_errors);
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("\"entry_count\": 2"));
    assert!(report.contains("duplicate reference id `duplicate-source`"));
    assert!(report.contains("reference link target not found: missing.md"));
    assert!(report.contains("invalid related section in `related_sections`"));
}

#[test]
fn reference_check_reads_shared_series_workspace() {
    let root = temp_dir("check-shared");
    write_series_repo(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/canon.md",
        r#"---
title: Shared Note
links:
  - https://example.com/shared
---
"#,
    );

    let result = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: true },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("reference check completed for shared")
    );
    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("\"scope\": \"shared-series\""));
}

#[test]
fn reference_check_requires_book_for_series_book_scope() {
    let root = temp_dir("check-series-book-required");
    write_series_repo(&root);

    let error = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceCheckError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn reference_check_reports_invalid_frontmatter_as_issue() {
    let root = temp_dir("check-invalid-frontmatter");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "references/entries/bad.md",
        r#"---
id: broken
"#,
    );

    let result = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap();

    assert_eq!(result.issue_count, 1);
    assert!(result.has_errors);
    assert!(
        result.issues[0]
            .cause
            .contains("invalid reference entry frontmatter")
    );
}

#[test]
fn reference_check_resolves_claim_ref_sources_in_single_book_scope() {
    let root = temp_dir("check-claim-ref-single");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_reference_entry(
        &root,
        "references/entries/market.md",
        "---\nid: market\n---\nmarket note\n",
    );
    write_claims(
        &root,
        "editorial/claims.yml",
        r#"
claims:
  - id: claim-market
    summary: "Summary"
    section: manuscript/01.md
    sources:
      - "ref:market"
"#,
    );
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
editorial:
  claims: editorial/claims.yml
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

    let result = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
}

#[test]
fn reference_check_resolves_claim_ref_sources_against_shared_entries() {
    let root = temp_dir("check-claim-ref-shared");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );
    write_claims(
        &root,
        "books/vol-01/editorial/claims.yml",
        r#"
claims:
  - id: claim-market
    summary: "Summary"
    section: books/vol-01/manuscript/01.md
    sources:
      - "ref:market"
"#,
    );
    fs::write(
        root.join("books/vol-01/book.yml"),
        r#"
project:
  type: novel
book:
  title: "Vol 1"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
editorial:
  claims: books/vol-01/editorial/claims.yml
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
"#,
    )
    .unwrap();

    let result = app::reference_check(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
}

#[test]
fn reference_check_reports_missing_claim_ref_source() {
    let root = temp_dir("check-claim-ref-missing");
    write_single_book(&root);
    app::reference_scaffold(
        &CommandContext::new(&root, None, None),
        app::ReferenceScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_claims(
        &root,
        "editorial/claims.yml",
        r#"
claims:
  - id: claim-market
    summary: "Summary"
    section: manuscript/01.md
    sources:
      - "ref:missing"
      - "ref:"
"#,
    );
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
editorial:
  claims: editorial/claims.yml
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

    let result = app::reference_check(
        &CommandContext::new(&root, None, None),
        app::ReferenceCheckOptions { shared: false },
    )
    .unwrap();

    assert_eq!(result.issue_count, 2);
    assert!(result.has_errors);
    assert!(result.issues.iter().any(|issue| {
        issue
            .cause
            .contains("references missing source `ref:missing`")
    }));
    assert!(
        result
            .issues
            .iter()
            .any(|issue| issue.cause.contains("has empty reference source `ref:`"))
    );
}

#[test]
fn reference_drift_reports_shared_reference_drift() {
    let root = temp_dir("drift-diverged");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/market.md",
        "---\nid: market\n---\nbook note\n",
    );

    let result = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("reference drift completed for vol-01")
    );
    assert!(result.has_errors);
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("shared reference drift for `market`"));
    assert!(report.contains("\"drifts\""));
    assert!(report.contains("\"id\": \"market\""));
    assert!(report.contains("\"status\": \"drift\""));
    assert!(report.contains("shared/metadata/references/entries/market.md"));
    assert!(report.contains("books/vol-01/references/entries/market.md"));
}

#[test]
fn reference_drift_warns_for_redundant_shared_and_book_copies() {
    let root = temp_dir("drift-redundant");
    write_series_repo(&root);
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
    let contents = "---\nid: market\nlinks:\n  - https://example.com/report\n---\nshared note\n";
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        contents,
    );
    write_reference_entry(&root, "books/vol-01/references/entries/market.md", contents);

    let result = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 1);
    assert!(!result.has_errors);
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("redundant shared/book reference copy for `market`"));
}

#[test]
fn reference_drift_reports_shared_only_and_book_only_gaps() {
    let root = temp_dir("drift-gaps");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/shared-only.md",
        "---\nid: shared-only\n---\nshared note\n",
    );
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/book-only.md",
        "---\nid: book-only\n---\nbook note\n",
    );

    let result = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 2);
    assert!(!result.has_errors);
    let report = fs::read_to_string(result.report_path).unwrap();
    assert!(report.contains("\"gaps\""));
    assert!(report.contains("shared-only reference gap for `shared-only`"));
    assert!(report.contains("book-only reference gap for `book-only`"));
    assert!(report.contains("\"status\": \"shared-only\""));
    assert!(report.contains("\"status\": \"book-only\""));
}

#[test]
fn reference_drift_is_series_only() {
    let root = temp_dir("drift-series-only");
    write_single_book(&root);

    let error = app::reference_drift(
        &CommandContext::new(&root, None, None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap_err();

    assert!(matches!(error, app::ReferenceDriftError::SeriesOnly));
}

#[test]
fn reference_sync_copies_missing_shared_entry_into_book_workspace() {
    let root = temp_dir("sync-copy-shared");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );

    let result = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            id: Some("market".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("reference sync: copied shared reference `market`")
    );
    assert!(result.changed);
    assert_eq!(result.changed_count, 1);
    assert_eq!(
        fs::read_to_string(root.join("books/vol-01/references/entries/market.md")).unwrap(),
        "---\nid: market\n---\nshared note\n"
    );
}

#[test]
fn reference_sync_to_shared_requires_force_to_overwrite_diverged_copy() {
    let root = temp_dir("sync-to-shared-conflict");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/market.md",
        "---\nid: market\n---\nbook note\n",
    );
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );

    let error = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            id: Some("market".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceSyncError::SharedEntryConflict { .. }
    ));
}

#[test]
fn reference_sync_report_applies_shared_to_book_batch() {
    let root = temp_dir("sync-report");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/market.md",
        "---\nid: market\n---\nbook note\n",
    );
    let drift = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    let result = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            id: None,
            report: Some(drift.report_path),
            force: true,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("reference sync: applied 1 applicable report entries")
    );
    assert!(result.changed);
    assert_eq!(
        fs::read_to_string(root.join("books/vol-01/references/entries/market.md")).unwrap(),
        "---\nid: market\n---\nshared note\n"
    );
}

#[test]
fn reference_sync_report_applies_shared_only_gap_and_skips_book_only_gap() {
    let root = temp_dir("sync-report-shared-gap");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/shared-only.md",
        "---\nid: shared-only\n---\nshared note\n",
    );
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/book-only.md",
        "---\nid: book-only\n---\nbook note\n",
    );
    let drift = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    let result = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            id: None,
            report: Some(drift.report_path),
            force: true,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("applied 1 applicable report entries")
    );
    assert!(result.summary.contains("skipped: 1"));
    assert_eq!(result.changed_count, 1);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(
        fs::read_to_string(root.join("books/vol-01/references/entries/shared-only.md")).unwrap(),
        "---\nid: shared-only\n---\nshared note\n"
    );
    assert!(
        !root
            .join("shared/metadata/references/entries/book-only.md")
            .exists()
    );
}

#[test]
fn reference_sync_report_applies_book_only_gap_to_shared_workspace() {
    let root = temp_dir("sync-report-book-gap");
    write_series_repo(&root);
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
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/book-only.md",
        "---\nid: book-only\n---\nbook note\n",
    );
    let drift = app::reference_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceDriftOptions::default(),
    )
    .unwrap();

    let result = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            id: None,
            report: Some(drift.report_path),
            force: true,
        },
    )
    .unwrap();

    assert_eq!(result.changed_count, 1);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(
        fs::read_to_string(root.join("shared/metadata/references/entries/book-only.md")).unwrap(),
        "---\nid: book-only\n---\nbook note\n"
    );
}

#[test]
fn reference_sync_report_requires_force() {
    let root = temp_dir("sync-report-force");
    write_series_repo(&root);

    let error = app::reference_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ReferenceSyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            id: None,
            report: Some(root.join("dist/reports/vol-01-reference-drift.json")),
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ReferenceSyncError::ReportSyncRequiresForce
    ));
}
