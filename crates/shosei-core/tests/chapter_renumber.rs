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

fn assert_no_staging_directories(root: &std::path::Path) {
    fn visit(path: &std::path::Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            assert!(
                !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".shosei-renumber-"),
                "staging directory remained at {}",
                entry.path().display()
            );
            visit(&entry.path());
        }
    }
    visit(root);
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
    assert_no_staging_directories(&root);
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

#[test]
fn renumber_preserves_preexisting_legacy_temporary_file() {
    let root = temp_dir("legacy-temp-collision");
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
    let legacy_temporary_path = root.join("manuscript/intro.md.shosei-renumber-0.tmp");
    fs::write(&legacy_temporary_path, "do not overwrite\n").unwrap();

    app::chapter_renumber(
        &CommandContext::new(&root, None, None),
        app::ChapterRenumberOptions {
            start_at: 1,
            width: 2,
            dry_run: false,
        },
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(root.join("manuscript/01-intro.md")).unwrap(),
        "# Intro\n"
    );
    assert_eq!(
        fs::read_to_string(legacy_temporary_path).unwrap(),
        "do not overwrite\n"
    );
    assert_no_staging_directories(&root);
}

#[cfg(unix)]
#[test]
fn renumber_updates_internal_symlink_target_without_replacing_config_symlink() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("internal-config-symlink");
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
    fs::create_dir(root.join("config")).unwrap();
    let config_target = root.join("config/book-source.yml");
    fs::rename(root.join("book.yml"), &config_target).unwrap();
    symlink("config/book-source.yml", root.join("book.yml")).unwrap();

    app::chapter_renumber(
        &CommandContext::new(&root, None, None),
        app::ChapterRenumberOptions {
            start_at: 1,
            width: 2,
            dry_run: false,
        },
    )
    .unwrap();

    assert!(
        fs::symlink_metadata(root.join("book.yml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&config_target)
            .unwrap()
            .contains("manuscript/01-intro.md")
    );
    assert!(!root.join("manuscript/intro.md").exists());
    assert!(root.join("manuscript/01-intro.md").is_file());
    assert_no_staging_directories(&root);
}

#[cfg(unix)]
#[test]
fn renumber_rejects_config_symlink_target_outside_repository_before_mutation() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("external-config-symlink");
    let external = temp_dir("external-config-target");
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
    let config_target = external.join("book-source.yml");
    fs::rename(root.join("book.yml"), &config_target).unwrap();
    symlink(&config_target, root.join("book.yml")).unwrap();
    let original_config = fs::read(&config_target).unwrap();

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
        app::ChapterError::RenumberConfigOutsideRepository { .. }
    ));
    assert_eq!(fs::read(&config_target).unwrap(), original_config);
    assert!(
        fs::symlink_metadata(root.join("book.yml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(root.join("manuscript/intro.md").is_file());
    assert!(!root.join("manuscript/01-intro.md").exists());
    assert_no_staging_directories(&root);
    assert_no_staging_directories(&external);
}
