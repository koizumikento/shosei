use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "shosei-cli-smoke-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn tiny_png() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn write_validate_fixture(root: &Path) {
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::create_dir_all(root.join("assets/cover")).unwrap();
    fs::write(
        root.join("manuscript/01.md"),
        "# Chapter 1\n\n![ ](assets/missing.png)\n\n[See appendix](missing.md)\n",
    )
    .unwrap();
    fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
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
cover:
  ebook_image: assets/cover/front.png
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
  epubcheck: false
  missing_alt: error
  broken_link: warn
git:
  lfs: true
"#,
    )
    .unwrap();
}

fn write_page_check_fixture(root: &Path) {
    fs::create_dir_all(root.join("manga/pages")).unwrap();
    fs::write(root.join("manga/pages/1.png"), tiny_png()).unwrap();
    fs::write(root.join("manga/pages/2.png"), tiny_png()).unwrap();
    fs::write(root.join("manga/pages/10.png"), tiny_png()).unwrap();
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: mixed
"#,
    )
    .unwrap();
}

fn write_reference_fixture(root: &Path) {
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

fn write_series_reference_fixture(root: &Path) {
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

fn write_reference_entry(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_claims(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_handoff_proof_fixture(root: &Path) {
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::create_dir_all(root.join("editorial")).unwrap();
    fs::create_dir_all(root.join("assets/images")).unwrap();
    fs::write(
        root.join("manuscript/01.md"),
        "# Chapter 1\nUse git in the workflow.\n![Figure](../assets/images/example.png)\n",
    )
    .unwrap();
    fs::write(root.join("assets/images/example.png"), tiny_png()).unwrap();
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
    reviewer_note: "double-check the source"
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
    source: "Internal"
    rights: "owned"
    reviewer_note: "replace logo"
"#,
    )
    .unwrap();
    fs::write(
        root.join("editorial/freshness.yml"),
        r#"
tracked:
  - kind: claim
    id: claim-1
    last_verified: 1999-01-01
    review_due_on: 2000-01-01
    note: "refresh before launch"
"#,
    )
    .unwrap();
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: business
  vcs: git
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
validation:
  strict: true
  epubcheck: false
  accessibility: warn
git:
  lfs: true
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
"#,
    )
    .unwrap();
}

#[cfg(unix)]
fn write_fake_pandoc(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let tools_dir = root.join("tools");
    fs::create_dir_all(&tools_dir).unwrap();
    let pandoc = tools_dir.join("pandoc");
    fs::write(
        &pandoc,
        r#"#!/bin/sh
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf 'fake epub' > "$out"
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&pandoc, permissions).unwrap();
    tools_dir
}

#[cfg(windows)]
fn write_fake_pandoc(root: &Path) -> PathBuf {
    let tools_dir = root.join("tools");
    fs::create_dir_all(&tools_dir).unwrap();
    let pandoc = tools_dir.join("pandoc.cmd");
    fs::write(
        &pandoc,
        "@echo off\r\nsetlocal enabledelayedexpansion\r\nset \"out=\"\r\nset \"prev=\"\r\n:loop\r\nif \"%~1\"==\"\" goto done\r\nif \"!prev!\"==\"--output\" set \"out=%~1\"\r\nset \"prev=%~1\"\r\nshift\r\ngoto loop\r\n:done\r\nif \"%out%\"==\"\" exit /b 1\r\nfor %%I in (\"%out%\") do if not exist \"%%~dpI\" mkdir \"%%~dpI\" >nul 2>&1\r\n> \"%out%\" echo fake epub\r\n",
    )
    .unwrap();
    tools_dir
}

fn prepend_path(path: &Path) -> std::ffi::OsString {
    let current = env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![path.to_path_buf()];
    paths.extend(env::split_paths(&current));
    env::join_paths(paths).unwrap()
}

#[test]
fn validate_cli_prints_issue_preview() {
    let root = temp_dir("validate-preview");
    write_validate_fixture(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["validate", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validation completed for default"));
    assert!(stdout.contains("issues:"));
    assert!(stdout.contains("[error] image is missing alt text: assets/missing.png"));
    assert!(stdout.contains("[warn] link target not found: missing.md"));
    assert!(stdout.contains(&format!(
        "location: {}:3",
        root.join("manuscript/01.md").display()
    )));
    assert!(stdout.contains("remedy: 画像参照に代替テキストを追加してください。"));
}

#[test]
fn page_check_cli_prints_summary_and_issue_preview() {
    let root = temp_dir("page-check-preview");
    write_page_check_fixture(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["page", "check", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("page check completed for default"));
    assert!(stdout.contains("page order: 1.png, 10.png, 2.png"));
    assert!(stdout.contains("spread candidates: none"));
    assert!(stdout.contains("issues:"));
    assert!(stdout.contains("[warn] lexicographic page order differs from numeric order"));
    assert!(stdout.contains(&format!(
        "location: {}",
        root.join("manga").join("pages").display()
    )));
    assert!(stdout.contains(
        "remedy: ページ順はファイル名の辞書順で決まります。ゼロ埋めした連番へ揃えてください。"
    ));
}

#[test]
fn reference_scaffold_cli_creates_workspace() {
    let root = temp_dir("reference-scaffold");
    write_reference_fixture(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "scaffold", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference scaffold: initialized single-book reference workspace"));
    assert!(root.join("references/README.md").is_file());
    assert!(root.join("references/entries/README.md").is_file());
}

#[test]
fn reference_map_cli_prints_summary_and_writes_report() {
    let root = temp_dir("reference-map");
    write_reference_fixture(&root);

    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "scaffold", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    write_reference_entry(
        &root,
        "references/entries/source.md",
        r#"---
title: Source Note
links:
  - https://example.com/source
status: summarized
---
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "map", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference map: 1 entry(s)"));
    assert!(stdout.contains("source"));
    assert!(
        root.join("dist/reports/default-reference-map.json")
            .is_file()
    );
}

#[test]
fn reference_map_cli_suggests_scaffold_when_workspace_is_missing() {
    let root = temp_dir("reference-map-missing-workspace");
    write_reference_fixture(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "map", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("reference entries directory not found"));
    assert!(stderr.contains("run `shosei reference scaffold` first"));
}

#[test]
fn reference_check_cli_prints_issue_preview_and_fails_on_errors() {
    let root = temp_dir("reference-check");
    write_reference_fixture(&root);

    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "scaffold", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    write_reference_entry(
        &root,
        "references/entries/source-a.md",
        r#"---
id: duplicate-source
links:
  - missing.md
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

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "check", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference check completed for default"));
    assert!(stdout.contains("issues:"));
    assert!(stdout.contains("[warn] reference link target not found: missing.md"));
    assert!(stdout.contains("[error] duplicate reference id `duplicate-source`"));
    assert!(stdout.contains(&format!(
        "location: {}",
        root.join("references/entries/source-b.md").display()
    )));
    assert!(
        root.join("dist/reports/default-reference-check.json")
            .is_file()
    );
}

#[test]
fn reference_check_cli_reports_missing_claim_reference_source() {
    let root = temp_dir("reference-check-claims");
    write_reference_fixture(&root);
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

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "scaffold", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["reference", "check", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference check completed for default"));
    assert!(stdout.contains("issues:"));
    assert!(stdout.contains("claim `claim-market` references missing source `ref:missing`"));
    assert!(stdout.contains(&format!(
        "location: {}",
        root.join("editorial/claims.yml").display()
    )));
}

#[test]
fn reference_drift_cli_writes_report_and_fails_on_drift() {
    let root = temp_dir("reference-drift");
    write_series_reference_fixture(&root);

    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--shared",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--book",
            "vol-01",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
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

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "drift",
            "--book",
            "vol-01",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference drift completed for vol-01"));
    assert!(
        root.join("dist/reports/vol-01-reference-drift.json")
            .is_file()
    );
}

#[test]
fn reference_sync_cli_copies_shared_entry_into_book_scope() {
    let root = temp_dir("reference-sync");
    write_series_reference_fixture(&root);

    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--shared",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--book",
            "vol-01",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "sync",
            "--book",
            "vol-01",
            "--from",
            "shared",
            "--id",
            "market",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference sync: copied shared reference `market`"));
    assert_eq!(
        fs::read_to_string(root.join("books/vol-01/references/entries/market.md")).unwrap(),
        "---\nid: market\n---\nshared note\n"
    );
}

#[test]
fn reference_sync_cli_report_applies_shared_gap_and_skips_book_only_gap() {
    let root = temp_dir("reference-sync-report");
    let report_path = root.join("dist/reports/vol-01-reference-drift.json");
    write_series_reference_fixture(&root);

    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--shared",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "scaffold",
            "--book",
            "vol-01",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    write_reference_entry(
        &root,
        "shared/metadata/references/entries/market.md",
        "---\nid: market\n---\nshared note\n",
    );
    write_reference_entry(
        &root,
        "books/vol-01/references/entries/local.md",
        "---\nid: local\n---\nbook note\n",
    );

    let drift = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "drift",
            "--book",
            "vol-01",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(drift.status.success());
    assert!(report_path.is_file());

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args([
            "reference",
            "sync",
            "--book",
            "vol-01",
            "--from",
            "shared",
            "--report",
            report_path.to_str().unwrap(),
            "--force",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reference sync: applied 1 applicable report entries"));
    assert!(stdout.contains("skipped: 1"));
    assert_eq!(
        fs::read_to_string(root.join("books/vol-01/references/entries/market.md")).unwrap(),
        "---\nid: market\n---\nshared note\n"
    );
    assert!(
        !root
            .join("shared/metadata/references/entries/local.md")
            .exists()
    );
}

#[cfg(any(unix, windows))]
#[test]
fn handoff_proof_cli_packages_review_packet() {
    let root = temp_dir("handoff-proof");
    write_handoff_proof_fixture(&root);
    let tools_dir = write_fake_pandoc(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_shosei"))
        .args(["handoff", "proof", "--path", root.to_str().unwrap()])
        .env("PATH", prepend_path(&tools_dir))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("handoff packaged for default (proof)"));
    assert!(stdout.contains("validation issues: "));

    let package_dir = root.join("dist/handoff/default-proof");
    assert!(
        package_dir
            .join("artifacts/default-kindle-ja.epub")
            .is_file()
    );
    assert!(package_dir.join("reports/validate.json").is_file());
    assert!(package_dir.join("reports/review-packet.json").is_file());
    assert!(package_dir.join("review-notes.md").is_file());
    assert!(package_dir.join("editorial/style.yml").is_file());
    assert!(package_dir.join("editorial/claims.yml").is_file());
    assert!(package_dir.join("editorial/figures.yml").is_file());
    assert!(package_dir.join("editorial/freshness.yml").is_file());

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(package_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["review_packet"], "reports/review-packet.json");
    assert_eq!(manifest["review_notes"], "review-notes.md");
    assert_eq!(manifest["editorial_summary"]["claim_count"], 1);
    assert_eq!(manifest["editorial_summary"]["figure_count"], 1);

    let review_packet = fs::read_to_string(package_dir.join("reports/review-packet.json")).unwrap();
    assert!(review_packet.contains("\"book_id\": \"default\""));
    assert!(review_packet.contains("\"issue_summary\""));
    assert!(review_packet.contains("\"reviewer_notes\""));
    assert!(review_packet.contains("\"claim-1\""));
    assert!(review_packet.contains("\"fig-1\""));

    let review_notes = fs::read_to_string(package_dir.join("review-notes.md")).unwrap();
    assert!(review_notes.contains("double-check the source"));
    assert!(review_notes.contains("replace logo"));
    assert!(review_notes.contains("refresh before launch"));
}
