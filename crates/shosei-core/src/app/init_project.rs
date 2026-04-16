use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

const SHOSEI_PROJECT_SKILL_TEMPLATE: &str = include_str!("../../templates/shosei-project-skill.md");
const SHOSEI_CONTENT_REVIEW_SKILL_TEMPLATE: &str =
    include_str!("../../templates/shosei-content-review.md");

#[derive(Debug, Clone)]
pub struct InitProjectOptions {
    pub root: PathBuf,
    pub non_interactive: bool,
    pub force: bool,
    pub config_template: Option<String>,
    pub config_profile: Option<String>,
    pub repo_mode: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub output_preset: Option<String>,
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
    #[error("unsupported config profile `{profile}`")]
    UnsupportedProfile { profile: String },
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
    Paper,
    Novel,
    LightNovel,
    Manga,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectProfile {
    Business,
    Paper,
    ConferencePreprint,
    Novel,
    LightNovel,
    Manga,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoTemplate {
    SingleBook,
    Series,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputPreset {
    Kindle,
    Print,
    Both,
}

#[derive(Debug, Clone)]
struct InitScaffoldConfig {
    template: ProjectTemplate,
    profile: ProjectProfile,
    title: String,
    author: String,
    language: String,
    output_preset: OutputPreset,
}

pub fn init_project(options: InitProjectOptions) -> Result<InitProjectResult, InitProjectError> {
    let template = ProjectTemplate::from_cli(options.config_template.as_deref())?;
    let profile = ProjectProfile::from_cli(options.config_profile.as_deref(), template)?;
    let repo_mode = RepoTemplate::from_cli(options.repo_mode.as_deref(), template)?;
    let scaffold = InitScaffoldConfig {
        template,
        profile,
        title: options.title.unwrap_or_else(|| profile.title().to_string()),
        author: options.author.unwrap_or_else(|| "Author Name".to_string()),
        language: options.language.unwrap_or_else(|| "ja".to_string()),
        output_preset: OutputPreset::from_cli(options.output_preset.as_deref(), profile)?,
    };
    let root = options.root;

    if !options.force && has_existing_config(&root) {
        return Err(InitProjectError::AlreadyInitialized {
            path: root.display().to_string(),
        });
    }

    ensure_dir(&root)?;

    match repo_mode {
        RepoTemplate::SingleBook => init_single_book(&root, &scaffold)?,
        RepoTemplate::Series => init_series(&root, &scaffold)?,
    }

    let mode_label = match repo_mode {
        RepoTemplate::SingleBook => "single-book",
        RepoTemplate::Series => "series",
    };

    Ok(InitProjectResult {
        summary: format!(
            "initialized {mode_label} scaffold for {} at {}{}",
            profile.as_str(),
            root.display(),
            if options.non_interactive {
                " (non-interactive defaults)"
            } else {
                " (interactive answers applied)"
            }
        ),
        root,
    })
}

impl RepoTemplate {
    fn from_cli(value: Option<&str>, template: ProjectTemplate) -> Result<Self, InitProjectError> {
        match value.unwrap_or(match template.default_repo_mode() {
            RepoTemplate::SingleBook => "single-book",
            RepoTemplate::Series => "series",
        }) {
            "single-book" => Ok(Self::SingleBook),
            "series" => Ok(Self::Series),
            other => Err(InitProjectError::UnsupportedRepoMode {
                mode: other.to_string(),
            }),
        }
    }
}

impl OutputPreset {
    fn from_cli(value: Option<&str>, profile: ProjectProfile) -> Result<Self, InitProjectError> {
        match value.unwrap_or(profile.default_output_preset()) {
            "kindle" => Ok(Self::Kindle),
            "print" => Ok(Self::Print),
            "both" => Ok(Self::Both),
            other => Err(InitProjectError::UnsupportedTemplate {
                template: other.to_string(),
            }),
        }
    }
}

impl ProjectTemplate {
    fn from_cli(value: Option<&str>) -> Result<Self, InitProjectError> {
        match value.unwrap_or("novel") {
            "business" => Ok(Self::Business),
            "paper" => Ok(Self::Paper),
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
            Self::Paper => "paper",
            Self::Novel => "novel",
            Self::LightNovel => "light-novel",
            Self::Manga => "manga",
        }
    }

    fn writing_mode(self) -> &'static str {
        match self {
            Self::Business | Self::Paper => "horizontal-ltr",
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
            Self::Business | Self::Paper | Self::Novel | Self::LightNovel => {
                RepoTemplate::SingleBook
            }
        }
    }
}

impl ProjectProfile {
    fn from_cli(value: Option<&str>, template: ProjectTemplate) -> Result<Self, InitProjectError> {
        match value {
            Some("business") if template == ProjectTemplate::Business => Ok(Self::Business),
            Some("paper") if template == ProjectTemplate::Paper => Ok(Self::Paper),
            Some("conference-preprint") if template == ProjectTemplate::Paper => {
                Ok(Self::ConferencePreprint)
            }
            Some("novel") if template == ProjectTemplate::Novel => Ok(Self::Novel),
            Some("light-novel") if template == ProjectTemplate::LightNovel => Ok(Self::LightNovel),
            Some("manga") if template == ProjectTemplate::Manga => Ok(Self::Manga),
            Some(other) => Err(InitProjectError::UnsupportedProfile {
                profile: other.to_string(),
            }),
            None => Ok(Self::default_for_template(template)),
        }
    }

    fn default_for_template(template: ProjectTemplate) -> Self {
        match template {
            ProjectTemplate::Business => Self::Business,
            ProjectTemplate::Paper => Self::Paper,
            ProjectTemplate::Novel => Self::Novel,
            ProjectTemplate::LightNovel => Self::LightNovel,
            ProjectTemplate::Manga => Self::Manga,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Business => "business",
            Self::Paper => "paper",
            Self::ConferencePreprint => "conference-preprint",
            Self::Novel => "novel",
            Self::LightNovel => "light-novel",
            Self::Manga => "manga",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Business => "Untitled Business Book",
            Self::Paper => "Untitled Paper",
            Self::ConferencePreprint => "Untitled Conference Preprint",
            Self::Novel => "Untitled Novel",
            Self::LightNovel => "Untitled Light Novel",
            Self::Manga => "Untitled Manga Volume",
        }
    }

    fn default_output_preset(self) -> &'static str {
        match self {
            Self::Business | Self::Novel | Self::LightNovel | Self::Manga => "kindle",
            Self::Paper | Self::ConferencePreprint => "print",
        }
    }

    fn chapter_start_page(self) -> &'static str {
        match self {
            Self::Paper | Self::ConferencePreprint => "any",
            _ => "odd",
        }
    }

    fn allow_blank_pages(self) -> &'static str {
        match self {
            Self::Paper | Self::ConferencePreprint => "false",
            _ => "true",
        }
    }

    fn manuscript_file(self) -> &'static str {
        match self {
            Self::Paper | Self::ConferencePreprint => "01-main.md",
            _ => "01-chapter-1.md",
        }
    }

    fn manuscript_heading(self) -> &'static str {
        match self {
            Self::Paper | Self::ConferencePreprint => "# Main\n\nWrite here.\n",
            _ => "# Chapter 1\n\nWrite here.\n",
        }
    }
}

fn init_single_book(root: &Path, scaffold: &InitScaffoldConfig) -> Result<(), InitProjectError> {
    let template = scaffold.template;
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
            &root.join(format!("manuscript/{}", scaffold.profile.manuscript_file())),
            scaffold.profile.manuscript_heading(),
        )?;
        write_editorial_scaffold(&root.join("editorial"))?;
    }

    write_file(&root.join("book.yml"), &book_yml(scaffold))?;
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    write_style_scaffold(&root.join("styles"), template, scaffold.profile)?;
    write_agent_skill_templates(root, template, RepoTemplate::SingleBook)?;
    Ok(())
}

fn init_series(root: &Path, scaffold: &InitScaffoldConfig) -> Result<(), InitProjectError> {
    let template = scaffold.template;
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
            &root.join(format!(
                "books/vol-01/manuscript/{}",
                scaffold.profile.manuscript_file()
            )),
            scaffold.profile.manuscript_heading(),
        )?;
    }

    write_file(&root.join("series.yml"), &series_yml(scaffold))?;
    write_file(
        &root.join("books/vol-01/book.yml"),
        &series_book_yml(scaffold),
    )?;
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    write_style_scaffold(&root.join("shared/styles"), template, scaffold.profile)?;
    write_agent_skill_templates(root, template, RepoTemplate::Series)?;
    Ok(())
}

fn book_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let manuscript_block = if template == ProjectTemplate::Manga {
        format!(
            "{}validation:\n  strict: true\n  epubcheck: false\n  accessibility: warn\ngit:\n  lfs: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n",
            outputs_block(scaffold)
        )
    } else {
        format!(
            "manuscript:\n  chapters:\n    - manuscript/{}\n{}validation:\n  strict: true\n  epubcheck: true\n  accessibility: warn\ngit:\n  lfs: true\neditorial:\n  style: editorial/style.yml\n  claims: editorial/claims.yml\n  figures: editorial/figures.yml\n  freshness: editorial/freshness.yml\n",
            scaffold.profile.manuscript_file(),
            outputs_block(scaffold)
        )
    };

    format!(
        "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\n  profile: {}\n  writing_mode: {}\n  reading_direction: {}\nlayout:\n  binding: {}\n  chapter_start_page: {}\n  allow_blank_pages: {}\n{}",
        template.as_str(),
        scaffold.title,
        scaffold.author,
        scaffold.language,
        scaffold.profile.as_str(),
        template.writing_mode(),
        template.reading_direction(),
        template.binding(),
        scaffold.profile.chapter_start_page(),
        scaffold.profile.allow_blank_pages(),
        manuscript_block
    )
}

fn series_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let outputs = indent_block(&outputs_block(scaffold), 2);

    format!(
        "series:\n  id: sample-series\n  title: \"{}\"\n  language: {}\n  type: {}\nshared:\n  assets:\n    - shared/assets\n  styles:\n    - shared/styles\n  fonts:\n    - shared/fonts\n  metadata:\n    - shared/metadata\ndefaults:\n  book:\n    profile: {}\n    writing_mode: {}\n    reading_direction: {}\n  layout:\n    binding: {}\n    chapter_start_page: {}\n    allow_blank_pages: {}\n{}validation:\n  strict: true\n  epubcheck: {}\n  accessibility: warn\ngit:\n  lfs: true\n  require_clean_worktree_for_handoff: true\nbooks:\n  - id: vol-01\n    path: books/vol-01\n    number: 1\n    title: \"Volume 1\"\n",
        scaffold.title,
        scaffold.language,
        template.as_str(),
        scaffold.profile.as_str(),
        template.writing_mode(),
        template.reading_direction(),
        template.binding(),
        scaffold.profile.chapter_start_page(),
        scaffold.profile.allow_blank_pages(),
        outputs,
        if template == ProjectTemplate::Manga {
            "false"
        } else {
            "true"
        }
    )
}

fn series_book_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    if template == ProjectTemplate::Manga {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n",
            template.as_str(),
            scaffold.title,
            scaffold.author,
            scaffold.language,
            template.binding(),
        )
    } else {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\nlayout:\n  binding: {}\n  chapter_start_page: {}\n  allow_blank_pages: {}\nmanuscript:\n  chapters:\n    - books/vol-01/manuscript/{}\neditorial:\n  style: books/vol-01/editorial/style.yml\n  claims: books/vol-01/editorial/claims.yml\n  figures: books/vol-01/editorial/figures.yml\n  freshness: books/vol-01/editorial/freshness.yml\n",
            template.as_str(),
            scaffold.title,
            scaffold.author,
            scaffold.language,
            template.binding(),
            scaffold.profile.chapter_start_page(),
            scaffold.profile.allow_blank_pages(),
            scaffold.profile.manuscript_file(),
        )
    }
}

fn outputs_block(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let profile = scaffold.profile;
    let preset = scaffold.output_preset;
    let mut lines = vec!["outputs:".to_string()];
    if matches!(preset, OutputPreset::Kindle | OutputPreset::Both) {
        let kindle_target = if template == ProjectTemplate::Manga {
            "kindle-comic"
        } else {
            "kindle-ja"
        };
        lines.push("  kindle:".to_string());
        lines.push("    enabled: true".to_string());
        lines.push(format!("    target: {kindle_target}"));
    }
    if matches!(preset, OutputPreset::Print | OutputPreset::Both) {
        let print_target = if template == ProjectTemplate::Manga {
            "print-manga"
        } else if template == ProjectTemplate::Paper {
            "print-jp-pdfx4"
        } else {
            "print-jp-pdfx1a"
        };
        lines.push("  print:".to_string());
        lines.push("    enabled: true".to_string());
        lines.push(format!("    target: {print_target}"));
    }
    if template != ProjectTemplate::Manga
        && matches!(preset, OutputPreset::Print | OutputPreset::Both)
    {
        lines.push("pdf:".to_string());
        lines.push(format!("  engine: {}", default_pdf_engine(profile)));
        match profile {
            ProjectProfile::Paper => {
                lines.push("  toc: false".to_string());
                lines.push("  page_number: true".to_string());
                lines.push("  running_header: none".to_string());
            }
            ProjectProfile::ConferencePreprint => {
                lines.push("  toc: false".to_string());
                lines.push("  page_number: false".to_string());
                lines.push("  running_header: none".to_string());
                lines.push("  column_count: 2".to_string());
                lines.push("  column_gap: 10mm".to_string());
                lines.push("  base_font_size: 9pt".to_string());
                lines.push("  line_height: 14pt".to_string());
            }
            _ => {
                lines.push("  toc: true".to_string());
                lines.push("  page_number: true".to_string());
                lines.push("  running_header: auto".to_string());
            }
        }
        lines.push("print:".to_string());
        match profile {
            ProjectProfile::Paper => {
                lines.push("  trim_size: A4".to_string());
                lines.push("  bleed: 0mm".to_string());
                lines.push("  crop_marks: false".to_string());
            }
            ProjectProfile::ConferencePreprint => {
                lines.push("  trim_size: A4".to_string());
                lines.push("  bleed: 0mm".to_string());
                lines.push("  crop_marks: false".to_string());
                lines.push("  page_margin:".to_string());
                lines.push("    top: 20mm".to_string());
                lines.push("    bottom: 20mm".to_string());
                lines.push("    left: 15mm".to_string());
                lines.push("    right: 15mm".to_string());
                lines.push("  sides: duplex".to_string());
                lines.push("  max_pages: 2".to_string());
            }
            _ => {
                lines.push("  trim_size: bunko".to_string());
                lines.push("  bleed: 3mm".to_string());
                lines.push("  crop_marks: true".to_string());
            }
        }
        lines.push("  body_pdf: true".to_string());
        lines.push("  cover_pdf: false".to_string());
        lines.push(format!(
            "  pdf_standard: {}",
            if template == ProjectTemplate::Paper {
                "pdfx4"
            } else {
                "pdfx1a"
            }
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn default_pdf_engine(profile: ProjectProfile) -> &'static str {
    match profile {
        ProjectProfile::Novel | ProjectProfile::LightNovel => "chromium",
        ProjectProfile::Business
        | ProjectProfile::Paper
        | ProjectProfile::ConferencePreprint
        | ProjectProfile::Manga => "weasyprint",
    }
}

fn indent_block(block: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    block
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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

fn write_style_scaffold(
    root: &Path,
    template: ProjectTemplate,
    profile: ProjectProfile,
) -> Result<(), InitProjectError> {
    ensure_dir(root)?;
    write_file(&root.join("base.css"), base_css_contents(template))?;
    write_file(&root.join("epub.css"), epub_css_contents(template))?;
    write_file(&root.join("print.css"), print_css_contents(profile))?;
    Ok(())
}

fn base_css_contents(template: ProjectTemplate) -> &'static str {
    match template {
        ProjectTemplate::Business => {
            "html {\n  line-height: 1.7;\n}\n\nbody {\n  font-family: sans-serif;\n  line-height: 1.7;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 1.4em 0 0.6em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 1.2em 0;\n}\n"
        }
        ProjectTemplate::Paper => {
            "html {\n  line-height: 1.65;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.65;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 0.9em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 1.3em 0 0.5em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 1em 0;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n"
        }
        ProjectTemplate::Novel => {
            "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 0;\n  margin-block-end: 1em;\n}\n"
        }
        ProjectTemplate::LightNovel => {
            "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nfigure {\n  margin: 0;\n  margin-block-end: 1em;\n  text-align: center;\n}\n\nimg, svg {\n  display: block;\n  margin: 0 auto;\n  max-width: 100%;\n  height: auto;\n}\n"
        }
        ProjectTemplate::Manga => {
            "body {\n  font-family: sans-serif;\n  line-height: 1.6;\n}\n\nimg {\n  display: block;\n  max-width: 100%;\n  height: auto;\n}\n"
        }
    }
}

fn epub_css_contents(template: ProjectTemplate) -> &'static str {
    match template {
        ProjectTemplate::Business => {
            "body {\n  margin: 5%;\n}\n\nnav ol {\n  padding-left: 1.2em;\n}\n\nblockquote {\n  margin-left: 1.5em;\n}\n"
        }
        ProjectTemplate::Paper => {
            "body {\n  margin: 6%;\n}\n\nnav ol {\n  padding-left: 1.2em;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n\ntable {\n  width: 100%;\n  border-collapse: collapse;\n}\n\nth,\ntd {\n  padding: 0.25em 0.5em;\n  border-bottom: 1px solid #999;\n}\n"
        }
        ProjectTemplate::Novel => {
            "body {\n  margin: 4%;\n}\n\nruby rt {\n  font-size: 0.5em;\n}\n\nimg {\n  display: block;\n  margin: 0 auto;\n}\n"
        }
        ProjectTemplate::LightNovel => {
            "body {\n  margin: 4%;\n}\n\nfigure {\n  break-inside: avoid;\n  text-align: center;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n"
        }
        ProjectTemplate::Manga => {
            "/* Manga fixed-layout EPUB styles are generated by the build pipeline. */\n"
        }
    }
}

fn print_css_contents(profile: ProjectProfile) -> &'static str {
    match profile {
        ProjectProfile::Business => {
            "body {\n  font-family: serif;\n}\n\np {\n  orphans: 2;\n  widows: 2;\n}\n\nfigure,\ntable,\npre,\nblockquote {\n  break-inside: avoid;\n}\n\ntable {\n  width: 100%;\n  border-collapse: collapse;\n}\n\nth,\ntd {\n  padding: 0.25em 0.5em;\n  border-bottom: 0.3pt solid #888;\n}\n"
        }
        ProjectProfile::Paper => {
            "body {\n  font-family: serif;\n}\n\np {\n  text-align: justify;\n}\n\nfigure,\ntable,\npre,\nblockquote {\n  break-inside: avoid;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n\ntable {\n  width: 100%;\n  border-collapse: collapse;\n}\n\nth,\ntd {\n  padding: 0.2em 0.4em;\n  border-bottom: 0.3pt solid #888;\n}\n"
        }
        ProjectProfile::ConferencePreprint => {
            "body {\n  font-family: serif;\n}\n\np {\n  text-align: justify;\n}\n\n.abstract,\n.keywords,\nfigure,\ntable,\npre,\nblockquote {\n  break-inside: avoid;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n\ntable {\n  width: 100%;\n  border-collapse: collapse;\n}\n\nth,\ntd {\n  padding: 0.2em 0.4em;\n  border-bottom: 0.3pt solid #888;\n}\n"
        }
        ProjectProfile::Novel => {
            "html {\n  font-size: 10.5pt;\n  line-height: 1.7;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.7;\n}\n\nheader#title-block-header {\n  margin: 0;\n}\n\nheader#title-block-header .title {\n  font-size: 1.55em;\n  line-height: 1.15;\n  margin: 0;\n}\n\nnav#TOC {\n  font-size: 0.9em;\n  line-height: 1.45;\n  margin: 0;\n}\n\nnav#TOC ul {\n  margin: 0;\n  padding: 0;\n}\n\nnav#TOC li {\n  margin: 0 0 0.35em;\n}\n\nnav#TOC li > ul {\n  margin-inline-start: 0.6em;\n}\n\nnav#TOC a {\n  color: inherit;\n  text-decoration: none;\n}\n\nh1 {\n  font-size: 1.5em;\n}\n\nh2 {\n  font-size: 1.25em;\n}\n\nh3 {\n  font-size: 1.1em;\n}\n\np {\n  orphans: 1;\n  widows: 1;\n}\n\nfigure,\ntable,\npre,\nblockquote {\n  break-inside: avoid;\n}\n"
        }
        ProjectProfile::LightNovel => {
            "html {\n  font-size: 10pt;\n  line-height: 1.7;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.7;\n}\n\nheader#title-block-header {\n  margin: 0;\n}\n\nheader#title-block-header .title {\n  font-size: 1.5em;\n  line-height: 1.15;\n  margin: 0;\n}\n\nnav#TOC {\n  font-size: 0.88em;\n  line-height: 1.4;\n  margin: 0;\n}\n\nnav#TOC ul {\n  margin: 0;\n  padding: 0;\n}\n\nnav#TOC li {\n  margin: 0 0 0.3em;\n}\n\nnav#TOC li > ul {\n  margin-inline-start: 0.6em;\n}\n\nnav#TOC a {\n  color: inherit;\n  text-decoration: none;\n}\n\nh1 {\n  font-size: 1.45em;\n}\n\nh2 {\n  font-size: 1.2em;\n}\n\nh3 {\n  font-size: 1.05em;\n}\n\nfigure {\n  break-inside: avoid;\n  text-align: center;\n}\n\nimg {\n  display: block;\n  margin: 0 auto;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n"
        }
        ProjectProfile::Manga => {
            "/* Manga fixed-layout and image print styling is handled by the build pipeline. */\n"
        }
    }
}

fn write_agent_skill_templates(
    root: &Path,
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> Result<(), InitProjectError> {
    write_project_skill_template(root, template, repo_mode)?;
    write_content_review_skill_template(root, template, repo_mode)
}

fn write_project_skill_template(
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
    AgentSkillTemplateContext::new(template, repo_mode).render()
}

fn write_content_review_skill_template(
    root: &Path,
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> Result<(), InitProjectError> {
    let skill_dir = root.join(".agents/skills/shosei-content-review");
    ensure_dir(&skill_dir)?;
    write_file(
        &skill_dir.join("SKILL.md"),
        &content_review_skill_contents(template, repo_mode),
    )
}

fn content_review_skill_contents(template: ProjectTemplate, repo_mode: RepoTemplate) -> String {
    ContentReviewSkillTemplateContext::new(template, repo_mode).render()
}

#[derive(Debug, Clone, Copy)]
struct AgentSkillTemplateContext {
    description: &'static str,
    repo_mode_label: &'static str,
    project_type: &'static str,
    primary_config: &'static str,
    primary_content_paths: &'static str,
    repo_mode_rules: &'static str,
    explain_command: &'static str,
    validate_command: &'static str,
    page_check_rule: &'static str,
    build_command: &'static str,
    preview_command: &'static str,
    handoff_command: &'static str,
}

impl AgentSkillTemplateContext {
    fn new(template: ProjectTemplate, repo_mode: RepoTemplate) -> Self {
        Self {
            description: agent_skill_description(template, repo_mode),
            repo_mode_label: repo_mode_label(repo_mode),
            project_type: template.as_str(),
            primary_config: primary_config_note(repo_mode),
            primary_content_paths: primary_content_paths(template, repo_mode),
            repo_mode_rules: repo_mode_rules(repo_mode),
            explain_command: explain_command(repo_mode),
            validate_command: validate_command(repo_mode),
            page_check_rule: page_check_rule(template, repo_mode),
            build_command: build_command(repo_mode),
            preview_command: preview_command(repo_mode),
            handoff_command: handoff_command(repo_mode),
        }
    }

    fn render(self) -> String {
        render_skill_template(
            SHOSEI_PROJECT_SKILL_TEMPLATE,
            &[
                self.identity_replacements().as_slice(),
                self.layout_replacements().as_slice(),
                self.command_replacements().as_slice(),
            ]
            .concat(),
        )
    }

    fn identity_replacements(self) -> [(&'static str, &'static str); 3] {
        [
            ("{{DESCRIPTION}}", self.description),
            ("{{REPO_MODE}}", self.repo_mode_label),
            ("{{PROJECT_TYPE}}", self.project_type),
        ]
    }

    fn layout_replacements(self) -> [(&'static str, &'static str); 3] {
        [
            ("{{PRIMARY_CONFIG}}", self.primary_config),
            ("{{PRIMARY_CONTENT_PATHS}}", self.primary_content_paths),
            ("{{REPO_MODE_RULES}}", self.repo_mode_rules),
        ]
    }

    fn command_replacements(self) -> [(&'static str, &'static str); 6] {
        [
            ("{{EXPLAIN_COMMAND}}", self.explain_command),
            ("{{VALIDATE_COMMAND}}", self.validate_command),
            ("{{PAGE_CHECK_RULE}}", self.page_check_rule),
            ("{{BUILD_COMMAND}}", self.build_command),
            ("{{PREVIEW_COMMAND}}", self.preview_command),
            ("{{HANDOFF_COMMAND}}", self.handoff_command),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
struct ContentReviewSkillTemplateContext {
    description: &'static str,
    repo_mode_label: &'static str,
    project_type: &'static str,
    primary_config: &'static str,
    primary_content_paths: &'static str,
    optional_content_paths: &'static str,
    review_focus: &'static str,
    repo_mode_rules: &'static str,
    explain_command: &'static str,
    validate_command: &'static str,
    page_check_command: &'static str,
    story_check_command: &'static str,
    reference_check_command: &'static str,
}

impl ContentReviewSkillTemplateContext {
    fn new(template: ProjectTemplate, repo_mode: RepoTemplate) -> Self {
        Self {
            description: content_review_skill_description(template, repo_mode),
            repo_mode_label: repo_mode_label(repo_mode),
            project_type: template.as_str(),
            primary_config: primary_config_note(repo_mode),
            primary_content_paths: content_review_primary_content_paths(template, repo_mode),
            optional_content_paths: content_review_optional_content_paths(template, repo_mode),
            review_focus: content_review_focus(template),
            repo_mode_rules: content_review_repo_mode_rules(repo_mode),
            explain_command: explain_command(repo_mode),
            validate_command: validate_command(repo_mode),
            page_check_command: page_check_command(template, repo_mode),
            story_check_command: story_check_command(repo_mode),
            reference_check_command: reference_check_command(repo_mode),
        }
    }

    fn render(self) -> String {
        render_skill_template(
            SHOSEI_CONTENT_REVIEW_SKILL_TEMPLATE,
            &[
                self.identity_replacements().as_slice(),
                self.layout_replacements().as_slice(),
                self.command_replacements().as_slice(),
            ]
            .concat(),
        )
    }

    fn identity_replacements(self) -> [(&'static str, &'static str); 3] {
        [
            ("{{DESCRIPTION}}", self.description),
            ("{{REPO_MODE}}", self.repo_mode_label),
            ("{{PROJECT_TYPE}}", self.project_type),
        ]
    }

    fn layout_replacements(self) -> [(&'static str, &'static str); 5] {
        [
            ("{{PRIMARY_CONFIG}}", self.primary_config),
            ("{{PRIMARY_CONTENT_PATHS}}", self.primary_content_paths),
            ("{{OPTIONAL_CONTENT_PATHS}}", self.optional_content_paths),
            ("{{REVIEW_FOCUS}}", self.review_focus),
            ("{{REPO_MODE_RULES}}", self.repo_mode_rules),
        ]
    }

    fn command_replacements(self) -> [(&'static str, &'static str); 5] {
        [
            ("{{EXPLAIN_COMMAND}}", self.explain_command),
            ("{{VALIDATE_COMMAND}}", self.validate_command),
            ("{{PAGE_CHECK_COMMAND}}", self.page_check_command),
            ("{{STORY_CHECK_COMMAND}}", self.story_check_command),
            ("{{REFERENCE_CHECK_COMMAND}}", self.reference_check_command),
        ]
    }
}

fn content_review_skill_description(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "Review this `shosei` manga series repo for content quality. Use when the task is to review a volume, chapter, proof packet, page flow, spread logic, dialogue order, or metadata/read-order consistency instead of implementing edits or rewrites."
        }
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "Review this `shosei` manga repo for content quality. Use when the task is to review a chapter, volume, proof packet, page flow, spread logic, dialogue order, or metadata/read-order consistency instead of implementing edits or rewrites."
        }
        (_, RepoTemplate::Series) => {
            "Review this `shosei` series publishing repo for content quality. Use when the task is to review a volume, chapter, manuscript, proof packet, or source-backed nonfiction content instead of implementing edits or rewrites."
        }
        (_, RepoTemplate::SingleBook) => {
            "Review this `shosei` single-book publishing repo for content quality. Use when the task is to review a chapter, manuscript, proof packet, or source-backed nonfiction content instead of implementing edits or rewrites."
        }
    }
}

fn content_review_primary_content_paths(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "`books/<book-id>/manga/`, `books/<book-id>/book.yml`, `shared/styles/`, `shared/assets/`, `shared/fonts/`"
        }
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "`manga/`, `book.yml`, `assets/`, `styles/`"
        }
        (_, RepoTemplate::Series) => {
            "`books/<book-id>/manuscript/`, `books/<book-id>/editorial/`, `books/<book-id>/book.yml`, `shared/styles/`, `shared/assets/`, `shared/fonts/`"
        }
        (_, RepoTemplate::SingleBook) => {
            "`manuscript/`, `editorial/`, `book.yml`, `assets/`, `styles/`"
        }
    }
}

fn content_review_optional_content_paths(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            "`shared/metadata/story/`, `shared/metadata/references/` if those sidecars exist"
        }
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "`story/`, `references/` if those sidecars exist"
        }
        (_, RepoTemplate::Series) => {
            "`books/<book-id>/story/`, `books/<book-id>/references/`, `shared/metadata/story/`, `shared/metadata/references/` if those sidecars exist"
        }
        (_, RepoTemplate::SingleBook) => "`story/`, `references/` if those sidecars exist",
    }
}

fn content_review_focus(template: ProjectTemplate) -> &'static str {
    match template {
        ProjectTemplate::Business | ProjectTemplate::Paper => {
            "claim support, stale facts, weak structure, source-to-text mismatch, and figure/table/caption consistency"
        }
        ProjectTemplate::Novel | ProjectTemplate::LightNovel => {
            "scene-by-scene causality, character knowledge drift, POV / voice drift, pacing, and setup/payoff"
        }
        ProjectTemplate::Manga => {
            "page-turn flow, spread logic, dialogue order, and metadata/read-order consistency"
        }
    }
}

fn content_review_repo_mode_rules(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => {
            "Read from the repository root unless the task explicitly targets a subdirectory; the root config is `book.yml`."
        }
        RepoTemplate::Series => {
            "From the repository root, use `--book <book-id>` for book-scoped checks; the series root is `series.yml` and the book root is `books/<book-id>/book.yml`."
        }
    }
}

fn page_check_command(template: ProjectTemplate, repo_mode: RepoTemplate) -> &'static str {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => "shosei page check",
        (ProjectTemplate::Manga, RepoTemplate::Series) => "shosei page check --book vol-01",
        _ => "Skip `shosei page check` unless this repo is using the manga workflow.",
    }
}

fn story_check_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei story check",
        RepoTemplate::Series => "shosei story check --book vol-01",
    }
}

fn reference_check_command(repo_mode: RepoTemplate) -> &'static str {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei reference check",
        RepoTemplate::Series => "shosei reference check --book vol-01",
    }
}

fn render_skill_template(template: &str, replacements: &[(&'static str, &'static str)]) -> String {
    let mut rendered = template.to_string();
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

    fn read_skill(root: &Path, skill_name: &str) -> String {
        fs::read_to_string(root.join(format!(".agents/skills/{skill_name}/SKILL.md"))).unwrap()
    }

    #[test]
    fn initializes_single_book_novel_scaffold() {
        let root = temp_dir("single");
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("both".to_string()),
        })
        .unwrap();

        assert!(root.join("book.yml").is_file());
        assert!(root.join("manuscript/01-chapter-1.md").is_file());
        assert!(root.join("editorial/style.yml").is_file());
        assert!(root.join("editorial/claims.yml").is_file());
        assert!(root.join("editorial/figures.yml").is_file());
        assert!(root.join("editorial/freshness.yml").is_file());
        assert!(root.join("styles/base.css").is_file());
        assert!(root.join("styles/epub.css").is_file());
        assert!(root.join("styles/print.css").is_file());
        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("editorial:\n  style: editorial/style.yml"));
        assert!(book.contains("engine: chromium"));
        let base_css = fs::read_to_string(root.join("styles/base.css")).unwrap();
        assert!(base_css.contains("writing-mode: vertical-rl"));
        let print_css = fs::read_to_string(root.join("styles/print.css")).unwrap();
        assert!(print_css.contains("font-size: 10.5pt"));
        assert!(print_css.contains("nav#TOC a"));
        assert!(!print_css.contains("page-break-after: always;"));
        let skill = read_skill(&root, "shosei-project");
        assert!(skill.contains("name: \"shosei-project\""));
        assert!(skill.contains("single-book"));
        assert!(skill.contains("shosei explain"));
        assert!(skill.contains("manuscript/"));
        assert!(skill.contains("shosei story scaffold"));
        assert!(skill.contains("shosei story check"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("name: \"shosei-content-review\""));
        assert!(content_review_skill.contains("single-book"));
        assert!(content_review_skill.contains("manuscript/"));
        assert!(content_review_skill.contains("shosei validate"));
        assert!(content_review_skill.contains("findings first"));
        assert!(content_review_skill.contains("scene-by-scene causality"));
        assert!(content_review_skill.contains("rewrite"));
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
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(root.join("books/vol-01/book.yml").is_file());
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("shared/styles/epub.css").is_file());
        assert!(root.join("shared/styles/print.css").is_file());
        assert!(root.join("books/vol-01/manga/pages").is_dir());
        let skill = read_skill(&root, "shosei-project");
        assert!(skill.contains("series"));
        assert!(skill.contains("shosei explain --book vol-01"));
        assert!(skill.contains("shosei page check --book vol-01"));
        assert!(skill.contains("books/<book-id>/manga/"));
        assert!(skill.contains("shosei story scaffold --book <book-id>"));
        assert!(skill.contains("shared/metadata/story/"));
        assert!(skill.contains("resolves scene references against both"));
        assert!(skill.contains("shosei story drift --book <book-id>"));
        assert!(skill.contains("shosei story sync --book <book-id> --from shared"));
        assert!(skill.contains("--to shared"));
        assert!(skill.contains("--report <drift-report> --force"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("series"));
        assert!(content_review_skill.contains("books/<book-id>/manga/"));
        assert!(content_review_skill.contains("shosei page check --book vol-01"));
        assert!(content_review_skill.contains("proof packet"));
        assert!(content_review_skill.contains("page-turn flow"));
        assert!(content_review_skill.contains("dialogue order"));
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
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
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
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
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
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
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
            config_profile: None,
            repo_mode: Some("series".to_string()),
            title: None,
            author: None,
            language: None,
            output_preset: None,
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
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("shared/styles/epub.css").is_file());
        assert!(root.join("shared/styles/print.css").is_file());
        let book = fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap();
        assert!(book.contains("editorial:\n  style: books/vol-01/editorial/style.yml"));
        let base_css = fs::read_to_string(root.join("shared/styles/base.css")).unwrap();
        assert!(base_css.contains("writing-mode: horizontal-tb"));
        let skill = read_skill(&root, "shosei-project");
        assert!(skill.contains("books/<book-id>/editorial/"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("books/<book-id>/manuscript/"));
        assert!(content_review_skill.contains("shosei reference check --book vol-01"));
        assert!(content_review_skill.contains("claim support"));
        assert!(content_review_skill.contains("release-readiness"));
        assert!(result.summary.contains("series scaffold"));
    }

    #[test]
    fn applies_interactive_answers_to_scaffold() {
        let root = temp_dir("interactive-values");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            title: Some("Custom Series".to_string()),
            author: Some("Ken".to_string()),
            language: Some("ja-JP".to_string()),
            output_preset: Some("both".to_string()),
        })
        .unwrap();

        let series = fs::read_to_string(root.join("series.yml")).unwrap();
        assert!(series.contains("title: \"Custom Series\""));
        assert!(series.contains("language: ja-JP"));
        assert!(series.contains("target: kindle-ja"));
        assert!(series.contains("target: print-jp-pdfx1a"));
        let book = fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap();
        assert!(book.contains("- \"Ken\""));
    }

    #[test]
    fn initializes_conference_preprint_scaffold() {
        let root = temp_dir("conference-preprint");
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("paper".to_string()),
            config_profile: Some("conference-preprint".to_string()),
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
        })
        .unwrap();

        assert!(root.join("manuscript/01-main.md").is_file());
        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("type: paper"));
        assert!(book.contains("profile: conference-preprint"));
        assert!(book.contains("target: print-jp-pdfx4"));
        assert!(book.contains("engine: weasyprint"));
        assert!(book.contains("column_count: 2"));
        assert!(book.contains("trim_size: A4"));
        assert!(book.contains("sides: duplex"));
        let print_css = fs::read_to_string(root.join("styles/print.css")).unwrap();
        assert!(print_css.contains(".abstract"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("source-backed nonfiction"));
        assert!(content_review_skill.contains("source-to-text mismatch"));
        assert!(content_review_skill.contains("figure/table/caption consistency"));
        assert!(result.summary.contains("conference-preprint"));
    }

    #[test]
    fn initializes_light_novel_print_css_with_tighter_default_scale() {
        let root = temp_dir("light-novel-print-css");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("light-novel".to_string()),
            config_profile: None,
            repo_mode: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("print".to_string()),
        })
        .unwrap();

        let print_css = fs::read_to_string(root.join("styles/print.css")).unwrap();
        assert!(print_css.contains("font-size: 10pt"));
        assert!(print_css.contains("h1 {\n  font-size: 1.45em;"));
        assert!(print_css.contains("nav#TOC a"));
        assert!(!print_css.contains("page-break-after: always;"));
    }
}
