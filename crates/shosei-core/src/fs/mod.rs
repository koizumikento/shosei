use std::path::{Path, PathBuf};

use crate::domain::RepoPath;

pub fn join_repo_path(repo_root: &Path, repo_path: &RepoPath) -> PathBuf {
    repo_root.join(repo_path.as_str())
}
