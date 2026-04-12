use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Business,
    Novel,
    LightNovel,
    Manga,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoMode {
    SingleBook,
    Series,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookContext {
    pub id: String,
    pub root: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContext {
    pub repo_root: PathBuf,
    pub mode: RepoMode,
    pub book: Option<BookContext>,
}
