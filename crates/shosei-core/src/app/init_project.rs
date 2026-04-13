use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

const SHOSEI_PROJECT_SKILL_TEMPLATE: &str = include_str!("../../templates/shosei-project-skill.md");

#[derive(Debug, Clone)]
pub struct InitProjectOptions {
    pub root: PathBuf,
    pub non_interactive: bool,
    pub force: bool,
    pub config_template: Option<String>,
    pub repo_mode: Option<String>,
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
    #[error("unsupported repo mode `{mode}`")]
    UnsupportedRepoMode { mode: String },
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

impl RepoTemplate {
    fn from_cli(value: Option<&str>, template: ProjectTemplate) -> Result<Self, InitProjectError> {
        match value {
            Some("single-book") => Ok(Self::SingleBook),
            Some("series") => Ok(Self::Series),
            Some(other) => Err(InitProjectError::UnsupportedRepoMode {
                mode: other.to_string(),
            }),
            None => Ok(template.default_repo_mode()),
        }
    }
}

pub fn init_project(options: InitProjectOptions) -> Result<InitProjectResult, InitProjectError> {
    let template = ProjectTemplate::from_cli(options.config_template.as_deref())?;
    let repo_mode = RepoTemplate::from_cli(options.repo_mode.as_deref(), template)?;
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
        write_editorial_scaffold(&root.join("editorial"))?;
    }

    write_file(&root.join("book.yml"), &book_yml(template))?;
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    write_file(&root.join("styles/base.css"), base_css_contents())?;
    write_file(&root.join("styles/epub.css"), "/* EPUB styles */\n")?;
    write_file(&root.join("styles/print.css"), "/* Print styles */\n")?;
    write_agent_skill_template(root, template, RepoTemplate::SingleBook)?;
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
        write_editorial_scaffold(&root.join("books/vol-01/editorial"))?;
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
    write_agent_skill_template(root, template, RepoTemplate::Series)?;
    Ok(())
}

fn book_yml(template: ProjectTemplate) -> String {
    let manuscript_block = if template == ProjectTemplate::Manga {
        String::from(
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n  print:\n    enabled: true\n    target: print-manga\nvalidation:\n  strict: true\n  epubcheck: false\n  accessibility: warn\ngit:\n  lfs: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n",
        )
    } else {
        String::from(
            "manuscript:\n  chapters:\n    - manuscript/01-chapter-1.md\noutputs:\n  kindle:\n    enabled: true\n    target: kindle-ja\nvalidation:\n  strict: true\n  epubcheck: true\n  accessibility: warn\ngit:\n  lfs: true\neditorial:\n  style: editorial/style.yml\n  claims: editorial/claims.yml\n  figures: editorial/figures.yml\n  freshness: editorial/freshness.yml\n",
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
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"Author Name\"\n  language: ja\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\nmanuscript:\n  chapters:\n    - books/vol-01/manuscript/01-chapter-1.md\neditorial:\n  style: books/vol-01/editorial/style.yml\n  claims: books/vol-01/editorial/claims.yml\n  figures: books/vol-01/editorial/figures.yml\n  freshness: books/vol-01/editorial/freshness.yml\n",
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

fn write_editorial_scaffold(root: &Path) -> Result<(), InitProjectError> {
    ensure_dir(root)?;
    write_file(
        &root.join("style.yml"),
        "preferred_terms: []\nbanned_terms: []\n",
    )?;
    write_file(&root.join("claims.yml"), "claims: []\n")?;
    write_file(&root.join("figures.yml"), "figures: []\n")?;
    write_file(&root.join("freshness.yml"), "tracked: []\n")?;
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

fn write_agent_skill_template(
    root: &Path,
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> Result<(), InitProjectError> {
    let skill_dir = root.join(".agents/skills/shosei-project");
    ensure_dir(&skill_dir)?;
    write_file(
        &skill_dir.join("SKILL.md"),
        &agent_skill_contents(template, repo_mode),
    )
}

fn agent_skill_contents(template: ProjectTemplate, repo_mode: RepoTemplate) -> String {
    let replacements = [
        (
            "{{DESCRIPTION}}",
            agent_skill_description(template, repo_mode),
        ),
        ("{{REPO_MODE}}", repo_mode_label(repo_mode)),
        ("{{PROJECT_TYPE}}", template.as_str()),
        ("{{PRIMARY_CONFIG}}", primary_config_note(repo_mode)),
        (
            "{{PRIMARY_CONTENT_PATHS}}",
            primary_content_paths(template, repo_mode),
        ),
        ("{{REPO_MODE_RULES}}", repo_mode_rules(repo_mode)),
        ("{{EXPLAIN_COMMAND}}", explain_command(repo_mode)),
        ("{{VALIDATE_COMMAND}}", validate_command(repo_mode)),
        ("{{PAGE_CHECK_RULE}}", page_check_rule(template, repo_mode)),
        ("{{BUILD_COMMAND}}", build_command(repo_mode)),
        ("{{PREVIEW_COMMAND}}", preview_command(repo_mode)),
        ("{{HANDOFF_COMMAND}}", handoff_command(repo_mode)),
    ];

    let mut rendered = SHOSEI_PROJECT_SKILL_TEMPLATE.to_string();
    for (placeholder, value) in replacements {
        rendered = rendered.replace(placeholder, value);
    }
    rendered
}

fn agent_skill_description(template: ProjectTemplate, repo_mode: RepoTemplate) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "Operate this `shosei` manga series repo. Use when the task is to update `series.yml` or `books/<book-id>/book.yml`, edit `books/<book-id>/manga/` inputs, run `shosei explain --book`, `shosei validate --book`, `shosei page check --book`, `shosei build --book`, `shosei preview --book`, or prepare handoff for a volume."
        }
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "Operate this `shosei` manga repo. Use when the task is to update `book.yml`, edit `manga/` inputs, run `shosei explain`, `shosei validate`, `shosei page check`, `shosei build`, `shosei preview`, or prepare handoff for this book."
        }
        (_, RepoTemplate::Series) => {
            "Operate this `shosei` series publishing repo. Use when the task is to update `series.yml` or `books/<book-id>/book.yml`, edit `books/<book-id>/manuscript/`, `books/<book-id>/editorial/`, or shared assets, run `shosei explain --book`, `shosei validate --book`, `shosei build --book`, `shosei preview --book`, or prepare handoff for a volume."
        }
        (_, RepoTemplate::SingleBook) => {
            "Operate this `shosei` single-book publishing repo. Use when the task is to update `book.yml`, edit `manuscript/`, `editorial/`, or project assets, run `shosei explain`, `shosei validate`, `shosei build`, `shosei preview`, or prepare handoff for this book."
        }
    }
}

fn repo_mode_label(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "single-book",
        RepoTemplate::Series => "series",
    }
}

fn primary_config_note(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "`book.yml`",
        RepoTemplate::Series => "`series.yml` and `books/<book-id>/book.yml`",
    }
}

fn primary_content_paths(template: ProjectTemplate, repo_mode: RepoTemplate) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "`books/<book-id>/manga/`, `shared/styles/`, `shared/assets/`, `shared/fonts/`"
        }
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => "`manga/`, `assets/`, `styles/`",
        (_, RepoTemplate::Series) => {
            "`books/<book-id>/manuscript/`, `books/<book-id>/editorial/`, `shared/styles/`, `shared/assets/`, `shared/fonts/`"
        }
        (_, RepoTemplate::SingleBook) => "`manuscript/`, `editorial/`, `assets/`, `styles/`",
    }
}

fn repo_mode_rules(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => {
            "No `--book` flag is needed; run commands from the repository root unless the task explicitly targets a subdirectory."
        }
        RepoTemplate::Series => {
            "From the repository root, pass `--book <book-id>` to `explain`, `build`, `validate`, `preview`, `page check`, and `handoff`, or run those commands from inside `books/<book-id>/...`."
        }
    }
}

fn explain_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei explain",
        RepoTemplate::Series => "shosei explain --book vol-01",
    }
}

fn validate_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei validate",
        RepoTemplate::Series => "shosei validate --book vol-01",
    }
}

fn page_check_rule(template: ProjectTemplate, repo_mode: RepoTemplate) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "Run `shosei page check` after changing manga page assets, page order, or spread-related settings."
        }
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "Run `shosei page check --book vol-01` after changing manga page assets, page order, or spread-related settings."
        }
        _ => "Skip `page check` unless this repo is using the manga workflow.",
    }
}

fn build_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei build",
        RepoTemplate::Series => "shosei build --book vol-01",
    }
}

fn preview_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei preview",
        RepoTemplate::Series => "shosei preview --book vol-01",
    }
}

fn handoff_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei handoff print",
        RepoTemplate::Series => "shosei handoff print --book vol-01",
    }
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
            repo_mode: None,
        })
        .unwrap();

        assert!(root.join("book.yml").is_file());
        assert!(root.join("manuscript/01-chapter-1.md").is_file());
        assert!(root.join("editorial/style.yml").is_file());
        assert!(root.join("editorial/claims.yml").is_file());
        assert!(root.join("editorial/figures.yml").is_file());
        assert!(root.join("editorial/freshness.yml").is_file());
        assert!(root.join("styles/base.css").is_file());
        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("editorial:\n  style: editorial/style.yml"));
        let skill =
            fs::read_to_string(root.join(".agents/skills/shosei-project/SKILL.md")).unwrap();
        assert!(skill.contains("name: \"shosei-project\""));
        assert!(skill.contains("single-book"));
        assert!(skill.contains("shosei explain"));
        assert!(skill.contains("manuscript/"));
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
            repo_mode: None,
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(root.join("books/vol-01/book.yml").is_file());
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("books/vol-01/manga/pages").is_dir());
        let skill =
            fs::read_to_string(root.join(".agents/skills/shosei-project/SKILL.md")).unwrap();
        assert!(skill.contains("series"));
        assert!(skill.contains("shosei explain --book vol-01"));
        assert!(skill.contains("shosei page check --book vol-01"));
        assert!(skill.contains("books/<book-id>/manga/"));
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
            repo_mode: None,
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
            repo_mode: None,
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
            repo_mode: None,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            InitProjectError::UnsupportedTemplate { .. }
        ));
    }

    #[test]
    fn initializes_series_business_scaffold_with_editorial_book_files() {
        let root = temp_dir("series-business");
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("business".to_string()),
            repo_mode: Some("series".to_string()),
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(root.join("books/vol-01/book.yml").is_file());
        assert!(
            root.join("books/vol-01/manuscript/01-chapter-1.md")
                .is_file()
        );
        assert!(root.join("books/vol-01/editorial/style.yml").is_file());
        assert!(root.join("books/vol-01/editorial/claims.yml").is_file());
        assert!(root.join("books/vol-01/editorial/figures.yml").is_file());
        assert!(root.join("books/vol-01/editorial/freshness.yml").is_file());
        let book = fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap();
        assert!(book.contains("editorial:\n  style: books/vol-01/editorial/style.yml"));
        let skill =
            fs::read_to_string(root.join(".agents/skills/shosei-project/SKILL.md")).unwrap();
        assert!(skill.contains("books/<book-id>/editorial/"));
        assert!(result.summary.contains("series scaffold"));
    }
}
