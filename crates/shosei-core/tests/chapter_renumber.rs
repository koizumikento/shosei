use std::fs;

use shosei_core::{app, cli_api::CommandContext, config, domain::RepoPath};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shosei-chapter-renumber-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_book(root: &std::path::Path, book_yml: &str) {
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::write(root.join("book.yml"), book_yml).unwrap();
}

#[test]
fn renumber_swaps_prefixes_to_match_chapter_order() {
    let root = temp_dir("swap-order");
    write_book(
        &root,
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
    - manuscript/02-chapter-two.md
    - manuscript/01-chapter-one.md
sections:
  - file: manuscript/01-chapter-one.md
    type: chapter
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
git:
  lfs: true
"#,
    );
    fs::write(root.join("manuscript/01-chapter-one.md"), "# Chapter One\n").unwrap();
    fs::write(root.join("manuscript/02-chapter-two.md"), "# Chapter Two\n").unwrap();

    let result = app::chapter_renumber(
        &CommandContext::new(&root, None, None),
        app::ChapterRenumberOptions {
            start_at: 1,
            width: 2,
            dry_run: false,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("renamed manuscript/02-chapter-two.md -> manuscript/01-chapter-two.md")
    );
    assert!(root.join("manuscript/01-chapter-two.md").is_file());
    assert!(root.join("manuscript/02-chapter-one.md").is_file());
    let resolved =
        config::resolve_book_config(&shosei_core::repo::discover(&root, None).unwrap()).unwrap();
    assert_eq!(
        resolved
            .effective
            .manuscript
            .unwrap()
            .chapters
            .iter()
            .map(RepoPath::as_str)
            .collect::<Vec<_>>(),
        vec![
            "manuscript/01-chapter-two.md",
            "manuscript/02-chapter-one.md"
        ]
    );
    let book_yml = fs::read_to_string(root.join("book.yml")).unwrap();
    assert!(book_yml.contains("file: manuscript/02-chapter-one.md"));
}

#[test]
fn renumber_dry_run_keeps_files_and_config_unchanged() {
    let root = temp_dir("dry-run");
    write_book(
        &root,
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
    - manuscript/chapter-a.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
git:
  lfs: true
"#,
    );
    fs::write(root.join("manuscript/chapter-a.md"), "# Chapter A\n").unwrap();

    let before = fs::read_to_string(root.join("book.yml")).unwrap();
    let result = app::chapter_renumber(
        &CommandContext::new(&root, None, None),
        app::ChapterRenumberOptions {
            start_at: 1,
            width: 2,
            dry_run: true,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("would rename manuscript/chapter-a.md -> manuscript/01-chapter-a.md")
    );
    assert!(root.join("manuscript/chapter-a.md").is_file());
    assert_eq!(fs::read_to_string(root.join("book.yml")).unwrap(), before);
}

#[test]
fn renumber_rejects_conflicting_target_files() {
    let root = temp_dir("conflict");
    write_book(
        &root,
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
    - manuscript/intro.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
git:
  lfs: true
"#,
    );
    fs::write(root.join("manuscript/intro.md"), "# Intro\n").unwrap();
    fs::write(root.join("manuscript/01-intro.md"), "# Existing\n").unwrap();

    let error = app::chapter_renumber(
        &CommandContext::new(&root, None, None),
        app::ChapterRenumberOptions {
            start_at: 1,
            width: 2,
            dry_run: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ChapterError::ChapterRenameConflict { .. }
    ));
}
