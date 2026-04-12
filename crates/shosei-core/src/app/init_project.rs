use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct InitProjectOptions {
    pub root: PathBuf,
    pub non_interactive: bool,
    pub force: bool,
    pub config_template: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InitProjectResult {
    pub root: PathBuf,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum InitProjectError {
    #[error("unsupported config template `{template}`")]
    UnsupportedTemplate { template: String },
    #[error("refusing to initialize {path}: existing shosei config found")]
    AlreadyInitialized { path: String },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectTemplate {
    Business,
    Novel,
    LightNovel,
    Manga,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoTemplate {
    SingleBook,
    Series,
}

pub fn init_project(options: InitProjectOptions) -> Result<InitProjectResult, InitProjectError> {
    let template = ProjectTemplate::from_cli(options.config_template.as_deref())?;
    let repo_mode = template.default_repo_mode();
    let root = options.root;

    if !options.force && has_existing_config(&root) {
        return Err(InitProjectError::AlreadyInitialized {
            path: root.display().to_string(),
        });
    }

    ensure_dir(&root)?;

    match repo_mode {
        RepoTemplate::SingleBook => init_single_book(&root, template)?,
        RepoTemplate::Series => init_series(&root, template)?,
    }

    let mode_label = match repo_mode {
        RepoTemplate::SingleBook => "single-book",
        RepoTemplate::Series => "series",
    };

    Ok(InitProjectResult {
        summary: format!(
            "initialized {mode_label} scaffold for {} at {}{}",
            template.as_str(),
            root.display(),
            if options.non_interactive {
                " (non-interactive defaults)"
            } else {
                " (interactive wizard pending; defaults applied)"
            }
        ),
        root,
    })
}

impl ProjectTemplate {
    fn from_cli(value: Option<&str>) -> Result<Self, InitProjectError> {
        match value.unwrap_or("novel") {
            "business" => Ok(Self::Business),
            "novel" => Ok(Self::Novel),
            "light-novel" => Ok(Self::LightNovel),
            "manga" => Ok(Self::Manga),
            other => Err(InitProjectError::UnsupportedTemplate {
                template: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Business => "business",
            Self::Novel => "novel",
            Self::LightNovel => "light-novel",
            Self::Manga => "manga",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Business => "Untitled Business Book",
            Self::Novel => "Untitled Novel",
            Self::LightNovel => "Untitled Light Novel",
            Self::Manga => "Untitled Manga Volume",
        }
    }

    fn profile(self) -> &'static str {
        self.as_str()
    }

    fn writing_mode(self) -> &'static str {
        match self {
            Self::Business => "horizontal-ltr",
            Self::Novel | Self::LightNovel | Self::Manga => "vertical-rl",
        }
    }

    fn reading_direction(self) -> &'static str {
        match self.writing_mode() {
            "horizontal-ltr" => "ltr",
            _ => "rtl",
        }
    }

    fn binding(self) -> &'static str {
        match self.writing_mode() {
            "horizontal-ltr" => "left",
            _ => "right",
        }
    }

    fn default_repo_mode(self) -> RepoTemplate {
        match self {
            Self::Manga => RepoTemplate::Series,
            Self::Business | Self::Novel | Self::LightNovel => RepoTemplate::SingleBook,
        }
    }
}

fn init_single_book(root: &Path, template: ProjectTemplate) -> Result<(), InitProjectError> {
    ensure_standard_dirs(root)?;
    if template == ProjectTemplate::Manga {
        ensure_dir(&root.join("manga/script"))?;
        ensure_dir(&root.join("manga/storyboard"))?;
        ensure_dir(&root.join("manga/pages"))?;
        ensure_dir(&root.join("manga/spreads"))?;
        ensure_dir(&root.join("manga/metadata"))?;
    } else {
        ensure_dir(&root.join("manuscript"))?;
        write_file(
            &root.join("manuscript/01-chapter-1.md"),
            "# Chapter 1\n\nWrite here.\n",
        )?;
    }

    write_file(&root.join("book.yml"), &book_yml(template))?;
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    write_file(&root.join("styles/base.css"), base_css_contents())?;
    write_file(&root.join("styles/epub.css"), "/* EPUB styles */\n")?;
    write_file(&root.join("styles/print.css"), "/* Print styles */\n")?;
    Ok(())
}

fn init_series(root: &Path, template: ProjectTemplate) -> Result<(), InitProjectError> {
    ensure_dir(&root.join("shared/assets"))?;
    ensure_dir(&root.join("shared/styles"))?;
    ensure_dir(&root.join("shared/fonts"))?;
    ensure_dir(&root.join("shared/metadata"))?;
    ensure_dir(&root.join("books/vol-01/assets"))?;
    ensure_dir(&root.join("books/vol-01/manuscript"))?;
    ensure_dir(&root.join("books/vol-01/manga/script"))?;
    ensure_dir(&root.join("books/vol-01/manga/storyboard"))?;
    ensure_dir(&root.join("books/vol-01/manga/pages"))?;
    ensure_dir(&root.join("books/vol-01/manga/spreads"))?;
    ensure_dir(&root.join("books/vol-01/manga/metadata"))?;
    ensure_dir(&root.join("dist"))?;

    if template != ProjectTemplate::Manga {
        write_file(
            &root.join("books/vol-01/manuscript/01-chapter-1.md"),
            "# Chapter 1\n\nWrite here.\n",
        )?;
    }

    write_file(&root.join("series.yml"), &series_yml(template))?;
    write_file(
        &root.join("books/vol-01/book.yml"),
        &series_book_yml(template),
    )?;
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    write_file(&root.join("shared/styles/base.css"), base_css_contents())?;
    Ok(())
}

fn book_yml(template: ProjectTemplate) -> String {
    let manuscript_block = if template == ProjectTemplate::Manga {
        String::from(
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n  print:\n    enabled: true\n    target: print-manga\nvalidation:\n  strict: true\n  epubcheck: false\n  accessibility: warn\ngit:\n  lfs: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n",
        )
    } else {
        String::from(
            "manuscript:\n  chapters:\n    - manuscript/01-chapter-1.md\noutputs:\n  kindle:\n    enabled: true\n    target: kindle-ja\nvalidation:\n  strict: true\n  epubcheck: true\n  accessibility: warn\ngit:\n  lfs: true\n",
        )
    };

    format!(
        "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"Author Name\"\n  language: ja\n  profile: {}\n  writing_mode: {}\n  reading_direction: {}\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\n{}",
        template.as_str(),
        template.title(),
        template.profile(),
        template.writing_mode(),
        template.reading_direction(),
        template.binding(),
        manuscript_block
    )
}

fn series_yml(template: ProjectTemplate) -> String {
    let outputs = if template == ProjectTemplate::Manga {
        "  outputs:\n    kindle:\n      enabled: true\n      target: kindle-comic\n    print:\n      enabled: true\n      target: print-manga\n"
    } else {
        "  outputs:\n    kindle:\n      enabled: true\n      target: kindle-ja\n"
    };

    format!(
        "series:\n  id: sample-series\n  title: \"Sample Series\"\n  language: ja\n  type: {}\nshared:\n  assets:\n    - shared/assets\n  styles:\n    - shared/styles\n  fonts:\n    - shared/fonts\n  metadata:\n    - shared/metadata\ndefaults:\n  book:\n    profile: {}\n    writing_mode: {}\n    reading_direction: {}\n  layout:\n    binding: {}\n    chapter_start_page: odd\n    allow_blank_pages: true\n{}validation:\n  strict: true\n  epubcheck: {}\n  accessibility: warn\ngit:\n  lfs: true\n  require_clean_worktree_for_handoff: true\nbooks:\n  - id: vol-01\n    path: books/vol-01\n    number: 1\n    title: \"Volume 1\"\n",
        template.as_str(),
        template.profile(),
        template.writing_mode(),
        template.reading_direction(),
        template.binding(),
        outputs,
        if template == ProjectTemplate::Manga {
            "false"
        } else {
            "true"
        }
    )
}

fn series_book_yml(template: ProjectTemplate) -> String {
    if template == ProjectTemplate::Manga {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"Author Name\"\n  language: ja\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n",
            template.as_str(),
            template.title(),
            template.binding(),
        )
    } else {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"Author Name\"\n  language: ja\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\nmanuscript:\n  chapters:\n    - books/vol-01/manuscript/01-chapter-1.md\n",
            template.as_str(),
            template.title(),
            template.binding(),
        )
    }
}

fn has_existing_config(root: &Path) -> bool {
    root.join("book.yml").exists() || root.join("series.yml").exists()
}

fn ensure_standard_dirs(root: &Path) -> Result<(), InitProjectError> {
    ensure_dir(&root.join("assets/cover"))?;
    ensure_dir(&root.join("assets/images"))?;
    ensure_dir(&root.join("assets/fonts"))?;
    ensure_dir(&root.join("styles"))?;
    ensure_dir(&root.join("dist"))?;
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), InitProjectError> {
    fs::create_dir_all(path).map_err(|source| InitProjectError::CreateDir {
        path: path.display().to_string(),
        source,
    })
}

fn write_file(path: &Path, contents: &str) -> Result<(), InitProjectError> {
    fs::write(path, contents).map_err(|source| InitProjectError::WriteFile {
        path: path.display().to_string(),
        source,
    })
}

fn gitignore_contents() -> &'static str {
    "dist/\ntarget/\n"
}

fn gitattributes_contents() -> &'static str {
    "*.psd filter=lfs diff=lfs merge=lfs -text lockable\n*.clip filter=lfs diff=lfs merge=lfs -text lockable\n*.kra filter=lfs diff=lfs merge=lfs -text lockable\n*.tif filter=lfs diff=lfs merge=lfs -text lockable\n"
}

fn base_css_contents() -> &'static str {
    "body {\n  font-family: sans-serif;\n  line-height: 1.8;\n}\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shosei-init-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn initializes_single_book_novel_scaffold() {
        let root = temp_dir("single");
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("novel".to_string()),
        })
        .unwrap();

        assert!(root.join("book.yml").is_file());
        assert!(root.join("manuscript/01-chapter-1.md").is_file());
        assert!(root.join("styles/base.css").is_file());
        assert!(result.summary.contains("single-book scaffold"));
    }

    #[test]
    fn initializes_series_manga_scaffold() {
        let root = temp_dir("series");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("manga".to_string()),
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(root.join("books/vol-01/book.yml").is_file());
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("books/vol-01/manga/pages").is_dir());
    }

    #[test]
    fn rejects_existing_config_without_force() {
        let root = temp_dir("existing");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("book.yml"), "project: {}\n").unwrap();

        let error = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: None,
        })
        .unwrap_err();

        assert!(matches!(error, InitProjectError::AlreadyInitialized { .. }));
    }

    #[test]
    fn supports_force_reinitialization() {
        let root = temp_dir("force");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("book.yml"), "stale: true\n").unwrap();

        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: true,
            config_template: Some("business".to_string()),
        })
        .unwrap();

        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("type: business"));
    }

    #[test]
    fn rejects_unknown_template() {
        let root = temp_dir("bad-template");
        let error = init_project(InitProjectOptions {
            root,
            non_interactive: true,
            force: false,
            config_template: Some("poetry".to_string()),
        })
        .unwrap_err();

        assert!(matches!(
            error,
            InitProjectError::UnsupportedTemplate { .. }
        ));
    }
}
