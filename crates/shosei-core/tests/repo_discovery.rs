use std::fs;

use shosei_core::{
    domain::RepoMode,
    repo::{self, RepoError},
};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("shosei-core-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn discovers_single_book_from_nested_directory() {
    let root = temp_dir("single-book");
    fs::write(root.join("book.yml"), "project: {}\n").unwrap();
    fs::create_dir_all(root.join("manuscript/ch1")).unwrap();

    let context = repo::discover(&root.join("manuscript/ch1"), None).unwrap();
    assert_eq!(context.mode, RepoMode::SingleBook);
    assert_eq!(context.repo_root, root);
    assert_eq!(context.book.unwrap().id, "default");
}

#[test]
fn discovers_series_book_from_nested_directory() {
    let root = temp_dir("series");
    fs::write(root.join("series.yml"), "series: {}\n").unwrap();
    fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
    fs::write(root.join("books/vol-01/book.yml"), "project: {}\n").unwrap();

    let context = repo::discover(&root.join("books/vol-01/manuscript"), None).unwrap();
    assert_eq!(context.mode, RepoMode::Series);
    let book = context.book.unwrap();
    assert_eq!(book.id, "vol-01");
    assert_eq!(book.root, root.join("books/vol-01"));
}

#[test]
fn requires_book_selection_at_series_root() {
    let root = temp_dir("series-root");
    fs::write(root.join("series.yml"), "series: {}\n").unwrap();

    let context = repo::discover(&root, None).unwrap();
    let error = repo::require_book_context(context).unwrap_err();
    assert!(matches!(error, RepoError::BookSelectionRequired { .. }));
}
