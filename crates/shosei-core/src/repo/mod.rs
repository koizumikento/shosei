use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::domain::{BookContext, RepoContext, RepoMode};

const BOOK_CONFIG: &str = "book.yml";
const SERIES_CONFIG: &str = "series.yml";

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("could not locate book.yml or series.yml from {start}")]
    NotInitialized { start: String },
    #[error("book.yml and series.yml cannot coexist in {path}")]
    ConflictingConfig { path: String },
    #[error("series repository at {repo_root} requires --book or execution under books/<book-id>/")]
    BookSelectionRequired { repo_root: String },
}

pub fn discover(start: &Path, selected_book: Option<&str>) -> Result<RepoContext, RepoError> {
    let start = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    let mut nearest_book_dir: Option<PathBuf> = None;
    let mut current = Some(start);

    while let Some(dir) = current {
        let book_exists = dir.join(BOOK_CONFIG).is_file();
        let series_exists = dir.join(SERIES_CONFIG).is_file();

        if book_exists && series_exists {
            return Err(RepoError::ConflictingConfig {
                path: dir.display().to_string(),
            });
        }

        if book_exists && nearest_book_dir.is_none() {
            nearest_book_dir = Some(dir.to_path_buf());
        }

        if series_exists {
            let repo_root = dir.to_path_buf();
            let book = selected_book
                .map(|book_id| series_book_context(&repo_root, book_id))
                .or_else(|| {
                    nearest_book_dir
                        .as_ref()
                        .and_then(|book_dir| infer_series_book(&repo_root, book_dir))
                });
            return Ok(RepoContext {
                repo_root,
                mode: RepoMode::Series,
                book,
            });
        }

        current = dir.parent();
    }

    if let Some(book_dir) = nearest_book_dir {
        return Ok(RepoContext {
            repo_root: book_dir.clone(),
            mode: RepoMode::SingleBook,
            book: Some(BookContext {
                id: "default".to_string(),
                root: book_dir.clone(),
                config_path: book_dir.join(BOOK_CONFIG),
            }),
        });
    }

    Err(RepoError::NotInitialized {
        start: start.display().to_string(),
    })
}

pub fn require_book_context(context: RepoContext) -> Result<RepoContext, RepoError> {
    if context.mode == RepoMode::Series && context.book.is_none() {
        return Err(RepoError::BookSelectionRequired {
            repo_root: context.repo_root.display().to_string(),
        });
    }
    Ok(context)
}

fn infer_series_book(repo_root: &Path, book_dir: &Path) -> Option<BookContext> {
    let relative = book_dir.strip_prefix(repo_root).ok()?;
    let mut segments = relative.iter();
    match (segments.next(), segments.next(), segments.next()) {
        (Some(books), Some(book_id), None) if books == "books" => Some(BookContext {
            id: book_id.to_string_lossy().into_owned(),
            root: book_dir.to_path_buf(),
            config_path: book_dir.join(BOOK_CONFIG),
        }),
        _ => None,
    }
}

fn series_book_context(repo_root: &Path, book_id: &str) -> BookContext {
    let root = repo_root.join("books").join(book_id);
    BookContext {
        id: book_id.to_string(),
        config_path: root.join(BOOK_CONFIG),
        root,
    }
}
