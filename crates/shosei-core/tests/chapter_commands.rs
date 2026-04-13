use std::fs;

use shosei_core::{app, cli_api::CommandContext, config, domain::RepoPath, repo::RepoError};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shosei-chapter-commands-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_single_book(root: &std::path::Path) {
    fs::create_dir_all(root.join("manuscript")).unwrap();
    fs::write(root.join("manuscript/01.md"), "# Chapter 1\n").unwrap();
    fs::write(root.join("manuscript/02.md"), "# Chapter 2\n").unwrap();
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
    - manuscript/02.md
sections:
  - file: manuscript/02.md
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
    )
    .unwrap();
}

fn write_series_book(root: &std::path::Path) {
    fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
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
validation:
  strict: true
books:
  - id: vol-01
    path: books/vol-01
"#,
    )
    .unwrap();
    fs::write(root.join("books/vol-01/manuscript/01.md"), "# Chapter 1\n").unwrap();
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
}

fn write_manga_book(root: &std::path::Path) {
    fs::create_dir_all(root.join("manga/pages")).unwrap();
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: manga
  vcs: git
book:
  title: "Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: monochrome
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
"#,
    )
    .unwrap();
}

#[test]
fn chapter_add_appends_and_creates_stub_file() {
    let root = temp_dir("add-single");
    write_single_book(&root);

    let result = app::chapter_add(
        &CommandContext::new(&root, None, None),
        app::ChapterAddOptions {
            chapter_path: "manuscript/03.md".to_string(),
            title: Some("Chapter 3".to_string()),
            before: None,
            after: Some("manuscript/02.md".to_string()),
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("inserted manuscript/03.md at position 3")
    );
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
        vec!["manuscript/01.md", "manuscript/02.md", "manuscript/03.md"]
    );
    assert_eq!(
        fs::read_to_string(root.join("manuscript/03.md")).unwrap(),
        "# Chapter 3\n"
    );
}

#[test]
fn chapter_move_reorders_chapters() {
    let root = temp_dir("move-single");
    write_single_book(&root);

    let result = app::chapter_move(
        &CommandContext::new(&root, None, None),
        app::ChapterMoveOptions {
            chapter_path: "manuscript/02.md".to_string(),
            before: Some("manuscript/01.md".to_string()),
            after: None,
        },
    )
    .unwrap();

    assert!(
        result
            .summary
            .contains("moved manuscript/02.md to position 1")
    );
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
        vec!["manuscript/02.md", "manuscript/01.md"]
    );
}

#[test]
fn chapter_remove_prunes_sections_and_keeps_file_by_default() {
    let root = temp_dir("remove-single");
    write_single_book(&root);

    let result = app::chapter_remove(
        &CommandContext::new(&root, None, None),
        app::ChapterRemoveOptions {
            chapter_path: "manuscript/02.md".to_string(),
            delete_file: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("removed manuscript/02.md"));
    assert!(root.join("manuscript/02.md").is_file());
    let book_yml = fs::read_to_string(root.join("book.yml")).unwrap();
    assert!(!book_yml.contains("file: manuscript/02.md"));
}

#[test]
fn chapter_remove_rejects_last_remaining_chapter() {
    let root = temp_dir("remove-last");
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

    let error = app::chapter_remove(
        &CommandContext::new(&root, None, None),
        app::ChapterRemoveOptions {
            chapter_path: "manuscript/01.md".to_string(),
            delete_file: false,
        },
    )
    .unwrap_err();

    assert!(matches!(error, app::ChapterError::CannotRemoveLastChapter));
}

#[test]
fn chapter_add_requires_book_selection_at_series_root() {
    let root = temp_dir("series-root-selection");
    write_series_book(&root);

    let error = app::chapter_add(
        &CommandContext::new(&root, None, None),
        app::ChapterAddOptions {
            chapter_path: "books/vol-01/manuscript/02.md".to_string(),
            title: Some("Chapter 2".to_string()),
            before: None,
            after: None,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ChapterError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn chapter_add_works_for_series_book() {
    let root = temp_dir("series-book-add");
    write_series_book(&root);

    let result = app::chapter_add(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::ChapterAddOptions {
            chapter_path: "books/vol-01/manuscript/02.md".to_string(),
            title: Some("Chapter 2".to_string()),
            before: None,
            after: Some("books/vol-01/manuscript/01.md".to_string()),
        },
    )
    .unwrap();

    assert!(result.summary.contains("chapter add: vol-01"));
    let context = shosei_core::repo::discover(&root, Some("vol-01")).unwrap();
    let resolved = config::resolve_book_config(&context).unwrap();
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
            "books/vol-01/manuscript/01.md",
            "books/vol-01/manuscript/02.md"
        ]
    );
}

#[test]
fn chapter_commands_reject_manga_projects() {
    let root = temp_dir("reject-manga");
    write_manga_book(&root);

    let error = app::chapter_add(
        &CommandContext::new(&root, None, None),
        app::ChapterAddOptions {
            chapter_path: "manuscript/01.md".to_string(),
            title: Some("Chapter 1".to_string()),
            before: None,
            after: None,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::ChapterError::UnsupportedProjectType {
            project_type: shosei_core::domain::ProjectType::Manga
        }
    ));
}
