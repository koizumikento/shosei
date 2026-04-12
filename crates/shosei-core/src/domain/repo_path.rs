use std::path::Path;

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
        if value.starts_with('/') || value.starts_with("./") || value.contains('\\') {
            return Err(RepoPathError::NotRepoRelative);
        }
        if Path::new(&value)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
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
