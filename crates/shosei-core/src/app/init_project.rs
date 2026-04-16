use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

use crate::app::CONFIG_REFERENCE_URL;

const SHOSEI_PROJECT_SKILL_TEMPLATE: &str = include_str!("../../templates/shosei-project-skill.md");
const SHOSEI_CONTENT_REVIEW_SKILL_TEMPLATE: &str =
    include_str!("../../templates/shosei-content-review.md");
const DEFAULT_SERIES_BOOK_ID: &str = "vol-01";

#[derive(Debug, Clone)]
pub struct InitProjectOptions {
    pub root: PathBuf,
    pub non_interactive: bool,
    pub force: bool,
    pub config_template: Option<String>,
    pub config_profile: Option<String>,
    pub repo_mode: Option<String>,
    pub initial_series_book_id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub output_preset: Option<String>,
    pub writing_mode: Option<String>,
    pub binding: Option<String>,
    pub print_target: Option<String>,
    pub print_trim_size: Option<String>,
    pub print_bleed: Option<String>,
    pub print_crop_marks: Option<bool>,
    pub print_sides: Option<String>,
    pub print_max_pages: Option<u64>,
    pub manga_spread_policy_for_kindle: Option<String>,
    pub manga_front_color_pages: Option<u64>,
    pub manga_body_mode: Option<String>,
    pub initialize_git: bool,
    pub git_lfs: Option<bool>,
    pub generate_sample: Option<bool>,
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
    #[error("initial series book id `{book_id}` requires repo mode `series`")]
    InitialSeriesBookIdRequiresSeriesRepoMode { book_id: String },
    #[error("invalid initial series book id `{book_id}`: {reason}")]
    InvalidInitialSeriesBookId {
        book_id: String,
        reason: &'static str,
    },
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
    #[error("unsupported value `{value}` for {field}")]
    UnsupportedValue { field: &'static str, value: String },
    #[error("failed to run `git init` in {path}: {source}")]
    GitInitSpawn {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`git init` failed in {path}: {stderr}")]
    GitInitFailed { path: String, stderr: String },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitWritingMode {
    HorizontalLtr,
    VerticalRl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitBinding {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitPrintTarget {
    PrintPdfx1a,
    PrintPdfx4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitPrintTrimSize {
    A4,
    A5,
    B6,
    Bunko,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitPrintSides {
    Simplex,
    Duplex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitMangaSpreadPolicy {
    Split,
    SinglePage,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitMangaBodyMode {
    Monochrome,
    Mixed,
    Color,
}

#[derive(Debug, Clone)]
struct InitPrintConfig {
    target: InitPrintTarget,
    trim_size: InitPrintTrimSize,
    bleed: String,
    crop_marks: bool,
    sides: Option<InitPrintSides>,
    max_pages: Option<u64>,
}

#[derive(Debug, Clone)]
struct InitMangaConfig {
    spread_policy_for_kindle: InitMangaSpreadPolicy,
    front_color_pages: u64,
    body_mode: InitMangaBodyMode,
}

#[derive(Debug, Clone)]
struct InitScaffoldConfig {
    template: ProjectTemplate,
    profile: ProjectProfile,
    title: String,
    author: String,
    language: String,
    output_preset: OutputPreset,
    initial_series_book_id: String,
    writing_mode: InitWritingMode,
    binding: InitBinding,
    print: Option<InitPrintConfig>,
    manga: Option<InitMangaConfig>,
    git_lfs: bool,
    generate_sample: bool,
}

pub fn init_project(options: InitProjectOptions) -> Result<InitProjectResult, InitProjectError> {
    let template = ProjectTemplate::from_cli(options.config_template.as_deref())?;
    let profile = ProjectProfile::from_cli(options.config_profile.as_deref(), template)?;
    let repo_mode = RepoTemplate::from_cli(options.repo_mode.as_deref(), template)?;
    let initial_series_book_id =
        resolve_initial_series_book_id(repo_mode, options.initial_series_book_id.as_deref())?;
    let writing_mode = InitWritingMode::from_value(options.writing_mode.as_deref(), template)?;
    let binding =
        InitBinding::from_value(options.binding.as_deref(), writing_mode.default_binding())?;
    let output_preset = OutputPreset::from_cli(options.output_preset.as_deref(), profile)?;
    let print = InitPrintConfig::from_options(
        template,
        profile,
        output_preset,
        options.print_target.as_deref(),
        options.print_trim_size.as_deref(),
        options.print_bleed.as_deref(),
        options.print_crop_marks,
        options.print_sides.as_deref(),
        options.print_max_pages,
    )?;
    let manga = InitMangaConfig::from_options(
        template,
        options.manga_spread_policy_for_kindle.as_deref(),
        options.manga_front_color_pages,
        options.manga_body_mode.as_deref(),
    )?;
    let git_lfs = options.git_lfs.unwrap_or(true);
    let generate_sample = options.generate_sample.unwrap_or(true);
    let scaffold = InitScaffoldConfig {
        template,
        profile,
        title: options.title.unwrap_or_else(|| profile.title().to_string()),
        author: options.author.unwrap_or_else(|| "Author Name".to_string()),
        language: options.language.unwrap_or_else(|| "ja".to_string()),
        output_preset,
        initial_series_book_id,
        writing_mode,
        binding,
        print,
        manga,
        git_lfs,
        generate_sample,
    };
    let root = options.root;
    let has_local_git_metadata = git_metadata_exists(&root);

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
    let git_initialized = maybe_init_git(&root, options.initialize_git, has_local_git_metadata)?;

    let mode_label = match repo_mode {
        RepoTemplate::SingleBook => "single-book",
        RepoTemplate::Series => "series",
    };
    let next_steps = next_steps(repo_mode, scaffold.series_book_id());
    let mut summary = vec![
        format!(
            "initialized {mode_label} scaffold for {} at {}{}",
            profile.as_str(),
            root.display(),
            if options.non_interactive {
                " (non-interactive defaults)"
            } else {
                " (interactive answers applied)"
            }
        ),
        format!("config reference: {CONFIG_REFERENCE_URL}"),
        "next:".to_string(),
        format!("- {}", next_steps.explain),
        format!("- {}", next_steps.validate),
    ];
    let setup_steps = setup_steps(has_local_git_metadata, git_initialized, scaffold.git_lfs);
    if !setup_steps.is_empty() {
        summary.push("setup:".to_string());
        summary.extend(setup_steps.into_iter().map(|step| format!("- {step}")));
    }

    Ok(InitProjectResult {
        summary: summary.join("\n"),
        root,
    })
}

struct InitNextSteps {
    explain: String,
    validate: String,
}

fn next_steps(repo_mode: RepoTemplate, initial_book_id: &str) -> InitNextSteps {
    match repo_mode {
        RepoTemplate::SingleBook => InitNextSteps {
            explain: "from the repo root, run: shosei explain".to_string(),
            validate: "then run: shosei validate".to_string(),
        },
        RepoTemplate::Series => InitNextSteps {
            explain: format!("from the repo root, run: shosei explain --book {initial_book_id}"),
            validate: format!("then run: shosei validate --book {initial_book_id}"),
        },
    }
}

fn setup_steps(has_local_git_metadata: bool, git_initialized: bool, git_lfs: bool) -> Vec<String> {
    let mut steps = Vec::new();
    if git_initialized {
        steps.push("initialized Git repository".to_string());
    } else if !has_local_git_metadata {
        steps.push("if this directory is not under Git yet, run: git init".to_string());
    }
    if git_lfs {
        steps.push("if Git LFS is not set up on this machine, run: git lfs install".to_string());
    }
    steps
}

fn git_metadata_exists(root: &Path) -> bool {
    root.join(".git").exists()
}

fn maybe_init_git(
    root: &Path,
    initialize_git: bool,
    has_local_git_metadata: bool,
) -> Result<bool, InitProjectError> {
    if !initialize_git || has_local_git_metadata {
        return Ok(false);
    }

    let output = Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .map_err(|source| InitProjectError::GitInitSpawn {
            path: root.display().to_string(),
            source,
        })?;

    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(InitProjectError::GitInitFailed {
            path: root.display().to_string(),
            stderr: if stderr.is_empty() {
                format!("exit status {}", output.status)
            } else {
                stderr
            },
        })
    }
}

impl InitScaffoldConfig {
    fn series_book_id(&self) -> &str {
        &self.initial_series_book_id
    }
}

fn resolve_initial_series_book_id(
    repo_mode: RepoTemplate,
    initial_series_book_id: Option<&str>,
) -> Result<String, InitProjectError> {
    match repo_mode {
        RepoTemplate::SingleBook => {
            if let Some(book_id) = initial_series_book_id {
                return Err(
                    InitProjectError::InitialSeriesBookIdRequiresSeriesRepoMode {
                        book_id: book_id.to_string(),
                    },
                );
            }
            Ok(DEFAULT_SERIES_BOOK_ID.to_string())
        }
        RepoTemplate::Series => {
            let book_id = initial_series_book_id.unwrap_or(DEFAULT_SERIES_BOOK_ID);
            validate_series_book_id(book_id)?;
            Ok(book_id.to_string())
        }
    }
}

fn validate_series_book_id(book_id: &str) -> Result<(), InitProjectError> {
    let reason = if book_id.is_empty() {
        Some("book id must not be empty")
    } else if matches!(book_id, "." | "..") {
        Some("book id must not be `.` or `..`")
    } else if book_id.contains('/') || book_id.contains('\\') {
        Some("book id must be a single path segment")
    } else if book_id.chars().any(char::is_whitespace) {
        Some("book id must not contain whitespace")
    } else {
        None
    };

    if let Some(reason) = reason {
        Err(InitProjectError::InvalidInitialSeriesBookId {
            book_id: book_id.to_string(),
            reason,
        })
    } else {
        Ok(())
    }
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

impl InitWritingMode {
    fn from_value(
        value: Option<&str>,
        template: ProjectTemplate,
    ) -> Result<Self, InitProjectError> {
        match value.unwrap_or(template.writing_mode()) {
            "horizontal-ltr" => Ok(Self::HorizontalLtr),
            "vertical-rl" => Ok(Self::VerticalRl),
            other => Err(InitProjectError::UnsupportedValue {
                field: "book.writing_mode",
                value: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::HorizontalLtr => "horizontal-ltr",
            Self::VerticalRl => "vertical-rl",
        }
    }

    fn reading_direction(self) -> &'static str {
        match self {
            Self::HorizontalLtr => "ltr",
            Self::VerticalRl => "rtl",
        }
    }

    fn default_binding(self) -> InitBinding {
        match self {
            Self::HorizontalLtr => InitBinding::Left,
            Self::VerticalRl => InitBinding::Right,
        }
    }
}

impl InitBinding {
    fn from_value(value: Option<&str>, default: InitBinding) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            other => Err(InitProjectError::UnsupportedValue {
                field: "layout.binding",
                value: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

impl InitPrintTarget {
    fn from_value(value: Option<&str>, default: InitPrintTarget) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "print-jp-pdfx1a" => Ok(Self::PrintPdfx1a),
            "print-jp-pdfx4" => Ok(Self::PrintPdfx4),
            other => Err(InitProjectError::UnsupportedValue {
                field: "outputs.print.target",
                value: other.to_string(),
            }),
        }
    }

    fn default_for(template: ProjectTemplate, profile: ProjectProfile) -> Self {
        if matches!(
            profile,
            ProjectProfile::Paper | ProjectProfile::ConferencePreprint
        ) || template == ProjectTemplate::Paper
        {
            Self::PrintPdfx4
        } else {
            Self::PrintPdfx1a
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::PrintPdfx1a => "print-jp-pdfx1a",
            Self::PrintPdfx4 => "print-jp-pdfx4",
        }
    }

    fn pdf_standard(self) -> &'static str {
        match self {
            Self::PrintPdfx1a => "pdfx1a",
            Self::PrintPdfx4 => "pdfx4",
        }
    }
}

impl InitPrintTrimSize {
    fn from_value(
        value: Option<&str>,
        default: InitPrintTrimSize,
    ) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "A4" => Ok(Self::A4),
            "A5" => Ok(Self::A5),
            "B6" => Ok(Self::B6),
            "bunko" => Ok(Self::Bunko),
            other => Err(InitProjectError::UnsupportedValue {
                field: "print.trim_size",
                value: other.to_string(),
            }),
        }
    }

    fn default_for(profile: ProjectProfile) -> Self {
        match profile {
            ProjectProfile::Paper | ProjectProfile::ConferencePreprint => Self::A4,
            _ => Self::Bunko,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A5 => "A5",
            Self::B6 => "B6",
            Self::Bunko => "bunko",
        }
    }
}

impl InitPrintSides {
    fn from_value(value: Option<&str>, default: InitPrintSides) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "simplex" => Ok(Self::Simplex),
            "duplex" => Ok(Self::Duplex),
            other => Err(InitProjectError::UnsupportedValue {
                field: "print.sides",
                value: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Simplex => "simplex",
            Self::Duplex => "duplex",
        }
    }
}

impl InitMangaSpreadPolicy {
    fn from_value(
        value: Option<&str>,
        default: InitMangaSpreadPolicy,
    ) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "split" => Ok(Self::Split),
            "single-page" => Ok(Self::SinglePage),
            "skip" => Ok(Self::Skip),
            other => Err(InitProjectError::UnsupportedValue {
                field: "manga.spread_policy_for_kindle",
                value: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::SinglePage => "single-page",
            Self::Skip => "skip",
        }
    }
}

impl InitMangaBodyMode {
    fn from_value(
        value: Option<&str>,
        default: InitMangaBodyMode,
    ) -> Result<Self, InitProjectError> {
        match value.unwrap_or(default.as_str()) {
            "monochrome" => Ok(Self::Monochrome),
            "mixed" => Ok(Self::Mixed),
            "color" => Ok(Self::Color),
            other => Err(InitProjectError::UnsupportedValue {
                field: "manga.body_mode",
                value: other.to_string(),
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Monochrome => "monochrome",
            Self::Mixed => "mixed",
            Self::Color => "color",
        }
    }
}

impl InitPrintConfig {
    #[allow(clippy::too_many_arguments)]
    fn from_options(
        template: ProjectTemplate,
        profile: ProjectProfile,
        output_preset: OutputPreset,
        target: Option<&str>,
        trim_size: Option<&str>,
        bleed: Option<&str>,
        crop_marks: Option<bool>,
        sides: Option<&str>,
        max_pages: Option<u64>,
    ) -> Result<Option<Self>, InitProjectError> {
        if template == ProjectTemplate::Manga
            || !matches!(output_preset, OutputPreset::Print | OutputPreset::Both)
        {
            return Ok(None);
        }

        let default_target = InitPrintTarget::default_for(template, profile);
        let trim_size_default = InitPrintTrimSize::default_for(profile);
        Ok(Some(Self {
            target: InitPrintTarget::from_value(target, default_target)?,
            trim_size: InitPrintTrimSize::from_value(trim_size, trim_size_default)?,
            bleed: bleed
                .unwrap_or(match profile {
                    ProjectProfile::Paper | ProjectProfile::ConferencePreprint => "0mm",
                    _ => "3mm",
                })
                .to_string(),
            crop_marks: crop_marks.unwrap_or(!matches!(
                profile,
                ProjectProfile::Paper | ProjectProfile::ConferencePreprint
            )),
            sides: if profile == ProjectProfile::ConferencePreprint {
                Some(InitPrintSides::from_value(sides, InitPrintSides::Duplex)?)
            } else {
                None
            },
            max_pages: if profile == ProjectProfile::ConferencePreprint {
                Some(max_pages.unwrap_or(2))
            } else {
                None
            },
        }))
    }
}

impl InitMangaConfig {
    fn from_options(
        template: ProjectTemplate,
        spread_policy_for_kindle: Option<&str>,
        front_color_pages: Option<u64>,
        body_mode: Option<&str>,
    ) -> Result<Option<Self>, InitProjectError> {
        if template != ProjectTemplate::Manga {
            return Ok(None);
        }

        Ok(Some(Self {
            spread_policy_for_kindle: InitMangaSpreadPolicy::from_value(
                spread_policy_for_kindle,
                InitMangaSpreadPolicy::Split,
            )?,
            front_color_pages: front_color_pages.unwrap_or(0),
            body_mode: InitMangaBodyMode::from_value(body_mode, InitMangaBodyMode::Monochrome)?,
        }))
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

    fn manuscript_heading(self, generate_sample: bool) -> &'static str {
        if !generate_sample {
            ""
        } else {
            match self {
                Self::Paper | Self::ConferencePreprint => "# Main\n\nWrite here.\n",
                _ => "# Chapter 1\n\nWrite here.\n",
            }
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
            scaffold
                .profile
                .manuscript_heading(scaffold.generate_sample),
        )?;
        write_editorial_scaffold(&root.join("editorial"))?;
    }

    write_file(&root.join("book.yml"), &book_yml(scaffold))?;
    write_git_scaffold(root, scaffold.git_lfs)?;
    write_style_scaffold(
        &root.join("styles"),
        template,
        scaffold.profile,
        scaffold.writing_mode,
    )?;
    write_agent_skill_templates(
        root,
        template,
        RepoTemplate::SingleBook,
        scaffold.series_book_id(),
    )?;
    Ok(())
}

fn init_series(root: &Path, scaffold: &InitScaffoldConfig) -> Result<(), InitProjectError> {
    let template = scaffold.template;
    let book_root = root.join("books").join(scaffold.series_book_id());
    ensure_dir(&root.join("shared/assets"))?;
    ensure_dir(&root.join("shared/styles"))?;
    ensure_dir(&root.join("shared/fonts"))?;
    ensure_dir(&root.join("shared/metadata"))?;
    ensure_dir(&book_root.join("assets"))?;
    ensure_dir(&root.join("dist"))?;

    if template == ProjectTemplate::Manga {
        ensure_dir(&book_root.join("manga/script"))?;
        ensure_dir(&book_root.join("manga/storyboard"))?;
        ensure_dir(&book_root.join("manga/pages"))?;
        ensure_dir(&book_root.join("manga/spreads"))?;
        ensure_dir(&book_root.join("manga/metadata"))?;
    } else {
        ensure_dir(&book_root.join("manuscript"))?;
        write_editorial_scaffold(&book_root.join("editorial"))?;
        write_file(
            &book_root
                .join("manuscript")
                .join(scaffold.profile.manuscript_file()),
            scaffold
                .profile
                .manuscript_heading(scaffold.generate_sample),
        )?;
    }

    write_file(&root.join("series.yml"), &series_yml(scaffold))?;
    write_file(&book_root.join("book.yml"), &series_book_yml(scaffold))?;
    write_git_scaffold(root, scaffold.git_lfs)?;
    write_style_scaffold(
        &root.join("shared/styles"),
        template,
        scaffold.profile,
        scaffold.writing_mode,
    )?;
    write_agent_skill_templates(
        root,
        template,
        RepoTemplate::Series,
        scaffold.series_book_id(),
    )?;
    Ok(())
}

fn book_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let manuscript_block = if template == ProjectTemplate::Manga {
        format!(
            "{}validation:\n  strict: true\n  epubcheck: false\n  accessibility: warn\ngit:\n  lfs: {}\nmanga:\n  reading_direction: {}\n  default_page_side: {}\n  spread_policy_for_kindle: {}\n  front_color_pages: {}\n  body_mode: {}\n",
            outputs_block(scaffold),
            if scaffold.git_lfs { "true" } else { "false" },
            scaffold.writing_mode.reading_direction(),
            if scaffold.binding == InitBinding::Left {
                "left"
            } else {
                "right"
            },
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .spread_policy_for_kindle
                .as_str(),
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .front_color_pages,
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .body_mode
                .as_str()
        )
    } else {
        format!(
            "manuscript:\n  chapters:\n    - manuscript/{}\n{}validation:\n  strict: true\n  epubcheck: true\n  accessibility: warn\ngit:\n  lfs: {}\neditorial:\n  style: editorial/style.yml\n  claims: editorial/claims.yml\n  figures: editorial/figures.yml\n  freshness: editorial/freshness.yml\n",
            scaffold.profile.manuscript_file(),
            outputs_block(scaffold),
            if scaffold.git_lfs { "true" } else { "false" }
        )
    };

    format!(
        "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\n  profile: {}\n  writing_mode: {}\n  reading_direction: {}\nlayout:\n  binding: {}\n  chapter_start_page: {}\n  allow_blank_pages: {}\n{}",
        template.as_str(),
        scaffold.title,
        scaffold.author,
        scaffold.language,
        scaffold.profile.as_str(),
        scaffold.writing_mode.as_str(),
        scaffold.writing_mode.reading_direction(),
        scaffold.binding.as_str(),
        scaffold.profile.chapter_start_page(),
        scaffold.profile.allow_blank_pages(),
        manuscript_block
    )
}

fn series_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let outputs = indent_block(&outputs_block(scaffold), 2);
    let book_id = scaffold.series_book_id();
    let book_path = format!("books/{book_id}");

    format!(
        "series:\n  id: sample-series\n  title: \"{}\"\n  language: {}\n  type: {}\nshared:\n  assets:\n    - shared/assets\n  styles:\n    - shared/styles\n  fonts:\n    - shared/fonts\n  metadata:\n    - shared/metadata\ndefaults:\n  book:\n    profile: {}\n    writing_mode: {}\n    reading_direction: {}\n  layout:\n    binding: {}\n    chapter_start_page: {}\n    allow_blank_pages: {}\n{}validation:\n  strict: true\n  epubcheck: {}\n  accessibility: warn\ngit:\n  lfs: {}\n  require_clean_worktree_for_handoff: true\nbooks:\n  - id: {}\n    path: {}\n    number: 1\n    title: \"Volume 1\"\n",
        scaffold.title,
        scaffold.language,
        template.as_str(),
        scaffold.profile.as_str(),
        scaffold.writing_mode.as_str(),
        scaffold.writing_mode.reading_direction(),
        scaffold.binding.as_str(),
        scaffold.profile.chapter_start_page(),
        scaffold.profile.allow_blank_pages(),
        outputs,
        if template == ProjectTemplate::Manga {
            "false"
        } else {
            "true"
        },
        if scaffold.git_lfs { "true" } else { "false" },
        book_id,
        book_path,
    )
}

fn series_book_yml(scaffold: &InitScaffoldConfig) -> String {
    let template = scaffold.template;
    let book_root = format!("books/{}", scaffold.series_book_id());
    if template == ProjectTemplate::Manga {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\nlayout:\n  binding: {}\n  chapter_start_page: odd\n  allow_blank_pages: true\nmanga:\n  reading_direction: {}\n  default_page_side: {}\n  spread_policy_for_kindle: {}\n  front_color_pages: {}\n  body_mode: {}\n",
            template.as_str(),
            scaffold.title,
            scaffold.author,
            scaffold.language,
            scaffold.binding.as_str(),
            scaffold.writing_mode.reading_direction(),
            if scaffold.binding == InitBinding::Left {
                "left"
            } else {
                "right"
            },
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .spread_policy_for_kindle
                .as_str(),
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .front_color_pages,
            scaffold
                .manga
                .as_ref()
                .expect("manga config must exist for manga templates")
                .body_mode
                .as_str(),
        )
    } else {
        format!(
            "project:\n  type: {}\n  vcs: git\n  version: 1\nbook:\n  title: \"{}\"\n  authors:\n    - \"{}\"\n  language: {}\nlayout:\n  binding: {}\n  chapter_start_page: {}\n  allow_blank_pages: {}\nmanuscript:\n  chapters:\n    - {}/manuscript/{}\neditorial:\n  style: {}/editorial/style.yml\n  claims: {}/editorial/claims.yml\n  figures: {}/editorial/figures.yml\n  freshness: {}/editorial/freshness.yml\n",
            template.as_str(),
            scaffold.title,
            scaffold.author,
            scaffold.language,
            scaffold.binding.as_str(),
            scaffold.profile.chapter_start_page(),
            scaffold.profile.allow_blank_pages(),
            book_root,
            scaffold.profile.manuscript_file(),
            book_root,
            book_root,
            book_root,
            book_root,
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
        } else {
            scaffold
                .print
                .as_ref()
                .expect("prose print config must exist when print output is enabled")
                .target
                .as_str()
        };
        lines.push("  print:".to_string());
        lines.push("    enabled: true".to_string());
        lines.push(format!("    target: {print_target}"));
    }
    if template != ProjectTemplate::Manga
        && matches!(preset, OutputPreset::Print | OutputPreset::Both)
    {
        let print = scaffold
            .print
            .as_ref()
            .expect("prose print config must exist when print output is enabled");
        lines.push("pdf:".to_string());
        lines.push(format!(
            "  engine: {}",
            default_pdf_engine(profile, scaffold.writing_mode)
        ));
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
        lines.push(format!("  trim_size: {}", print.trim_size.as_str()));
        lines.push(format!("  bleed: {}", print.bleed));
        lines.push(format!(
            "  crop_marks: {}",
            if print.crop_marks { "true" } else { "false" }
        ));
        if profile == ProjectProfile::ConferencePreprint {
            lines.push("  page_margin:".to_string());
            lines.push("    top: 20mm".to_string());
            lines.push("    bottom: 20mm".to_string());
            lines.push("    left: 15mm".to_string());
            lines.push("    right: 15mm".to_string());
            lines.push(format!(
                "  sides: {}",
                print
                    .sides
                    .expect("conference-preprint print sides must exist")
                    .as_str()
            ));
            lines.push(format!(
                "  max_pages: {}",
                print
                    .max_pages
                    .expect("conference-preprint max pages must exist")
            ));
        }
        lines.push("  body_pdf: true".to_string());
        lines.push("  cover_pdf: false".to_string());
        lines.push(format!("  pdf_standard: {}", print.target.pdf_standard()));
    }
    format!("{}\n", lines.join("\n"))
}

fn default_pdf_engine(profile: ProjectProfile, writing_mode: InitWritingMode) -> &'static str {
    match profile {
        ProjectProfile::Business
        | ProjectProfile::Paper
        | ProjectProfile::Novel
        | ProjectProfile::LightNovel => {
            if writing_mode == InitWritingMode::VerticalRl {
                "chromium"
            } else {
                "weasyprint"
            }
        }
        ProjectProfile::ConferencePreprint | ProjectProfile::Manga => "weasyprint",
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

fn write_git_scaffold(root: &Path, git_lfs: bool) -> Result<(), InitProjectError> {
    write_file(&root.join(".gitignore"), gitignore_contents())?;
    if git_lfs {
        write_file(&root.join(".gitattributes"), gitattributes_contents())?;
    } else {
        remove_file_if_exists(&root.join(".gitattributes"))?;
    }
    Ok(())
}

fn write_style_scaffold(
    root: &Path,
    template: ProjectTemplate,
    profile: ProjectProfile,
    writing_mode: InitWritingMode,
) -> Result<(), InitProjectError> {
    ensure_dir(root)?;
    write_file(
        &root.join("base.css"),
        base_css_contents(template, writing_mode),
    )?;
    write_file(&root.join("epub.css"), epub_css_contents(template))?;
    write_file(&root.join("print.css"), print_css_contents(profile))?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), InitProjectError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(InitProjectError::WriteFile {
            path: path.display().to_string(),
            source,
        }),
    }
}

fn base_css_contents(template: ProjectTemplate, writing_mode: InitWritingMode) -> &'static str {
    match template {
        ProjectTemplate::Business => match writing_mode {
            InitWritingMode::HorizontalLtr => {
                "html {\n  line-height: 1.7;\n}\n\nbody {\n  font-family: sans-serif;\n  line-height: 1.7;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 1.4em 0 0.6em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 1.2em 0;\n}\n"
            }
            InitWritingMode::VerticalRl => {
                "html {\n  line-height: 1.7;\n}\n\nbody {\n  font-family: sans-serif;\n  line-height: 1.7;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 0;\n  margin-block-end: 0.8em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 0;\n  margin-block-end: 1em;\n}\n"
            }
        },
        ProjectTemplate::Paper => match writing_mode {
            InitWritingMode::HorizontalLtr => {
                "html {\n  line-height: 1.65;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.65;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 0.9em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 1.3em 0 0.5em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 1em 0;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n"
            }
            InitWritingMode::VerticalRl => {
                "html {\n  line-height: 1.65;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.65;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 0.9em;\n}\n\nh1, h2, h3 {\n  line-height: 1.3;\n  margin: 0;\n  margin-block-end: 0.7em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nfigcaption {\n  font-size: 0.9em;\n}\n"
            }
        },
        ProjectTemplate::Novel => match writing_mode {
            InitWritingMode::HorizontalLtr => {
                "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 1.2em 0 0.6em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 1em 0;\n}\n"
            }
            InitWritingMode::VerticalRl => {
                "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nimg, svg {\n  max-width: 100%;\n  height: auto;\n}\n\nfigure, table, pre, blockquote {\n  margin: 0;\n  margin-block-end: 1em;\n}\n"
            }
        },
        ProjectTemplate::LightNovel => match writing_mode {
            InitWritingMode::HorizontalLtr => {
                "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: horizontal-tb;\n  direction: ltr;\n}\n\np {\n  margin: 0 0 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 1.2em 0 0.6em;\n}\n\nfigure {\n  margin: 1em 0;\n  text-align: center;\n}\n\nimg, svg {\n  display: block;\n  margin: 0 auto;\n  max-width: 100%;\n  height: auto;\n}\n"
            }
            InitWritingMode::VerticalRl => {
                "html {\n  line-height: 1.9;\n}\n\nbody {\n  font-family: serif;\n  line-height: 1.9;\n  writing-mode: vertical-rl;\n  -epub-writing-mode: vertical-rl;\n  -webkit-writing-mode: vertical-rl;\n  text-orientation: mixed;\n}\n\np {\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nh1, h2, h3 {\n  line-height: 1.4;\n  margin: 0;\n  margin-block-end: 1em;\n}\n\nfigure {\n  margin: 0;\n  margin-block-end: 1em;\n  text-align: center;\n}\n\nimg, svg {\n  display: block;\n  margin: 0 auto;\n  max-width: 100%;\n  height: auto;\n}\n"
            }
        },
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
    initial_book_id: &str,
) -> Result<(), InitProjectError> {
    write_project_skill_template(root, template, repo_mode, initial_book_id)?;
    write_content_review_skill_template(root, template, repo_mode, initial_book_id)
}

fn write_project_skill_template(
    root: &Path,
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> Result<(), InitProjectError> {
    let skill_dir = root.join(".agents/skills/shosei-project");
    ensure_dir(&skill_dir)?;
    write_file(
        &skill_dir.join("SKILL.md"),
        &agent_skill_contents(template, repo_mode, initial_book_id),
    )
}

fn agent_skill_contents(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> String {
    AgentSkillTemplateContext::new(template, repo_mode, initial_book_id).render()
}

fn write_content_review_skill_template(
    root: &Path,
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> Result<(), InitProjectError> {
    let skill_dir = root.join(".agents/skills/shosei-content-review");
    ensure_dir(&skill_dir)?;
    write_file(
        &skill_dir.join("SKILL.md"),
        &content_review_skill_contents(template, repo_mode, initial_book_id),
    )
}

fn content_review_skill_contents(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> String {
    ContentReviewSkillTemplateContext::new(template, repo_mode, initial_book_id).render()
}

#[derive(Debug, Clone)]
struct AgentSkillTemplateContext {
    description: &'static str,
    repo_mode_label: &'static str,
    project_type: &'static str,
    primary_config: &'static str,
    primary_content_paths: &'static str,
    repo_mode_rules: &'static str,
    explain_command: String,
    validate_command: String,
    page_check_rule: String,
    build_command: String,
    preview_command: String,
    handoff_command: String,
}

impl AgentSkillTemplateContext {
    fn new(template: ProjectTemplate, repo_mode: RepoTemplate, initial_book_id: &str) -> Self {
        Self {
            description: agent_skill_description(template, repo_mode),
            repo_mode_label: repo_mode_label(repo_mode),
            project_type: template.as_str(),
            primary_config: primary_config_note(repo_mode),
            primary_content_paths: primary_content_paths(template, repo_mode),
            repo_mode_rules: repo_mode_rules(repo_mode),
            explain_command: explain_command(repo_mode, initial_book_id),
            validate_command: validate_command(repo_mode, initial_book_id),
            page_check_rule: page_check_rule(template, repo_mode, initial_book_id),
            build_command: build_command(repo_mode, initial_book_id),
            preview_command: preview_command(repo_mode, initial_book_id),
            handoff_command: handoff_command(repo_mode, initial_book_id),
        }
    }

    fn render(&self) -> String {
        let replacements = [
            ("{{DESCRIPTION}}", self.description),
            ("{{REPO_MODE}}", self.repo_mode_label),
            ("{{PROJECT_TYPE}}", self.project_type),
            ("{{PRIMARY_CONFIG}}", self.primary_config),
            ("{{PRIMARY_CONTENT_PATHS}}", self.primary_content_paths),
            ("{{REPO_MODE_RULES}}", self.repo_mode_rules),
            ("{{EXPLAIN_COMMAND}}", self.explain_command.as_str()),
            ("{{VALIDATE_COMMAND}}", self.validate_command.as_str()),
            ("{{PAGE_CHECK_RULE}}", self.page_check_rule.as_str()),
            ("{{BUILD_COMMAND}}", self.build_command.as_str()),
            ("{{PREVIEW_COMMAND}}", self.preview_command.as_str()),
            ("{{HANDOFF_COMMAND}}", self.handoff_command.as_str()),
        ];
        render_skill_template(SHOSEI_PROJECT_SKILL_TEMPLATE, &replacements)
    }
}

#[derive(Debug, Clone)]
struct ContentReviewSkillTemplateContext {
    description: &'static str,
    repo_mode_label: &'static str,
    project_type: &'static str,
    primary_config: &'static str,
    primary_content_paths: &'static str,
    optional_content_paths: &'static str,
    review_focus: &'static str,
    repo_mode_rules: &'static str,
    explain_command: String,
    validate_command: String,
    page_check_command: String,
    story_check_command: String,
    reference_map_command: String,
    reference_check_command: String,
    reference_alignment_command: String,
}

impl ContentReviewSkillTemplateContext {
    fn new(template: ProjectTemplate, repo_mode: RepoTemplate, initial_book_id: &str) -> Self {
        Self {
            description: content_review_skill_description(template, repo_mode),
            repo_mode_label: repo_mode_label(repo_mode),
            project_type: template.as_str(),
            primary_config: primary_config_note(repo_mode),
            primary_content_paths: content_review_primary_content_paths(template, repo_mode),
            optional_content_paths: content_review_optional_content_paths(template, repo_mode),
            review_focus: content_review_focus(template),
            repo_mode_rules: content_review_repo_mode_rules(repo_mode),
            explain_command: explain_command(repo_mode, initial_book_id),
            validate_command: validate_command(repo_mode, initial_book_id),
            page_check_command: page_check_command(template, repo_mode, initial_book_id),
            story_check_command: story_check_command(repo_mode, initial_book_id),
            reference_map_command: reference_map_command(repo_mode, initial_book_id),
            reference_check_command: reference_check_command(repo_mode, initial_book_id),
            reference_alignment_command: reference_alignment_command(repo_mode, initial_book_id),
        }
    }

    fn render(&self) -> String {
        let replacements = [
            ("{{DESCRIPTION}}", self.description),
            ("{{REPO_MODE}}", self.repo_mode_label),
            ("{{PROJECT_TYPE}}", self.project_type),
            ("{{PRIMARY_CONFIG}}", self.primary_config),
            ("{{PRIMARY_CONTENT_PATHS}}", self.primary_content_paths),
            ("{{OPTIONAL_CONTENT_PATHS}}", self.optional_content_paths),
            ("{{REVIEW_FOCUS}}", self.review_focus),
            ("{{REPO_MODE_RULES}}", self.repo_mode_rules),
            ("{{EXPLAIN_COMMAND}}", self.explain_command.as_str()),
            ("{{VALIDATE_COMMAND}}", self.validate_command.as_str()),
            ("{{PAGE_CHECK_COMMAND}}", self.page_check_command.as_str()),
            ("{{STORY_CHECK_COMMAND}}", self.story_check_command.as_str()),
            (
                "{{REFERENCE_MAP_COMMAND}}",
                self.reference_map_command.as_str(),
            ),
            (
                "{{REFERENCE_CHECK_COMMAND}}",
                self.reference_check_command.as_str(),
            ),
            (
                "{{REFERENCE_ALIGNMENT_COMMAND}}",
                self.reference_alignment_command.as_str(),
            ),
        ];
        render_skill_template(SHOSEI_CONTENT_REVIEW_SKILL_TEMPLATE, &replacements)
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

fn page_check_command(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> String {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "Use `shosei page check` when the review scope includes page order, spreads, or proof packet flow."
                .to_string()
        }
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            format!(
                "Use `shosei page check --book {initial_book_id}` when the review scope includes page order, spreads, or proof packet flow."
            )
        }
        _ => "Skip `shosei page check` unless this repo is using the manga workflow."
            .to_string(),
    }
}

fn story_check_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei story check".to_string(),
        RepoTemplate::Series => format!("shosei story check --book {initial_book_id}"),
    }
}

fn reference_map_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => {
            "Use `shosei reference map` to inventory available reference entries before reviewing source-backed sections, claim support, or release-readiness when reference sidecars are present.".to_string()
        }
        RepoTemplate::Series => format!(
            "Use `shosei reference map --book {initial_book_id}` for book-scoped entries and `shosei reference map --shared` for shared reference entries before reviewing source-backed sections, claim support, or release-readiness."
        ),
    }
}

fn reference_check_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => {
            "Use `shosei reference check` when reference or source sidecars are present."
                .to_string()
        }
        RepoTemplate::Series => format!(
            "Use `shosei reference check --book {initial_book_id}` for book-scoped reference sidecars or `shosei reference check --shared` for shared reference sidecars."
        ),
    }
}

fn reference_alignment_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => {
            "After `shosei reference map` or `shosei reference check`, read the relevant files under `references/entries/` directly instead of relying on report shape alone when claim support is in question.".to_string()
        }
        RepoTemplate::Series => format!(
            "When both shared and book-scoped reference sidecars may matter, use `shosei reference drift --book {initial_book_id}` before assuming either scope is the source of truth, then read the relevant files under `books/<book-id>/references/entries/` or `shared/metadata/references/entries/` directly."
        ),
    }
}

fn render_skill_template(template: &str, replacements: &[(&'static str, &str)]) -> String {
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

fn explain_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei explain".to_string(),
        RepoTemplate::Series => format!("shosei explain --book {initial_book_id}"),
    }
}

fn validate_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei validate".to_string(),
        RepoTemplate::Series => format!("shosei validate --book {initial_book_id}"),
    }
}

fn page_check_rule(
    template: ProjectTemplate,
    repo_mode: RepoTemplate,
    initial_book_id: &str,
) -> String {
    match (template, repo_mode) {
        (ProjectTemplate::Manga, RepoTemplate::SingleBook) => {
            "Run `shosei page check` after changing manga page assets, page order, or spread-related settings."
                .to_string()
        }
        (ProjectTemplate::Manga, RepoTemplate::Series) => {
            format!(
                "Run `shosei page check --book {initial_book_id}` after changing manga page assets, page order, or spread-related settings."
            )
        }
        _ => "Skip `page check` unless this repo is using the manga workflow.".to_string(),
    }
}

fn build_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei build".to_string(),
        RepoTemplate::Series => format!("shosei build --book {initial_book_id}"),
    }
}

fn preview_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei preview".to_string(),
        RepoTemplate::Series => format!("shosei preview --book {initial_book_id}"),
    }
}

fn handoff_command(repo_mode: RepoTemplate, initial_book_id: &str) -> String {
    match repo_mode {
        RepoTemplate::SingleBook => "shosei handoff print".to_string(),
        RepoTemplate::Series => format!("shosei handoff print --book {initial_book_id}"),
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("both".to_string()),
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
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
        crate::config::load_book_config(&root.join("book.yml")).unwrap();
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
        assert!(content_review_skill.contains("shosei reference map"));
        assert!(content_review_skill.contains("primary review aids"));
        assert!(content_review_skill.contains("rewrite"));
        assert!(result.summary.contains("single-book scaffold"));
        assert!(result.summary.contains("config reference:"));
        assert!(
            result
                .summary
                .contains("from the repo root, run: shosei explain")
        );
        assert!(result.summary.contains("then run: shosei validate"));
        assert!(
            result
                .summary
                .contains("if this directory is not under Git yet, run: git init")
        );
        assert!(
            result
                .summary
                .contains("if Git LFS is not set up on this machine, run: git lfs install")
        );
    }

    #[test]
    fn initializes_series_manga_scaffold() {
        let root = temp_dir("series");
        let book_root = root.join("books").join(DEFAULT_SERIES_BOOK_ID);
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("manga".to_string()),
            config_profile: None,
            repo_mode: None,
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(book_root.join("book.yml").is_file());
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("shared/styles/epub.css").is_file());
        assert!(root.join("shared/styles/print.css").is_file());
        assert!(book_root.join("manga/pages").is_dir());
        crate::config::load_series_config(&root.join("series.yml")).unwrap();
        crate::config::load_book_config(&book_root.join("book.yml")).unwrap();
        let skill = read_skill(&root, "shosei-project");
        assert!(skill.contains("series"));
        assert!(skill.contains(&format!("shosei explain --book {DEFAULT_SERIES_BOOK_ID}")));
        assert!(skill.contains(&format!(
            "shosei page check --book {DEFAULT_SERIES_BOOK_ID}"
        )));
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
        assert!(content_review_skill.contains(&format!(
            "shosei page check --book {DEFAULT_SERIES_BOOK_ID}"
        )));
        assert!(content_review_skill.contains("proof packet"));
        assert!(content_review_skill.contains("page-turn flow"));
        assert!(content_review_skill.contains("dialogue order"));
        assert!(result.summary.contains("config reference:"));
        assert!(result.summary.contains(&format!(
            "from the repo root, run: shosei explain --book {DEFAULT_SERIES_BOOK_ID}"
        )));
        assert!(result.summary.contains(&format!(
            "then run: shosei validate --book {DEFAULT_SERIES_BOOK_ID}"
        )));
        assert!(
            result
                .summary
                .contains("if this directory is not under Git yet, run: git init")
        );
        assert!(
            result
                .summary
                .contains("if Git LFS is not set up on this machine, run: git lfs install")
        );
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("type: business"));
    }

    #[test]
    fn skips_git_init_hint_when_local_git_metadata_exists() {
        let root = temp_dir("git-metadata");
        fs::create_dir_all(root.join(".git")).unwrap();

        let result = init_project(InitProjectOptions {
            root,
            non_interactive: true,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: None,
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        assert!(!result.summary.contains("run: git init"));
        assert!(result.summary.contains("run: git lfs install"));
    }

    #[test]
    fn rejects_initial_series_book_id_for_single_book() {
        let root = temp_dir("single-book-id-override");
        let error = init_project(InitProjectOptions {
            root,
            non_interactive: true,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: Some("single-book".to_string()),
            initial_series_book_id: Some("pilot".to_string()),
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            InitProjectError::InitialSeriesBookIdRequiresSeriesRepoMode { .. }
        ));
    }

    #[test]
    fn rejects_invalid_initial_series_book_id() {
        let root = temp_dir("invalid-series-book-id");
        let error = init_project(InitProjectOptions {
            root,
            non_interactive: true,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            initial_series_book_id: Some("bad/id".to_string()),
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            InitProjectError::InvalidInitialSeriesBookId { .. }
        ));
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
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
        let book_root = root.join("books").join(DEFAULT_SERIES_BOOK_ID);
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: true,
            force: false,
            config_template: Some("business".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        assert!(root.join("series.yml").is_file());
        assert!(book_root.join("book.yml").is_file());
        assert!(book_root.join("manuscript/01-chapter-1.md").is_file());
        assert!(book_root.join("editorial/style.yml").is_file());
        assert!(book_root.join("editorial/claims.yml").is_file());
        assert!(book_root.join("editorial/figures.yml").is_file());
        assert!(book_root.join("editorial/freshness.yml").is_file());
        assert!(root.join("shared/styles/base.css").is_file());
        assert!(root.join("shared/styles/epub.css").is_file());
        assert!(root.join("shared/styles/print.css").is_file());
        let book = fs::read_to_string(book_root.join("book.yml")).unwrap();
        assert!(book.contains(&format!(
            "editorial:\n  style: books/{DEFAULT_SERIES_BOOK_ID}/editorial/style.yml"
        )));
        crate::config::load_book_config(&book_root.join("book.yml")).unwrap();
        let base_css = fs::read_to_string(root.join("shared/styles/base.css")).unwrap();
        assert!(base_css.contains("writing-mode: horizontal-tb"));
        let skill = read_skill(&root, "shosei-project");
        assert!(skill.contains("books/<book-id>/editorial/"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("books/<book-id>/manuscript/"));
        assert!(content_review_skill.contains("shosei reference map --book vol-01"));
        assert!(content_review_skill.contains("shosei reference map --shared"));
        assert!(content_review_skill.contains("shosei reference check --book vol-01"));
        assert!(content_review_skill.contains("shosei reference check --shared"));
        assert!(content_review_skill.contains("shosei reference drift --book vol-01"));
        assert!(content_review_skill.contains("claim support"));
        assert!(content_review_skill.contains("release-readiness"));
        assert!(result.summary.contains("series scaffold"));
        assert!(result.summary.contains("config reference:"));
    }

    #[test]
    fn applies_interactive_answers_to_scaffold() {
        let root = temp_dir("interactive-values");
        let book_root = root.join("books").join("pilot");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            initial_series_book_id: Some("pilot".to_string()),
            title: Some("Custom Series".to_string()),
            author: Some("Ken".to_string()),
            language: Some("ja-JP".to_string()),
            output_preset: Some("both".to_string()),
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        let series = fs::read_to_string(root.join("series.yml")).unwrap();
        assert!(series.contains("title: \"Custom Series\""));
        assert!(series.contains("language: ja-JP"));
        assert!(series.contains("id: pilot"));
        assert!(series.contains("path: books/pilot"));
        assert!(series.contains("target: kindle-ja"));
        assert!(series.contains("target: print-jp-pdfx1a"));
        let book = fs::read_to_string(book_root.join("book.yml")).unwrap();
        assert!(book.contains("- \"Ken\""));
        assert!(book.contains("books/pilot/manuscript/01-chapter-1.md"));
        let project_skill = read_skill(&root, "shosei-project");
        assert!(project_skill.contains("shosei explain --book pilot"));
        let content_review_skill = read_skill(&root, "shosei-content-review");
        assert!(content_review_skill.contains("shosei story check --book pilot"));
        assert!(content_review_skill.contains("shosei reference map --book pilot"));
        assert!(content_review_skill.contains("shosei reference map --shared"));
        assert!(content_review_skill.contains("shosei reference check --book pilot"));
        assert!(content_review_skill.contains("shosei reference check --shared"));
        assert!(content_review_skill.contains("shosei reference drift --book pilot"));
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
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
        assert!(result.summary.contains("config reference:"));
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
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("print".to_string()),
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        let print_css = fs::read_to_string(root.join("styles/print.css")).unwrap();
        assert!(print_css.contains("font-size: 10pt"));
        assert!(print_css.contains("h1 {\n  font-size: 1.45em;"));
        assert!(print_css.contains("nav#TOC a"));
        assert!(!print_css.contains("page-break-after: always;"));
    }

    #[test]
    fn applies_layout_print_and_git_overrides_to_scaffold() {
        let root = temp_dir("interactive-overrides");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("novel".to_string()),
            config_profile: None,
            repo_mode: Some("single-book".to_string()),
            initial_series_book_id: None,
            title: Some("Horizontal Novel".to_string()),
            author: Some("Ken".to_string()),
            language: Some("ja".to_string()),
            output_preset: Some("print".to_string()),
            writing_mode: Some("horizontal-ltr".to_string()),
            binding: Some("left".to_string()),
            print_target: Some("print-jp-pdfx4".to_string()),
            print_trim_size: Some("A5".to_string()),
            print_bleed: Some("5mm".to_string()),
            print_crop_marks: Some(false),
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: Some(false),
            generate_sample: Some(false),
        })
        .unwrap();

        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("writing_mode: horizontal-ltr"));
        assert!(book.contains("reading_direction: ltr"));
        assert!(book.contains("binding: left"));
        assert!(book.contains("target: print-jp-pdfx4"));
        assert!(book.contains("trim_size: A5"));
        assert!(book.contains("bleed: 5mm"));
        assert!(book.contains("crop_marks: false"));
        assert!(book.contains("engine: weasyprint"));
        assert!(book.contains("git:\n  lfs: false"));
        assert_eq!(
            fs::read_to_string(root.join("manuscript/01-chapter-1.md")).unwrap(),
            ""
        );
        assert!(!root.join(".gitattributes").exists());
        let base_css = fs::read_to_string(root.join("styles/base.css")).unwrap();
        assert!(base_css.contains("writing-mode: horizontal-tb"));
        assert!(base_css.contains("direction: ltr"));
    }

    #[test]
    fn initializes_git_repository_when_requested() {
        let root = temp_dir("git-init-requested");
        let result = init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("business".to_string()),
            config_profile: None,
            repo_mode: Some("single-book".to_string()),
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: None,
            writing_mode: None,
            binding: None,
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: true,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        assert!(root.join(".git").exists());
        assert!(result.summary.contains("initialized Git repository"));
    }

    #[test]
    fn applies_manga_branching_overrides_to_scaffold() {
        let root = temp_dir("manga-overrides");
        let book_root = root.join("books/pilot");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("manga".to_string()),
            config_profile: None,
            repo_mode: Some("series".to_string()),
            initial_series_book_id: Some("pilot".to_string()),
            title: Some("Pilot Volume".to_string()),
            author: Some("Author".to_string()),
            language: Some("ja".to_string()),
            output_preset: Some("both".to_string()),
            writing_mode: Some("vertical-rl".to_string()),
            binding: Some("right".to_string()),
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: Some("single-page".to_string()),
            manga_front_color_pages: Some(4),
            manga_body_mode: Some("mixed".to_string()),
            initialize_git: false,
            git_lfs: Some(true),
            generate_sample: None,
        })
        .unwrap();

        let series = fs::read_to_string(root.join("series.yml")).unwrap();
        assert!(series.contains("target: kindle-comic"));
        assert!(series.contains("target: print-manga"));
        assert!(series.contains("lfs: true"));
        let book = fs::read_to_string(book_root.join("book.yml")).unwrap();
        assert!(book.contains("spread_policy_for_kindle: single-page"));
        assert!(book.contains("front_color_pages: 4"));
        assert!(book.contains("body_mode: mixed"));
    }

    #[test]
    fn vertical_paper_print_defaults_to_chromium() {
        let root = temp_dir("vertical-paper-print");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("paper".to_string()),
            config_profile: Some("paper".to_string()),
            repo_mode: Some("single-book".to_string()),
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("print".to_string()),
            writing_mode: Some("vertical-rl".to_string()),
            binding: Some("right".to_string()),
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: None,
            print_max_pages: None,
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("writing_mode: vertical-rl"));
        assert!(book.contains("engine: chromium"));
    }

    #[test]
    fn conference_preprint_stays_on_weasyprint_even_if_vertical_is_requested() {
        let root = temp_dir("vertical-conference-preprint");
        init_project(InitProjectOptions {
            root: root.clone(),
            non_interactive: false,
            force: false,
            config_template: Some("paper".to_string()),
            config_profile: Some("conference-preprint".to_string()),
            repo_mode: Some("single-book".to_string()),
            initial_series_book_id: None,
            title: None,
            author: None,
            language: None,
            output_preset: Some("print".to_string()),
            writing_mode: Some("vertical-rl".to_string()),
            binding: Some("right".to_string()),
            print_target: None,
            print_trim_size: None,
            print_bleed: None,
            print_crop_marks: None,
            print_sides: Some("duplex".to_string()),
            print_max_pages: Some(2),
            manga_spread_policy_for_kindle: None,
            manga_front_color_pages: None,
            manga_body_mode: None,
            initialize_git: false,
            git_lfs: None,
            generate_sample: None,
        })
        .unwrap();

        let book = fs::read_to_string(root.join("book.yml")).unwrap();
        assert!(book.contains("profile: conference-preprint"));
        assert!(book.contains("engine: weasyprint"));
    }
}
