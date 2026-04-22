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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValueOrigin {
    BookConfig,
    SeriesDefaults,
    BuiltInDefault,
}

impl std::fmt::Display for ConfigValueOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BookConfig => "book.yml",
            Self::SeriesDefaults => "series defaults",
            Self::BuiltInDefault => "built-in default",
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExplainedValue {
    pub field: String,
    pub value: String,
    pub origin: ConfigValueOrigin,
}

#[derive(Debug, Clone)]
pub struct ExplainedConfig {
    pub resolved: ResolvedBookConfig,
    pub values: Vec<ExplainedValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBookConfig {
    pub project: ProjectSettings,
    pub book: BookSettings,
    pub layout: LayoutSettings,
    pub cover: CoverSettings,
    pub pdf: Option<PdfSettings>,
    pub print: Option<PrintSettings>,
    pub outputs: OutputSettings,
    pub editorial: EditorialSettings,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverSettings {
    pub ebook_image: Option<RepoPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfSettings {
    pub engine: PdfEngine,
    pub toc: bool,
    pub page_number: bool,
    pub running_header: PdfRunningHeader,
    pub column_count: u64,
    pub column_gap: String,
    pub base_font_size: String,
    pub line_height: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfEngine {
    Weasyprint,
    Chromium,
    Typst,
    Lualatex,
}

impl PdfEngine {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "weasyprint" => Some(Self::Weasyprint),
            "chromium" => Some(Self::Chromium),
            "typst" => Some(Self::Typst),
            "lualatex" => Some(Self::Lualatex),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weasyprint => "weasyprint",
            Self::Chromium => "chromium",
            Self::Typst => "typst",
            Self::Lualatex => "lualatex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfRunningHeader {
    Auto,
    None,
    Title,
    Chapter,
}

impl PdfRunningHeader {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "none" => Some(Self::None),
            "title" => Some(Self::Title),
            "chapter" => Some(Self::Chapter),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::Title => "title",
            Self::Chapter => "chapter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintSettings {
    pub trim_size: PrintTrimSize,
    pub bleed: String,
    pub crop_marks: bool,
    pub page_margin: Option<PageMarginSettings>,
    pub sides: PrintSides,
    pub max_pages: Option<u64>,
    pub body_pdf: bool,
    pub cover_pdf: bool,
    pub pdf_standard: PrintPdfStandard,
    pub body_mode: PrintBodyMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageMarginSettings {
    pub top: String,
    pub bottom: String,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintTrimSize {
    A4,
    A5,
    B6,
    Bunko,
    Custom,
}

impl PrintTrimSize {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "A4" => Some(Self::A4),
            "A5" => Some(Self::A5),
            "B6" => Some(Self::B6),
            "bunko" => Some(Self::Bunko),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A5 => "A5",
            Self::B6 => "B6",
            Self::Bunko => "bunko",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintSides {
    Simplex,
    Duplex,
}

impl PrintSides {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "simplex" => Some(Self::Simplex),
            "duplex" => Some(Self::Duplex),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simplex => "simplex",
            Self::Duplex => "duplex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintPdfStandard {
    Pdfx1a,
    Pdfx4,
}

impl PrintPdfStandard {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "pdfx1a" => Some(Self::Pdfx1a),
            "pdfx4" => Some(Self::Pdfx4),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdfx1a => "pdfx1a",
            Self::Pdfx4 => "pdfx4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintBodyMode {
    Auto,
    Monochrome,
    Color,
}

impl PrintBodyMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "monochrome" => Some(Self::Monochrome),
            "color" => Some(Self::Color),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Monochrome => "monochrome",
            Self::Color => "color",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSettings {
    pub kindle: Option<String>,
    pub print: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorialSettings {
    pub style: Option<RepoPath>,
    pub claims: Option<RepoPath>,
    pub figures: Option<RepoPath>,
    pub freshness: Option<RepoPath>,
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
    pub kindle_previewer: bool,
    pub accessibility: ValidationLevel,
    pub missing_image: ValidationSeverity,
    pub missing_alt: ValidationSeverity,
    pub broken_link: ValidationSeverity,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    Off,
    Warn,
    Error,
}

impl ValidationLevel {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Error => "error",
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

pub fn explain_book_config(context: &RepoContext) -> Result<ExplainedConfig, ConfigError> {
    let resolved = resolve_book_config(context)?;
    let book = context
        .book
        .as_ref()
        .expect("book context must be selected before config explanation");
    let book_config = load_book_config(&book.config_path)?;

    let series_defaults = if context.mode == RepoMode::Series {
        let series_config = load_series_config(&context.repo_root.join("series.yml"))?;
        Some(series_defaults_root(&series_config.raw))
    } else {
        None
    };

    let origin =
        |path: &[&str]| config_value_origin(&book_config.raw, series_defaults.as_ref(), path);

    let mut values = vec![
        explained(
            "project.type",
            resolved.effective.project.project_type.as_str(),
            origin(&["project", "type"]),
        ),
        explained(
            "project.vcs",
            &resolved.effective.project.vcs,
            origin(&["project", "vcs"]),
        ),
        explained(
            "project.version",
            resolved.effective.project.version.to_string(),
            origin(&["project", "version"]),
        ),
        explained(
            "book.title",
            &resolved.effective.book.title,
            origin(&["book", "title"]),
        ),
        explained(
            "book.authors",
            format_list(&resolved.effective.book.authors),
            origin(&["book", "authors"]),
        ),
        explained(
            "book.language",
            &resolved.effective.book.language,
            origin(&["book", "language"]),
        ),
        explained(
            "book.profile",
            &resolved.effective.book.profile,
            origin(&["book", "profile"]),
        ),
        explained(
            "book.writing_mode",
            match resolved.effective.book.writing_mode {
                WritingMode::HorizontalLtr => "horizontal-ltr",
                WritingMode::VerticalRl => "vertical-rl",
            },
            origin(&["book", "writing_mode"]),
        ),
        explained(
            "book.reading_direction",
            resolved.effective.book.reading_direction.as_str(),
            origin(&["book", "reading_direction"]),
        ),
        explained(
            "layout.binding",
            match resolved.effective.layout.binding {
                Binding::Left => "left",
                Binding::Right => "right",
            },
            origin(&["layout", "binding"]),
        ),
        explained(
            "layout.chapter_start_page",
            &resolved.effective.layout.chapter_start_page,
            origin(&["layout", "chapter_start_page"]),
        ),
        explained(
            "layout.allow_blank_pages",
            resolved.effective.layout.allow_blank_pages.to_string(),
            origin(&["layout", "allow_blank_pages"]),
        ),
        explained_optional_repo_path(
            "cover.ebook_image",
            resolved.effective.cover.ebook_image.as_ref(),
            origin(&["cover", "ebook_image"]),
        ),
        explained_optional(
            "pdf.engine",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.engine.as_str()),
            origin(&["pdf", "engine"]),
            "n/a",
        ),
        explained_optional(
            "pdf.toc",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| if pdf.toc { "true" } else { "false" }),
            origin(&["pdf", "toc"]),
            "n/a",
        ),
        explained_optional(
            "pdf.page_number",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| if pdf.page_number { "true" } else { "false" }),
            origin(&["pdf", "page_number"]),
            "n/a",
        ),
        explained_optional(
            "pdf.running_header",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.running_header.as_str()),
            origin(&["pdf", "running_header"]),
            "n/a",
        ),
        explained_optional(
            "pdf.column_count",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.column_count.to_string())
                .as_deref(),
            origin(&["pdf", "column_count"]),
            "n/a",
        ),
        explained_optional(
            "pdf.column_gap",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.column_gap.as_str()),
            origin(&["pdf", "column_gap"]),
            "n/a",
        ),
        explained_optional(
            "pdf.base_font_size",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.base_font_size.as_str()),
            origin(&["pdf", "base_font_size"]),
            "n/a",
        ),
        explained_optional(
            "pdf.line_height",
            resolved
                .effective
                .pdf
                .as_ref()
                .map(|pdf| pdf.line_height.as_str()),
            origin(&["pdf", "line_height"]),
            "n/a",
        ),
        explained_output_target(
            "outputs.kindle.target",
            resolved.effective.outputs.kindle.as_deref(),
            output_origin(&book_config.raw, series_defaults.as_ref(), "kindle"),
        ),
        explained_output_target(
            "outputs.print.target",
            resolved.effective.outputs.print.as_deref(),
            output_origin(&book_config.raw, series_defaults.as_ref(), "print"),
        ),
        explained_optional(
            "print.trim_size",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| print.trim_size.as_str()),
            origin(&["print", "trim_size"]),
            "n/a",
        ),
        explained_optional(
            "print.bleed",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| print.bleed.as_str()),
            origin(&["print", "bleed"]),
            "n/a",
        ),
        explained_optional(
            "print.crop_marks",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| if print.crop_marks { "true" } else { "false" }),
            origin(&["print", "crop_marks"]),
            "n/a",
        ),
        explained_optional(
            "print.page_margin",
            resolved
                .effective
                .print
                .as_ref()
                .and_then(|print| print.page_margin.as_ref())
                .map(format_page_margin)
                .as_deref(),
            origin(&["print", "page_margin"]),
            "none",
        ),
        explained_optional(
            "print.sides",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| print.sides.as_str()),
            origin(&["print", "sides"]),
            "n/a",
        ),
        explained_optional(
            "print.max_pages",
            resolved
                .effective
                .print
                .as_ref()
                .and_then(|print| print.max_pages.map(|value| value.to_string()))
                .as_deref(),
            origin(&["print", "max_pages"]),
            "none",
        ),
        explained_optional(
            "print.body_pdf",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| if print.body_pdf { "true" } else { "false" }),
            origin(&["print", "body_pdf"]),
            "n/a",
        ),
        explained_optional(
            "print.cover_pdf",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| if print.cover_pdf { "true" } else { "false" }),
            origin(&["print", "cover_pdf"]),
            "n/a",
        ),
        explained_optional(
            "print.pdf_standard",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| print.pdf_standard.as_str()),
            origin(&["print", "pdf_standard"]),
            "n/a",
        ),
        explained_optional(
            "print.body_mode",
            resolved
                .effective
                .print
                .as_ref()
                .map(|print| print.body_mode.as_str()),
            origin(&["print", "body_mode"]),
            "n/a",
        ),
        explained_optional_repo_path(
            "editorial.style",
            resolved.effective.editorial.style.as_ref(),
            origin(&["editorial", "style"]),
        ),
        explained_optional_repo_path(
            "editorial.claims",
            resolved.effective.editorial.claims.as_ref(),
            origin(&["editorial", "claims"]),
        ),
        explained_optional_repo_path(
            "editorial.figures",
            resolved.effective.editorial.figures.as_ref(),
            origin(&["editorial", "figures"]),
        ),
        explained_optional_repo_path(
            "editorial.freshness",
            resolved.effective.editorial.freshness.as_ref(),
            origin(&["editorial", "freshness"]),
        ),
        explained(
            "validation.strict",
            resolved.effective.validation.strict.to_string(),
            origin(&["validation", "strict"]),
        ),
        explained(
            "validation.epubcheck",
            resolved.effective.validation.epubcheck.to_string(),
            origin(&["validation", "epubcheck"]),
        ),
        explained(
            "validation.kindle_previewer",
            resolved.effective.validation.kindle_previewer.to_string(),
            origin(&["validation", "kindle_previewer"]),
        ),
        explained(
            "validation.accessibility",
            resolved.effective.validation.accessibility.as_str(),
            origin(&["validation", "accessibility"]),
        ),
        explained(
            "validation.missing_image",
            match resolved.effective.validation.missing_image {
                ValidationSeverity::Warn => "warn",
                ValidationSeverity::Error => "error",
            },
            origin(&["validation", "missing_image"]),
        ),
        explained(
            "validation.missing_alt",
            match resolved.effective.validation.missing_alt {
                ValidationSeverity::Warn => "warn",
                ValidationSeverity::Error => "error",
            },
            origin(&["validation", "missing_alt"]),
        ),
        explained(
            "validation.broken_link",
            match resolved.effective.validation.broken_link {
                ValidationSeverity::Warn => "warn",
                ValidationSeverity::Error => "error",
            },
            origin(&["validation", "broken_link"]),
        ),
        explained(
            "git.lfs",
            resolved.effective.git.lfs.to_string(),
            origin(&["git", "lfs"]),
        ),
        explained(
            "git.require_clean_worktree_for_handoff",
            resolved
                .effective
                .git
                .require_clean_worktree_for_handoff
                .to_string(),
            origin(&["git", "require_clean_worktree_for_handoff"]),
        ),
    ];

    if let Some(manuscript) = &resolved.effective.manuscript {
        values.push(explained(
            "manuscript.frontmatter",
            format_repo_paths(&manuscript.frontmatter),
            origin(&["manuscript", "frontmatter"]),
        ));
        values.push(explained(
            "manuscript.chapters",
            format_repo_paths(&manuscript.chapters),
            origin(&["manuscript", "chapters"]),
        ));
        values.push(explained(
            "manuscript.backmatter",
            format_repo_paths(&manuscript.backmatter),
            origin(&["manuscript", "backmatter"]),
        ));
    }

    if let Some(manga) = &resolved.effective.manga {
        values.push(explained(
            "manga.reading_direction",
            manga.reading_direction.as_str(),
            origin(&["manga", "reading_direction"]),
        ));
        values.push(explained(
            "manga.default_page_side",
            match manga.default_page_side {
                MangaPageSide::Left => "left",
                MangaPageSide::Right => "right",
            },
            origin(&["manga", "default_page_side"]),
        ));
        values.push(explained(
            "manga.page_width",
            &manga.page_width,
            origin(&["manga", "page_width"]),
        ));
        values.push(explained(
            "manga.page_height",
            &manga.page_height,
            origin(&["manga", "page_height"]),
        ));
        values.push(explained(
            "manga.spread_policy_for_kindle",
            match manga.spread_policy_for_kindle {
                SpreadPolicyForKindle::Split => "split",
                SpreadPolicyForKindle::SinglePage => "single-page",
                SpreadPolicyForKindle::Skip => "skip",
            },
            origin(&["manga", "spread_policy_for_kindle"]),
        ));
        values.push(explained(
            "manga.front_color_pages",
            manga.front_color_pages.to_string(),
            origin(&["manga", "front_color_pages"]),
        ));
        values.push(explained(
            "manga.body_mode",
            match manga.body_mode {
                MangaBodyMode::Monochrome => "monochrome",
                MangaBodyMode::Color => "color",
                MangaBodyMode::Mixed => "mixed",
            },
            origin(&["manga", "body_mode"]),
        ));
    }

    Ok(ExplainedConfig { resolved, values })
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
    let _ = optional_repo_path_at(raw, &["editorial", "style"], config_path)?;
    let _ = optional_repo_path_at(raw, &["editorial", "claims"], config_path)?;
    let _ = optional_repo_path_at(raw, &["editorial", "figures"], config_path)?;
    let _ = optional_repo_path_at(raw, &["editorial", "freshness"], config_path)?;

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
    let profile = parse_profile(raw, config_path, project_type)?;
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
            profile: profile.clone(),
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
        cover: parse_cover(raw, config_path)?,
        pdf: parse_pdf(raw, config_path, project_type, &profile, writing_mode)?,
        print: parse_print(raw, config_path, outputs.print.as_deref())?,
        outputs,
        editorial: parse_editorial(raw, config_path)?,
        manga,
        manuscript: parse_manuscript(raw, config_path, project_type)?,
        validation: ValidationSettings {
            strict: optional_bool_at(raw, &["validation", "strict"], config_path)?.unwrap_or(true),
            epubcheck: optional_bool_at(raw, &["validation", "epubcheck"], config_path)?
                .unwrap_or(true),
            kindle_previewer: optional_bool_at(
                raw,
                &["validation", "kindle_previewer"],
                config_path,
            )?
            .unwrap_or(false),
            accessibility: parse_validation_level(
                raw,
                config_path,
                "validation.accessibility",
                &["validation", "accessibility"],
                ValidationLevel::Warn,
            )?,
            missing_image: parse_validation_severity(
                raw,
                config_path,
                "validation.missing_image",
                &["validation", "missing_image"],
                ValidationSeverity::Error,
            )?,
            missing_alt: parse_validation_severity(
                raw,
                config_path,
                "validation.missing_alt",
                &["validation", "missing_alt"],
                ValidationSeverity::Error,
            )?,
            broken_link: parse_validation_severity(
                raw,
                config_path,
                "validation.broken_link",
                &["validation", "broken_link"],
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
            "must be one of business, paper, novel, light-novel, manga",
        )
    })
}

fn parse_cover(raw: &Value, config_path: &Path) -> Result<CoverSettings, ConfigError> {
    Ok(CoverSettings {
        ebook_image: parse_cover_ebook_image(raw, config_path)?,
    })
}

fn parse_cover_ebook_image(
    raw: &Value,
    config_path: &Path,
) -> Result<Option<RepoPath>, ConfigError> {
    let Some(path) = optional_repo_path_at(raw, &["cover", "ebook_image"], config_path)? else {
        return Ok(None);
    };
    if !has_allowed_cover_extension(&path) {
        return Err(invalid_value(
            config_path,
            "cover.ebook_image",
            path.as_str().to_string(),
            "must reference a .jpg, .jpeg, or .png file in v0.1",
        ));
    }
    Ok(Some(path))
}

fn parse_pdf(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
    profile: &str,
    writing_mode: WritingMode,
) -> Result<Option<PdfSettings>, ConfigError> {
    if !project_type.is_prose() {
        return Ok(None);
    }

    Ok(Some(PdfSettings {
        engine: parse_pdf_engine(raw, config_path, profile, writing_mode)?,
        toc: optional_bool_at(raw, &["pdf", "toc"], config_path)?.unwrap_or(true),
        page_number: optional_bool_at(raw, &["pdf", "page_number"], config_path)?.unwrap_or(true),
        running_header: parse_pdf_running_header(raw, config_path)?,
        column_count: parse_positive_u64_field(
            raw,
            config_path,
            "pdf.column_count",
            &["pdf", "column_count"],
        )?
        .unwrap_or(1),
        column_gap: parse_length_or_auto(
            raw,
            config_path,
            "pdf.column_gap",
            &["pdf", "column_gap"],
        )?
        .unwrap_or_else(|| "auto".to_string()),
        base_font_size: parse_length_or_auto(
            raw,
            config_path,
            "pdf.base_font_size",
            &["pdf", "base_font_size"],
        )?
        .unwrap_or_else(|| "auto".to_string()),
        line_height: parse_length_or_auto(
            raw,
            config_path,
            "pdf.line_height",
            &["pdf", "line_height"],
        )?
        .unwrap_or_else(|| "auto".to_string()),
    }))
}

fn parse_pdf_engine(
    raw: &Value,
    config_path: &Path,
    profile: &str,
    writing_mode: WritingMode,
) -> Result<PdfEngine, ConfigError> {
    match optional_string_at(raw, &["pdf", "engine"], config_path)? {
        Some(value) => PdfEngine::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "pdf.engine",
                value,
                "must be weasyprint, chromium, typst, or lualatex",
            )
        }),
        None => Ok(default_pdf_engine(profile, writing_mode)),
    }
}

fn default_pdf_engine(profile: &str, writing_mode: WritingMode) -> PdfEngine {
    if profile == "conference-preprint" {
        PdfEngine::Weasyprint
    } else {
        match writing_mode {
            WritingMode::VerticalRl => PdfEngine::Chromium,
            WritingMode::HorizontalLtr => PdfEngine::Weasyprint,
        }
    }
}

fn parse_pdf_running_header(
    raw: &Value,
    config_path: &Path,
) -> Result<PdfRunningHeader, ConfigError> {
    match optional_string_at(raw, &["pdf", "running_header"], config_path)? {
        Some(value) => PdfRunningHeader::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "pdf.running_header",
                value,
                "must be auto, none, title, or chapter",
            )
        }),
        None => Ok(PdfRunningHeader::Auto),
    }
}

fn parse_profile(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<String, ConfigError> {
    let profile = optional_string_at(raw, &["book", "profile"], config_path)?
        .unwrap_or_else(|| project_type.as_str().to_string());
    let allowed = [
        "business",
        "paper",
        "conference-preprint",
        "novel",
        "light-novel",
        "manga",
    ];
    if !allowed.contains(&profile.as_str()) {
        return Err(invalid_value(
            config_path,
            "book.profile",
            profile,
            "must be one of business, paper, conference-preprint, novel, light-novel, manga",
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
    if profile == "conference-preprint" && project_type != ProjectType::Paper {
        return Err(invalid_value(
            config_path,
            "book.profile",
            profile,
            "profile conference-preprint is only allowed when project.type is paper",
        ));
    }
    Ok(profile)
}

fn parse_editorial(raw: &Value, config_path: &Path) -> Result<EditorialSettings, ConfigError> {
    Ok(EditorialSettings {
        style: optional_repo_path_at(raw, &["editorial", "style"], config_path)?,
        claims: optional_repo_path_at(raw, &["editorial", "claims"], config_path)?,
        figures: optional_repo_path_at(raw, &["editorial", "figures"], config_path)?,
        freshness: optional_repo_path_at(raw, &["editorial", "freshness"], config_path)?,
    })
}

fn parse_writing_mode(
    raw: &Value,
    config_path: &Path,
    project_type: ProjectType,
) -> Result<WritingMode, ConfigError> {
    let default = if matches!(project_type, ProjectType::Business | ProjectType::Paper) {
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

fn parse_print(
    raw: &Value,
    config_path: &Path,
    print_target: Option<&str>,
) -> Result<Option<PrintSettings>, ConfigError> {
    let print_section = lookup(raw, &["print"]);
    if print_target.is_none() && print_section.is_none() {
        return Ok(None);
    }

    if matches!(print_section, Some(value) if !matches!(value, Value::Mapping(_))) {
        return Err(invalid_type(config_path, "print".to_string(), "a mapping"));
    }

    let default_pdf_standard = match print_target {
        Some("print-jp-pdfx4") => PrintPdfStandard::Pdfx4,
        _ => PrintPdfStandard::Pdfx1a,
    };

    Ok(Some(PrintSettings {
        trim_size: parse_print_trim_size(raw, config_path)?,
        bleed: parse_length(raw, config_path, "print.bleed", &["print", "bleed"])?
            .unwrap_or_else(|| "3mm".to_string()),
        crop_marks: optional_bool_at(raw, &["print", "crop_marks"], config_path)?.unwrap_or(true),
        page_margin: parse_page_margin(raw, config_path)?,
        sides: parse_print_sides(raw, config_path)?,
        max_pages: parse_positive_u64_field(
            raw,
            config_path,
            "print.max_pages",
            &["print", "max_pages"],
        )?,
        body_pdf: optional_bool_at(raw, &["print", "body_pdf"], config_path)?.unwrap_or(true),
        cover_pdf: optional_bool_at(raw, &["print", "cover_pdf"], config_path)?.unwrap_or(false),
        pdf_standard: parse_print_pdf_standard(raw, config_path)?.unwrap_or(default_pdf_standard),
        body_mode: parse_print_body_mode(raw, config_path)?,
    }))
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

fn parse_validation_level(
    raw: &Value,
    config_path: &Path,
    field: &str,
    path: &[&str],
    default: ValidationLevel,
) -> Result<ValidationLevel, ConfigError> {
    match optional_string_at(raw, path, config_path)? {
        Some(value) => ValidationLevel::parse(&value)
            .ok_or_else(|| invalid_value(config_path, field, value, "must be off, warn, or error")),
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

fn parse_length(
    raw: &Value,
    config_path: &Path,
    field: &str,
    path: &[&str],
) -> Result<Option<String>, ConfigError> {
    match optional_string_at(raw, path, config_path)? {
        Some(value) if !value.trim().is_empty() => Ok(Some(value)),
        Some(value) => Err(invalid_value(
            config_path,
            field,
            value,
            "must be a non-empty CSS length string",
        )),
        None => Ok(None),
    }
}

fn parse_positive_u64_field(
    raw: &Value,
    config_path: &Path,
    field: &str,
    path: &[&str],
) -> Result<Option<u64>, ConfigError> {
    let Some(value) = optional_u64_at(raw, path, config_path)? else {
        return Ok(None);
    };
    if value == 0 {
        return Err(invalid_value(
            config_path,
            field,
            value.to_string(),
            "must be greater than 0",
        ));
    }
    Ok(Some(value))
}

fn parse_print_trim_size(raw: &Value, config_path: &Path) -> Result<PrintTrimSize, ConfigError> {
    match optional_string_at(raw, &["print", "trim_size"], config_path)? {
        Some(value) => PrintTrimSize::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "print.trim_size",
                value,
                "must be A4, A5, B6, bunko, or custom",
            )
        }),
        None => Ok(PrintTrimSize::A5),
    }
}

fn parse_print_sides(raw: &Value, config_path: &Path) -> Result<PrintSides, ConfigError> {
    match optional_string_at(raw, &["print", "sides"], config_path)? {
        Some(value) => PrintSides::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "print.sides",
                value,
                "must be simplex or duplex",
            )
        }),
        None => Ok(PrintSides::Simplex),
    }
}

fn parse_print_pdf_standard(
    raw: &Value,
    config_path: &Path,
) -> Result<Option<PrintPdfStandard>, ConfigError> {
    match optional_string_at(raw, &["print", "pdf_standard"], config_path)? {
        Some(value) => PrintPdfStandard::parse(&value).map(Some).ok_or_else(|| {
            invalid_value(
                config_path,
                "print.pdf_standard",
                value,
                "must be pdfx1a or pdfx4",
            )
        }),
        None => Ok(None),
    }
}

fn parse_print_body_mode(raw: &Value, config_path: &Path) -> Result<PrintBodyMode, ConfigError> {
    match optional_string_at(raw, &["print", "body_mode"], config_path)? {
        Some(value) => PrintBodyMode::parse(&value).ok_or_else(|| {
            invalid_value(
                config_path,
                "print.body_mode",
                value,
                "must be auto, monochrome, or color",
            )
        }),
        None => Ok(PrintBodyMode::Auto),
    }
}

fn parse_page_margin(
    raw: &Value,
    config_path: &Path,
) -> Result<Option<PageMarginSettings>, ConfigError> {
    match lookup(raw, &["print", "page_margin"]) {
        Some(Value::Mapping(_)) => Ok(Some(PageMarginSettings {
            top: required_string_at(raw, &["print", "page_margin", "top"], config_path)?
                .ok_or_else(|| missing_field(config_path, "print.page_margin.top"))?,
            bottom: required_string_at(raw, &["print", "page_margin", "bottom"], config_path)?
                .ok_or_else(|| missing_field(config_path, "print.page_margin.bottom"))?,
            left: required_string_at(raw, &["print", "page_margin", "left"], config_path)?
                .ok_or_else(|| missing_field(config_path, "print.page_margin.left"))?,
            right: required_string_at(raw, &["print", "page_margin", "right"], config_path)?
                .ok_or_else(|| missing_field(config_path, "print.page_margin.right"))?,
        })),
        Some(_) => Err(invalid_type(
            config_path,
            "print.page_margin".to_string(),
            "a mapping",
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
            if key.as_str() == Some("cover") {
                continue;
            }
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

fn optional_repo_path_at(
    raw: &Value,
    path: &[&str],
    config_path: &Path,
) -> Result<Option<RepoPath>, ConfigError> {
    optional_string_at(raw, path, config_path)?
        .map(|value| parse_repo_path(config_path, &value))
        .transpose()
}

fn config_value_origin(
    book_raw: &Value,
    series_defaults: Option<&Value>,
    path: &[&str],
) -> ConfigValueOrigin {
    if lookup(book_raw, path).is_some() {
        ConfigValueOrigin::BookConfig
    } else if series_defaults
        .and_then(|value| lookup(value, path))
        .is_some()
    {
        ConfigValueOrigin::SeriesDefaults
    } else {
        ConfigValueOrigin::BuiltInDefault
    }
}

fn output_origin(
    book_raw: &Value,
    series_defaults: Option<&Value>,
    output_name: &str,
) -> ConfigValueOrigin {
    let enabled_path = ["outputs", output_name, "enabled"];
    let target_path = ["outputs", output_name, "target"];
    if lookup(book_raw, &enabled_path).is_some() || lookup(book_raw, &target_path).is_some() {
        ConfigValueOrigin::BookConfig
    } else if series_defaults
        .and_then(|value| lookup(value, &enabled_path).or_else(|| lookup(value, &target_path)))
        .is_some()
    {
        ConfigValueOrigin::SeriesDefaults
    } else {
        ConfigValueOrigin::BuiltInDefault
    }
}

fn explained(
    field: impl Into<String>,
    value: impl Into<String>,
    origin: ConfigValueOrigin,
) -> ExplainedValue {
    ExplainedValue {
        field: field.into(),
        value: value.into(),
        origin,
    }
}

fn explained_output_target(
    field: impl Into<String>,
    value: Option<&str>,
    origin: ConfigValueOrigin,
) -> ExplainedValue {
    explained(field, value.unwrap_or("disabled"), origin)
}

fn explained_optional_repo_path(
    field: impl Into<String>,
    value: Option<&RepoPath>,
    origin: ConfigValueOrigin,
) -> ExplainedValue {
    explained(field, value.map(RepoPath::as_str).unwrap_or("none"), origin)
}

fn explained_optional(
    field: impl Into<String>,
    value: Option<&str>,
    origin: ConfigValueOrigin,
    missing: &'static str,
) -> ExplainedValue {
    explained(field, value.unwrap_or(missing), origin)
}

fn format_list(values: &[String]) -> String {
    values.join(", ")
}

fn format_repo_paths(values: &[RepoPath]) -> String {
    values
        .iter()
        .map(RepoPath::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_page_margin(margin: &PageMarginSettings) -> String {
    format!(
        "top={}, bottom={}, left={}, right={}",
        margin.top, margin.bottom, margin.left, margin.right
    )
}

fn has_allowed_cover_extension(path: &RepoPath) -> bool {
    Path::new(path.as_str())
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        .unwrap_or(false)
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
  kindle_previewer: true
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
validation:
  epubcheck: false
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
        assert!(!resolved.effective.validation.epubcheck);
        assert!(resolved.effective.validation.kindle_previewer);
    }

    #[test]
    fn resolves_cover_ebook_image_from_book_config() {
        let root = temp_dir("cover-image");
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
cover:
  ebook_image: assets/cover/front.jpg
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
        let resolved = resolve_book_config(&context).unwrap();

        assert_eq!(
            resolved
                .effective
                .cover
                .ebook_image
                .as_ref()
                .map(RepoPath::as_str),
            Some("assets/cover/front.jpg")
        );
    }

    #[test]
    fn ignores_cover_in_series_defaults() {
        let root = temp_dir("cover-series-defaults");
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
  cover:
    ebook_image: shared/assets/cover/default.jpg
  outputs:
    kindle:
      enabled: true
      target: kindle-ja
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
  vcs: git
book:
  title: "Vol 1"
  authors:
    - "Author"
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
"#,
        )
        .unwrap();

        let context = repo::discover(&root.join("books/vol-01"), None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();

        assert!(resolved.effective.cover.ebook_image.is_none());
    }

    #[test]
    fn resolves_pdf_defaults_for_prose_books() {
        let root = temp_dir("pdf-defaults");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();
        let pdf = resolved.effective.pdf.as_ref().unwrap();

        assert_eq!(pdf.engine, PdfEngine::Chromium);
        assert!(pdf.toc);
        assert!(pdf.page_number);
        assert_eq!(pdf.running_header, PdfRunningHeader::Auto);
    }

    #[test]
    fn defaults_horizontal_prose_print_engine_to_weasyprint() {
        let root = temp_dir("pdf-default-horizontal");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: paper
  vcs: git
book:
  title: "Sample Paper"
  authors:
    - "Author"
  reading_direction: ltr
layout:
  binding: left
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx4
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();
        let pdf = resolved.effective.pdf.as_ref().unwrap();

        assert_eq!(pdf.engine, PdfEngine::Weasyprint);
    }

    #[test]
    fn resolves_pdf_settings_from_book_config() {
        let root = temp_dir("pdf-explicit");
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
  print:
    enabled: true
    target: print-jp-pdfx1a
pdf:
  engine: typst
  toc: false
  page_number: false
  running_header: chapter
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();
        let pdf = resolved.effective.pdf.as_ref().unwrap();

        assert_eq!(pdf.engine, PdfEngine::Typst);
        assert!(!pdf.toc);
        assert!(!pdf.page_number);
        assert_eq!(pdf.running_header, PdfRunningHeader::Chapter);
    }

    #[test]
    fn resolves_conference_preprint_pdf_and_print_settings() {
        let root = temp_dir("conference-preprint");
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: paper
  vcs: git
book:
  title: "Sample Preprint"
  authors:
    - "Author"
  profile: conference-preprint
manuscript:
  chapters:
    - manuscript/01-main.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx4
pdf:
  toc: false
  page_number: false
  running_header: none
  column_count: 2
  column_gap: 10mm
  base_font_size: 9pt
  line_height: 14pt
print:
  trim_size: A4
  bleed: 0mm
  crop_marks: false
  page_margin:
    top: 20mm
    bottom: 20mm
    left: 15mm
    right: 15mm
  sides: duplex
  max_pages: 2
  pdf_standard: pdfx4
validation:
  strict: true
git:
  lfs: true
"#,
        )
        .unwrap();

        let context = repo::discover(&root, None).unwrap();
        let resolved = resolve_book_config(&context).unwrap();
        let pdf = resolved.effective.pdf.as_ref().unwrap();
        let print = resolved.effective.print.as_ref().unwrap();

        assert_eq!(resolved.effective.project.project_type, ProjectType::Paper);
        assert_eq!(resolved.effective.book.profile, "conference-preprint");
        assert_eq!(pdf.column_count, 2);
        assert_eq!(pdf.column_gap, "10mm");
        assert_eq!(pdf.base_font_size, "9pt");
        assert_eq!(pdf.line_height, "14pt");
        assert_eq!(print.trim_size, PrintTrimSize::A4);
        assert_eq!(print.sides, PrintSides::Duplex);
        assert_eq!(print.max_pages, Some(2));
        assert_eq!(print.pdf_standard, PrintPdfStandard::Pdfx4);
        assert_eq!(print.page_margin.as_ref().unwrap().top, "20mm");
    }

    #[test]
    fn rejects_conference_preprint_profile_for_non_paper_project() {
        let root = temp_dir("bad-conference-preprint-profile");
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
  profile: conference-preprint
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  print:
    enabled: true
    target: print-jp-pdfx1a
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
            ConfigError::InvalidFieldValue { field, .. } if field == "book.profile"
        ));
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
    fn rejects_cover_image_with_unsupported_extension() {
        let root = temp_dir("invalid-cover-extension");
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
cover:
  ebook_image: assets/cover/front.webp
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
            ConfigError::InvalidFieldValue { field, .. } if field == "cover.ebook_image"
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
