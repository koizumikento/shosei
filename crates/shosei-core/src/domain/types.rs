use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Business,
    Paper,
    Novel,
    LightNovel,
    Manga,
}

impl ProjectType {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "business" => Some(Self::Business),
            "paper" => Some(Self::Paper),
            "novel" => Some(Self::Novel),
            "light-novel" => Some(Self::LightNovel),
            "manga" => Some(Self::Manga),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Business => "business",
            Self::Paper => "paper",
            Self::Novel => "novel",
            Self::LightNovel => "light-novel",
            Self::Manga => "manga",
        }
    }

    pub fn is_prose(self) -> bool {
        !matches!(self, Self::Manga)
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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
