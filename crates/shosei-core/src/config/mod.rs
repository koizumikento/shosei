use std::{fs, path::Path};

use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::{
    domain::{ProjectType, RepoContext, RepoMode, RepoPath, RepoPathError},
    fs::join_repo_path,
};

#[derive(Debug, Clone)]
pub struct BookConfig {
    pub path: std::path::PathBuf,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct SeriesConfig {
    pub path: std::path::PathBuf,
    pub raw: Value,
}

#[derive(Debug, Clone, Default)]
pub struct SharedPaths {
    pub assets: Vec<RepoPath>,
    pub styles: Vec<RepoPath>,
    pub fonts: Vec<RepoPath>,
    pub metadata: Vec<RepoPath>,
}

#[derive(Debug, Clone)]
pub struct ResolvedBookConfig {
    pub repo: RepoContext,
    pub raw: Value,
    pub effective: EffectiveBookConfig,
    pub shared: SharedPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBookConfig {
    pub project: ProjectSettings,
    pub book: BookSettings,
    pub layout: LayoutSettings,
    pub outputs: OutputSettings,
    pub manga: Option<MangaSettings>,
    pub manuscript: Option<ManuscriptSettings>,
    pub validation: ValidationSettings,
    pub git: GitSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSettings {
    pub project_type: ProjectType,
    pub vcs: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSettings {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub profile: String,
    pub writing_mode: WritingMode,
    pub reading_direction: ReadingDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritingMode {
    HorizontalLtr,
    VerticalRl,
}

impl WritingMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "horizontal-ltr" => Some(Self::HorizontalLtr),
            "vertical-rl" => Some(Self::VerticalRl),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingDirection {
    Ltr,
    Rtl,
}

impl ReadingDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Left,
    Right,
}

impl Binding {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSettings {
    pub binding: Binding,
    pub chapter_start_page: String,
    pub allow_blank_pages: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSettings {
    pub kindle: Option<String>,
    pub print: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaSettings {
    pub reading_direction: ReadingDirection,
    pub default_page_side: MangaPageSide,
    pub page_width: String,
    pub page_height: String,
    pub spread_policy_for_kindle: SpreadPolicyForKindle,
    pub front_color_pages: u64,
    pub body_mode: MangaBodyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaPageSide {
    Left,
    Right,
}

impl MangaPageSide {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadPolicyForKindle {
    Split,
    SinglePage,
    Skip,
}

impl SpreadPolicyForKindle {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "split" => Some(Self::Split),
            "single-page" => Some(Self::SinglePage),
            "skip" => Some(Self::Skip),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaBodyMode {
    Monochrome,
    Color,
    Mixed,
}

impl MangaBodyMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "monochrome" => Some(Self::Monochrome),
            "color" => Some(Self::Color),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManuscriptSettings {
    pub frontmatter: Vec<RepoPath>,
    pub chapters: Vec<RepoPath>,
    pub backmatter: Vec<RepoPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSettings {
    pub strict: bool,
    pub epubcheck: bool,
    pub accessibility: String,
    pub missing_image: ValidationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warn,
    Error,
}

impl ValidationSeverity {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSettings {
    pub lfs: bool,
    pub require_clean_worktree_for_handoff: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML in {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("top-level YAML document in {path} must be a mapping")]
    NotMapping { path: String },
    #[error("path `{value}` in {path} must use repo-relative '/' separators")]
    InvalidRepoPath {
        path: String,
        value: String,
        #[source]
        source: RepoPathError,
    },
    #[error("series config {path} must define a books entry for `{book_id}`")]
    MissingSeriesBook { path: String, book_id: String },
    #[error("series config {path} points `{book_id}` to `{book_path}`, but book.yml was not found")]
    MissingSeriesBookConfig {
        path: String,
        book_id: String,
        book_path: String,
    },
    #[error("missing required field `{field}` in {path}")]
    MissingField { path: String, field: String },
    #[error("field `{field}` in {path} must be {expected}")]
    InvalidFieldType {
        path: String,
        field: String,
        expected: &'static str,
    },
    #[error("field `{field}` in {path} has invalid value `{value}`: {reason}")]
    InvalidFieldValue {
        path: String,
        field: String,
        value: String,
        reason: &'static str,
    },
    #[error("config in {path} must enable at least one output")]
    NoEnabledOutputs { path: String },
}

impl ResolvedBookConfig {
    pub fn has_path(&self, path: &[&str]) -> bool {
        lookup(&self.raw, path).is_some()
    }

    pub fn outputs(&self) -> Vec<String> {
        let mut outputs = Vec::new();
        if let Some(target) = &self.effective.outputs.kindle {
            outputs.push(target.clone());
        }
        if let Some(target) = &self.effective.outputs.print {
            outputs.push(target.clone());
        }
        outputs
    }

    pub fn manuscript_files(&self) -> Vec<RepoPath> {
        let mut files = Vec::new();
        if let Some(manuscript) = &self.effective.manuscript {
            files.extend(manuscript.frontmatter.clone());
            files.extend(manuscript.chapters.clone());
            files.extend(manuscript.backmatter.clone());
        }
        files
    }
}

pub fn load_book_config(path: &Path) -> Result<BookConfig, ConfigError> {
    Ok(BookConfig {
        path: path.to_path_buf(),
        raw: load_yaml_mapping(path)?,
    })
}

pub fn load_series_config(path: &Path) -> Result<SeriesConfig, ConfigError> {
    Ok(SeriesConfig {
        path: path.to_path_buf(),
        raw: load_yaml_mapping(path)?,
    })
}

pub fn resolve_book_config(context: &RepoContext) -> Result<ResolvedBookConfig, ConfigError> {
    let book = context
        .book
        .as_ref()
        .expect("book context must be selected before config resolution");

    let book_config = load_book_config(&book.config_path)?;

    let (raw, effective, shared) = match context.mode {
        RepoMode::SingleBook => {
            validate_repo_paths(&book_config.raw, &book_config.path)?;
            (
                book_config.raw.clone(),
                parse_effective_book_config(&book_config.raw, &book_config.path)?,
                SharedPaths::default(),
            )
        }
        RepoMode::Series => {
            let series_path = context.repo_root.join("series.yml");
            let series_config = load_series_config(&series_path)?;
            validate_repo_paths(&series_config.raw, &series_config.path)?;
            validate_repo_paths(&book_config.raw, &book_config.path)?;

            let book_entry = series_book_entry(&series_config, &book.id)?;
            let expected_root = join_repo_path(&context.repo_root, &book_entry.path);
            let expected_book_config = expected_root.join("book.yml");
            if !expected_book_config.is_file() {
                return Err(ConfigError::MissingSeriesBookConfig {
                    path: series_config.path.display().to_string(),
                    book_id: book.id.clone(),
                    book_path: book_entry.path.to_string(),
                });
            }

            let merged = merge_values(
                &series_defaults_root(&series_config.raw),
                &book_config.raw.clone(),
            );
            (
                merged.clone(),
                parse_effective_book_config(&merged, &book_config.path)?,
                shared_paths(&series_config.raw, &series_config.path)?,
            )
        }
    };

    Ok(ResolvedBookConfig {
        repo: context.clone(),
        raw,
        effective,
        shared,
    })
}

#[derive(Debug, Clone)]
struct SeriesBookEntry {
    path: RepoPath,
}

fn load_yaml_mapping(path: &Path) -> Result<Value, ConfigError> {
    let display = path.display().to_string();
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: display.clone(),
        source,
    })?;
    let value: Value = serde_yaml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: display.clone(),
        source,
    })?;
    if !matches!(value, Value::Mapping(_)) {
        return Err(ConfigError::NotMapping { path: display });
    }
    Ok(value)
}

fn validate_repo_paths(raw: &Value, config_path: &Path) -> Result<(), ConfigError> {
    let _ = parse_repo_path_values(raw, &["manuscript", "frontmatter"], config_path)?;
    let _ = parse_repo_path_values(raw, &["manuscript", "chapters"], config_path)?;
    let _ = parse_repo_path_values(raw, &["manuscript", "backmatter"], config_path)?;
    let _ = parse_repo_path_values(raw, &["shared", "assets"], config_path)?;
    let _ = parse_repo_path_values(raw, &["shared", "styles"], config_path)?;
    let _ = parse_repo_path_values(raw, &["shared", "fonts"], config_path)?;
    let _ = parse_repo_path_values(raw, &["shared", "metadata"], config_path)?;
    let _ = parse_repo_path_values(raw, &["git", "lockable"], config_path)?;

    if let Some(books) = lookup(raw, &["books"]).and_then(Value::as_sequence) {
        for book in books {
            if let Some(path) = lookup(book, &["path"]).and_then(Value::as_str) {
                let _ = parse_repo_path(config_path, path)?;
            }
        }
    }

    Ok(())
}

fn shared_paths(raw: &Value, config_path: &Path) -> Result<SharedPaths, ConfigError> {
    Ok(SharedPaths {
        assets: parse_repo_path_values(raw, &["shared", "assets"], config_path)?,
        styles: parse_repo_path_values(raw, &["shared", "styles"], config_path)?,
        fonts: parse_repo_path_values(raw, &["shared", "fonts"], config_path)?,
        metadata: parse_repo_path_values(raw, &["shared", "metadata"], config_path)?,
    })
}

fn parse_effective_book_config(
    raw: &Value,
    config_path: &Path,
) -> Result<EffectiveBookConfig, ConfigError> {
    let project_type = parse_project_type(raw, config_path)?;
    let writing_mode = parse_writing_mode(raw, config_path, project_type)?;
    let (reading_direction, reading_direction_explicit) =
        parse_reading_direction(raw, config_path, writing_mode)?;
    let outputs = parse_outputs(raw, config_path, project_type)?;
    if outputs.kindle.is_some() && !reading_direction_explicit {
        return Err(missing_field(config_path, "book.reading_direction"));
    }
    let manga = parse_manga(raw, config_path, project_type, reading_direction)?;

    Ok(EffectiveBookConfig {
        project: ProjectSettings {
            project_type,
            vcs: required_string_at(raw, &["project", "vcs"], config_path)?
                .unwrap_or_else(|| "git".to_string()),
            version: optional_u64_at(raw, &["project", "version"], config_path)?.unwrap_or(1),
        },
        book: BookSettings {
            title: required_string_at(raw, &["book", "title"], config_path)?
                .ok_or_else(|| missing_field(config_path, "book.title"))?,
            authors: parse_authors(raw, config_path)?,
            language: optional_string_at(raw, &["book", "language"], config_path)?
                .unwrap_or_else(|| "ja".to_string()),
            profile: parse_profile(raw, config_path, project_type)?,
            writing_mode,
            reading_direction,
        },
        layout: LayoutSettings {
            binding: parse_binding(raw, config_path, writing_mode)?,
            chapter_start_page: optional_string_at(
                raw,
                &["layout", "chapter_start_page"],
                config_path,
            )?
            .unwrap_or_else(|| "any".to_string()),
            allow_blank_pages: optional_bool_at(
                raw,
                &["layout", "allow_blank_pages"],
                config_path,
            )?
            .unwrap_or(true),
        },
        outputs,
        manga,
        manuscript: parse_manuscript(raw, config_path, project_type)?,
        validation: ValidationSettings {
            strict: optional_bool_at(raw, &["validation", "strict"], config_path)?.unwrap_or(true),
            epubcheck: optional_bool_at(raw, &["validation", "epubcheck"], config_path)?
                .unwrap_or(true),
            accessibility: optional_string_at(raw, &["validation", "accessibility"], config_path)?
                .unwrap_or_else(|| "warn".to_string()),
            missing_image: parse_validation_severity(
                raw,
                config_path,
                "validation.missing_image",
                &["validation", "missing_image"],
                ValidationSeverity::Error,
            )?,
        },
        git: GitSettings {
            lfs: optional_bool_at(raw, &["git", "lfs"], config_path)?.unwrap_or(true),
            require_clean_worktree_for_handoff: optional_bool_at(
                raw,
                &["git", "require_clean_worktree_for_handoff"],
                config_path,
            )?
            .unwrap_or(true),
        },
    })
}

fn parse_project_type(raw: &Value, config_path: &Path) -> Result<ProjectType, ConfigError> {
    let value = required_string_at(raw, &["project", "type"], config_path)?
        .ok_or_else(|| missing_field(config_path, "project.type"))?;
    ProjectType::parse(&value).ok_or_else(|| {
        invalid_value(
            config_path,
            "project.type",
            value,
            "must be one of business, novel, light-novel, manga",
        )
    })
}

fn parse_profile(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<String, ConfigError> {
    let profile = optional_string_at(raw, &["book", "profile"], config_path)?
        .unwrap_or_else(|| project_type.as_str().to_string());
    let allowed = ["business", "novel", "light-novel", "manga"];
    if !allowed.contains(&profile.as_str()) {
        return Err(invalid_value(
            config_path,
            "book.profile",
            profile,
            "must be one of business, novel, light-novel, manga",
        ));
    }
    if profile == "manga" && project_type != ProjectType::Manga {
        return Err(invalid_value(
            config_path,
            "book.profile",
            profile,
            "profile manga is only allowed when project.type is manga",
        ));
    }
    Ok(profile)
}

fn parse_writing_mode(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<WritingMode, ConfigError> {
    let default = if matches!(project_type, ProjectType::Business) {
        WritingMode::HorizontalLtr
    } else {
        WritingMode::VerticalRl
    };
    let value = optional_string_at(raw, &["book", "writing_mode"], config_path)?;
    match value {
        Some(value) => WritingMode::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "book.writing_mode",
                value,
                "must be horizontal-ltr or vertical-rl",
            )
        }),
        None => Ok(default),
    }
}

fn parse_reading_direction(
    raw: &Value,
    config_path: &Path,
    writing_mode: WritingMode,
) -> Result<(ReadingDirection, bool), ConfigError> {
    let default = match writing_mode {
        WritingMode::HorizontalLtr => ReadingDirection::Ltr,
        WritingMode::VerticalRl => ReadingDirection::Rtl,
    };
    let value = optional_string_at(raw, &["book", "reading_direction"], config_path)?;
    match value {
        Some(value) => ReadingDirection::parse(&value)
            .map(|reading_direction| (reading_direction, true))
            .ok_or_else(|| {
                invalid_value(
                    config_path,
                    "book.reading_direction",
                    value,
                    "must be ltr or rtl",
                )
            }),
        None => Ok((default, false)),
    }
}

fn parse_binding(
    raw: &Value,
    config_path: &Path,
    writing_mode: WritingMode,
) -> Result<Binding, ConfigError> {
    let default = match writing_mode {
        WritingMode::HorizontalLtr => Binding::Left,
        WritingMode::VerticalRl => Binding::Right,
    };
    let value = optional_string_at(raw, &["layout", "binding"], config_path)?;
    match value {
        Some(value) => Binding::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "layout.binding",
                value,
                "must be left or right",
            )
        }),
        None => Ok(default),
    }
}

fn parse_validation_severity(
    raw: &Value,
    config_path: &Path,
    field: &str,
    path: &[&str],
    default: ValidationSeverity,
) -> Result<ValidationSeverity, ConfigError> {
    match optional_string_at(raw, path, config_path)? {
        Some(value) => ValidationSeverity::parse(&value)
            .ok_or_else(|| invalid_value(config_path, field, value, "must be warn or error")),
        None => Ok(default),
    }
}

fn parse_authors(raw: &Value, config_path: &Path) -> Result<Vec<String>, ConfigError> {
    let authors = string_list_at(raw, &["book", "authors"], config_path)?;
    if authors.is_empty() {
        return Err(invalid_value(
            config_path,
            "book.authors",
            "[]".to_string(),
            "must contain at least one author",
        ));
    }
    Ok(authors)
}

fn parse_outputs(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<OutputSettings, ConfigError> {
    let kindle = parse_output_target(
        raw,
        config_path,
        "kindle",
        &["kindle-ja", "kindle-comic"],
        match project_type {
            ProjectType::Manga => Some("kindle-comic"),
            _ => Some("kindle-ja"),
        },
    )?;
    let print = parse_output_target(
        raw,
        config_path,
        "print",
        &["print-jp-pdfx1a", "print-jp-pdfx4", "print-manga"],
        match project_type {
            ProjectType::Manga => Some("print-manga"),
            _ => Some("print-jp-pdfx1a"),
        },
    )?;

    if kindle.is_none() && print.is_none() {
        return Err(ConfigError::NoEnabledOutputs {
            path: config_path.display().to_string(),
        });
    }

    if let Some(target) = &kindle
        && project_type != ProjectType::Manga
        && target == "kindle-comic"
    {
        return Err(invalid_value(
            config_path,
            "outputs.kindle.target",
            target.clone(),
            "kindle-comic is only allowed for manga projects",
        ));
    }
    if let Some(target) = &print
        && project_type != ProjectType::Manga
        && target == "print-manga"
    {
        return Err(invalid_value(
            config_path,
            "outputs.print.target",
            target.clone(),
            "print-manga is only allowed for manga projects",
        ));
    }

    Ok(OutputSettings { kindle, print })
}

fn parse_output_target(
    raw: &Value,
    config_path: &Path,
    output_name: &str,
    allowed: &[&str],
    default_target: Option<&str>,
) -> Result<Option<String>, ConfigError> {
    let enabled =
        optional_bool_at(raw, &["outputs", output_name, "enabled"], config_path)?.unwrap_or(false);
    if !enabled {
        return Ok(None);
    }

    let field = format!("outputs.{output_name}.target");
    let target = optional_string_at(raw, &["outputs", output_name, "target"], config_path)?
        .or_else(|| default_target.map(str::to_string))
        .ok_or_else(|| missing_field(config_path, &field))?;
    if !allowed.contains(&target.as_str()) {
        return Err(invalid_value(
            config_path,
            &field,
            target,
            "must be a supported output target",
        ));
    }
    Ok(Some(target))
}

fn parse_manuscript(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<Option<ManuscriptSettings>, ConfigError> {
    if !project_type.is_prose() {
        return Ok(None);
    }

    let frontmatter = parse_repo_path_values(raw, &["manuscript", "frontmatter"], config_path)?;
    validate_markdown_paths(&frontmatter, config_path, "manuscript.frontmatter")?;

    let chapters = parse_repo_path_values(raw, &["manuscript", "chapters"], config_path)?;
    if chapters.is_empty() {
        return Err(missing_field(config_path, "manuscript.chapters"));
    }
    validate_markdown_paths(&chapters, config_path, "manuscript.chapters")?;

    let backmatter = parse_repo_path_values(raw, &["manuscript", "backmatter"], config_path)?;
    validate_markdown_paths(&backmatter, config_path, "manuscript.backmatter")?;

    Ok(Some(ManuscriptSettings {
        frontmatter,
        chapters,
        backmatter,
    }))
}

fn parse_manga(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
    book_reading_direction: ReadingDirection,
) -> Result<Option<MangaSettings>, ConfigError> {
    if project_type != ProjectType::Manga {
        return Ok(None);
    }

    match lookup(raw, &["manga"]) {
        Some(Value::Mapping(_)) => {}
        Some(_) => return Err(invalid_type(config_path, "manga".to_string(), "a mapping")),
        None => return Err(missing_field(config_path, "manga")),
    }

    let reading_direction_value =
        required_string_at(raw, &["manga", "reading_direction"], config_path)?
            .ok_or_else(|| missing_field(config_path, "manga.reading_direction"))?;
    let reading_direction = ReadingDirection::parse(&reading_direction_value).ok_or_else(|| {
        invalid_value(
            config_path,
            "manga.reading_direction",
            reading_direction_value.clone(),
            "must be ltr or rtl",
        )
    })?;
    if reading_direction != book_reading_direction {
        return Err(invalid_value(
            config_path,
            "manga.reading_direction",
            reading_direction_value,
            "must match book.reading_direction for manga projects",
        ));
    }

    Ok(Some(MangaSettings {
        reading_direction,
        default_page_side: parse_manga_page_side(raw, config_path)?,
        page_width: parse_length_or_auto(
            raw,
            config_path,
            "manga.page_width",
            &["manga", "page_width"],
        )?
        .unwrap_or_else(|| "auto".to_string()),
        page_height: parse_length_or_auto(
            raw,
            config_path,
            "manga.page_height",
            &["manga", "page_height"],
        )?
        .unwrap_or_else(|| "auto".to_string()),
        spread_policy_for_kindle: parse_spread_policy_for_kindle(raw, config_path)?,
        front_color_pages: optional_u64_at(raw, &["manga", "front_color_pages"], config_path)?
            .unwrap_or(0),
        body_mode: parse_manga_body_mode(raw, config_path)?,
    }))
}

fn parse_manga_page_side(raw: &Value, config_path: &Path) -> Result<MangaPageSide, ConfigError> {
    match optional_string_at(raw, &["manga", "default_page_side"], config_path)? {
        Some(value) => MangaPageSide::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "manga.default_page_side",
                value,
                "must be left or right",
            )
        }),
        None => Ok(MangaPageSide::Right),
    }
}

fn parse_spread_policy_for_kindle(
    raw: &Value,
    config_path: &Path,
) -> Result<SpreadPolicyForKindle, ConfigError> {
    match optional_string_at(raw, &["manga", "spread_policy_for_kindle"], config_path)? {
        Some(value) => SpreadPolicyForKindle::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "manga.spread_policy_for_kindle",
                value,
                "must be split, single-page, or skip",
            )
        }),
        None => Ok(SpreadPolicyForKindle::Split),
    }
}

fn parse_manga_body_mode(raw: &Value, config_path: &Path) -> Result<MangaBodyMode, ConfigError> {
    match optional_string_at(raw, &["manga", "body_mode"], config_path)? {
        Some(value) => MangaBodyMode::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "manga.body_mode",
                value,
                "must be monochrome, color, or mixed",
            )
        }),
        None => Ok(MangaBodyMode::Monochrome),
    }
}

fn parse_length_or_auto(
    raw: &Value,
    config_path: &Path,
    field: &str,
    path: &[&str],
) -> Result<Option<String>, ConfigError> {
    match optional_string_at(raw, path, config_path)? {
        Some(value) if value == "auto" || !value.trim().is_empty() => Ok(Some(value)),
        Some(value) => Err(invalid_value(
            config_path,
            field,
            value,
            "must be auto or a non-empty CSS length string",
        )),
        None => Ok(None),
    }
}

fn validate_markdown_paths(
    paths: &[RepoPath],
    config_path: &Path,
    field: &str,
) -> Result<(), ConfigError> {
    for path in paths {
        if !path.as_str().ends_with(".md") {
            return Err(invalid_value(
                config_path,
                field,
                path.as_str().to_string(),
                "must reference a .md file in v0.1",
            ));
        }
    }
    Ok(())
}

fn series_defaults_root(raw: &Value) -> Value {
    let mut merged = Mapping::new();
    if let Some(defaults) = lookup(raw, &["defaults"]).and_then(Value::as_mapping) {
        for (key, value) in defaults {
            merged.insert(key.clone(), value.clone());
        }
    }
    if let Some(validation) = lookup(raw, &["validation"]) {
        merged.insert(Value::String("validation".to_string()), validation.clone());
    }
    if let Some(git) = lookup(raw, &["git"]) {
        merged.insert(Value::String("git".to_string()), git.clone());
    }
    Value::Mapping(merged)
}

fn series_book_entry(series: &SeriesConfig, book_id: &str) -> Result<SeriesBookEntry, ConfigError> {
    let books = lookup(&series.raw, &["books"])
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten();

    for book in books {
        let id = lookup(book, &["id"]).and_then(Value::as_str);
        if id == Some(book_id) {
            let path = lookup(book, &["path"])
                .and_then(Value::as_str)
                .ok_or_else(|| ConfigError::MissingSeriesBook {
                    path: series.path.display().to_string(),
                    book_id: book_id.to_string(),
                })?;
            return Ok(SeriesBookEntry {
                path: parse_repo_path(&series.path, path)?,
            });
        }
    }

    Err(ConfigError::MissingSeriesBook {
        path: series.path.display().to_string(),
        book_id: book_id.to_string(),
    })
}

fn parse_repo_path_values(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Vec<RepoPath>, ConfigError> {
    lookup(raw, path)
        .and_then(Value::as_sequence)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(|value| parse_repo_path(config_path, value))
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_repo_path(config_path: &Path, value: &str) -> Result<RepoPath, ConfigError> {
    RepoPath::parse(value.to_string()).map_err(|source| ConfigError::InvalidRepoPath {
        path: config_path.display().to_string(),
        value: value.to_string(),
        source,
    })
}

fn merge_values(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            let mut merged = base_map.clone();
            for (key, overlay_value) in overlay_map {
                if let Some(base_value) = merged.get(key) {
                    merged.insert(key.clone(), merge_values(base_value, overlay_value));
                } else {
                    merged.insert(key.clone(), overlay_value.clone());
                }
            }
            Value::Mapping(merged)
        }
        (_, other) => other.clone(),
    }
}

fn lookup<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        let mapping = current.as_mapping()?;
        current = mapping.get(Value::String((*segment).to_string()))?;
    }
    Some(current)
}

fn optional_string_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Option<String>, ConfigError> {
    match lookup(raw, path) {
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| invalid_type(config_path, path_label(path), "a string")),
        None => Ok(None),
    }
}

fn required_string_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Option<String>, ConfigError> {
    optional_string_at(raw, path, config_path)
}

fn optional_bool_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Option<bool>, ConfigError> {
    match lookup(raw, path) {
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| invalid_type(config_path, path_label(path), "a boolean")),
        None => Ok(None),
    }
}

fn optional_u64_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Option<u64>, ConfigError> {
    match lookup(raw, path) {
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| invalid_type(config_path, path_label(path), "a positive integer")),
        None => Ok(None),
    }
}

fn string_list_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Vec<String>, ConfigError> {
    match lookup(raw, path) {
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|item| {
                item.as_str().map(|value| value.to_string()).ok_or_else(|| {
                    invalid_type(config_path, path_label(path), "an array of strings")
                })
            })
            .collect(),
        Some(_) => Err(invalid_type(
            config_path,
            path_label(path),
            "an array of strings",
        )),
        None => Err(missing_field(config_path, &path_label(path))),
    }
}

fn path_label(path: &[&str]) -> String {
    path.join(".")
}

fn missing_field(config_path: &Path, field: &str) -> ConfigError {
    ConfigError::MissingField {
        path: config_path.display().to_string(),
        field: field.to_string(),
    }
}

fn invalid_type(config_path: &Path, field: String, expected: &'static str) -> ConfigError {
    ConfigError::InvalidFieldType {
        path: config_path.display().to_string(),
        field,
        expected,
    }
}

fn invalid_value(
    config_path: &Path,
    field: &str,
    value: String,
    reason: &'static str,
) -> ConfigError {
    ConfigError::InvalidFieldValue {
        path: config_path.display().to_string(),
        field: field.to_string(),
        value,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        domain::{BookContext, RepoMode},
        repo,
    };

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("shosei-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolves_single_book_outputs_and_manuscript_paths() {
        let root = temp_dir("single");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();

        assert_eq!(resolved.outputs(), vec!["kindle-ja"]);
        assert_eq!(resolved.manuscript_files()[0].as_str(), "manuscript/01.md");
        assert_eq!(resolved.effective.project.project_type, ProjectType::Novel);
    }

    #[test]
    fn merges_series_defaults_with_book_overrides() {
        let root = temp_dir("series");
        fs::create_dir_all(root.join("books/vol-01")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: sample
  title: Sample
  type: novel
defaults:
  book:
    language: ja
    writing_mode: vertical-rl
    reading_direction: rtl
  outputs:
    kindle:
      enabled: true
      target: kindle-ja
    print:
      enabled: true
      target: print-jp-pdfx1a
validation:
  strict: true
shared:
  assets:
    - shared/assets
books:
  - id: vol-01
    path: books/vol-01
"#,
        )
        .unwrap();
        fs::write(
            root.join("books/vol-01/book.yml"),
            r#"
project:
  type: novel
book:
  title: "Vol 1"
  authors:
    - "Author"
  language: en
outputs:
  print:
    enabled: false
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
"#,
        )
        .unwrap();

        let context = repo::discover(&root.join("books/vol-01"), None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();

        assert_eq!(resolved.effective.book.language, "en");
        assert_eq!(resolved.outputs(), vec!["kindle-ja"]);
        assert_eq!(resolved.shared.assets[0].as_str(), "shared/assets");
        assert!(resolved.effective.validation.strict);
    }

    #[test]
    fn rejects_series_book_without_catalog_entry() {
        let root = temp_dir("missing-series-book");
        fs::create_dir_all(root.join("books/vol-01")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: sample
  title: Sample
  type: novel
books: []
"#,
        )
        .unwrap();
        fs::write(
            root.join("books/vol-01/book.yml"),
            "project: { type: novel }\nbook: { title: Vol 1 }\n",
        )
        .unwrap();

        let context = RepoContext {
            repo_root: root.clone(),
            mode: RepoMode::Series,
            book: Some(BookContext {
                id: "vol-01".to_string(),
                root: root.join("books/vol-01"),
                config_path: root.join("books/vol-01/book.yml"),
            }),
        };
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(error, ConfigError::MissingSeriesBook { .. }));
    }

    #[test]
    fn rejects_prose_config_without_chapters() {
        let root = temp_dir("missing-chapters");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(
            matches!(error, ConfigError::MissingField { field, .. } if field == "manuscript.chapters")
        );
    }

    #[test]
    fn rejects_config_without_enabled_outputs() {
        let root = temp_dir("missing-outputs");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(error, ConfigError::NoEnabledOutputs { .. }));
    }

    #[test]
    fn rejects_non_markdown_manuscript_paths() {
        let root = temp_dir("non-markdown");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.txt
outputs:
  kindle:
    enabled: true
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::InvalidFieldValue { field, .. } if field == "manuscript.chapters"
        ));
    }

    #[test]
    fn rejects_kindle_config_without_explicit_reading_direction() {
        let root = temp_dir("missing-reading-direction");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
layout:
  binding: right
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::MissingField { field, .. } if field == "book.reading_direction"
        ));
    }

    #[test]
    fn resolves_manga_settings() {
        let root = temp_dir("manga-settings");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: left
  page_width: 148mm
  page_height: 210mm
  spread_policy_for_kindle: skip
  front_color_pages: 4
  body_mode: mixed
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();
        let manga = resolved.effective.manga.as_ref().unwrap();

        assert_eq!(manga.reading_direction, ReadingDirection::Rtl);
        assert_eq!(manga.default_page_side, MangaPageSide::Left);
        assert_eq!(manga.page_width, "148mm");
        assert_eq!(manga.page_height, "210mm");
        assert_eq!(manga.spread_policy_for_kindle, SpreadPolicyForKindle::Skip);
        assert_eq!(manga.front_color_pages, 4);
        assert_eq!(manga.body_mode, MangaBodyMode::Mixed);
    }

    #[test]
    fn rejects_manga_config_without_manga_section() {
        let root = temp_dir("missing-manga-section");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::MissingField { field, .. } if field == "manga"
        ));
    }

    #[test]
    fn rejects_manga_reading_direction_mismatch() {
        let root = temp_dir("manga-reading-direction-mismatch");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: ltr
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: monochrome
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::InvalidFieldValue { field, .. } if field == "manga.reading_direction"
        ));
    }

    #[test]
    fn rejects_invalid_manga_spread_policy() {
        let root = temp_dir("invalid-manga-spread-policy");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: foldout
  front_color_pages: 0
  body_mode: monochrome
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let error = resolve_book_config(&context).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::InvalidFieldValue { field, .. } if field == "manga.spread_policy_for_kindle"
        ));
    }
}
