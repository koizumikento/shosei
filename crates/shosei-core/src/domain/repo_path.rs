use std::path::{Component, Path};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoPath(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RepoPathError {
    #[error("path must not be empty")]
    Empty,
    #[error("path must use repo-relative '/' separators")]
    NotRepoRelative,
    #[error("path must not contain '..' segments")]
    Traversal,
}

impl RepoPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, RepoPathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(RepoPathError::Empty);
        }
        let path = Path::new(&value);
        let has_windows_drive_prefix = value.as_bytes().get(1) == Some(&b':')
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic);
        if value.starts_with('/')
            || value.starts_with("./")
            || value.contains('\\')
            || has_windows_drive_prefix
            || path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
        {
            return Err(RepoPathError::NotRepoRelative);
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(RepoPathError::Traversal);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RepoPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_repo_relative_forward_slash_path() {
        let path = RepoPath::parse("books/vol-01/book.yml").unwrap();
        let root = std::env::temp_dir().join("shosei-repo-path-root");

        assert_eq!(path.as_str(), "books/vol-01/book.yml");
        assert!(crate::fs::join_repo_path(&root, &path).starts_with(&root));
    }

    #[test]
    fn rejects_platform_independent_absolute_and_drive_paths() {
        for value in [
            "/books/vol-01/book.yml",
            "C:/books/vol-01/book.yml",
            "C:books/vol-01/book.yml",
            "z:",
            r"\\server\share\book.yml",
        ] {
            assert_eq!(
                RepoPath::parse(value),
                Err(RepoPathError::NotRepoRelative),
                "{value} must not be accepted as a repo-relative path"
            );
        }
    }

    #[test]
    fn rejects_parent_traversal() {
        assert_eq!(
            RepoPath::parse("books/../outside/book.yml"),
            Err(RepoPathError::Traversal)
        );
    }
}
