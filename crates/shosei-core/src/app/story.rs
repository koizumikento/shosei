use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    diagnostics::{Severity, ValidationIssue},
    domain::{RepoMode, RepoPath, RepoPathError},
    fs::join_repo_path,
    markdown::parse_frontmatter,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct StoryScaffoldOptions {
    pub shared: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct StoryScaffoldResult {
    pub summary: String,
    pub story_root: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct StoryMapOptions {}

#[derive(Debug, Clone)]
pub struct StoryMapResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub scene_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct StoryCheckOptions {}

#[derive(Debug, Clone)]
pub struct StoryCheckResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StoryDriftOptions {}

#[derive(Debug, Clone)]
pub struct StoryDriftResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Error)]
pub enum StoryScaffoldError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("story shared scaffold is only supported in series repositories")]
    SharedRequiresSeries,
    #[error("use either --shared or --book, not both")]
    ConflictingScope,
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

#[derive(Debug, Error)]
pub enum StoryMapError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("story scenes file not found: {path}")]
    MissingScenesFile { path: PathBuf },
    #[error("failed to read story scenes file {path}: {source}")]
    ReadScenesFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse story scenes file {path}: {source}")]
    ParseScenesFile {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("invalid scene file `{value}`: {source}")]
    InvalidSceneFile {
        value: String,
        #[source]
        source: RepoPathError,
    },
    #[error("failed to write story map report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize story map report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum StoryCheckError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("story scenes file not found: {path}")]
    MissingScenesFile { path: PathBuf },
    #[error("failed to read story scenes file {path}: {source}")]
    ReadScenesFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse story scenes file {path}: {source}")]
    ParseScenesFile {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to write story check report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize story check report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum StoryDriftError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("story drift is only supported in series repositories")]
    SeriesOnly,
    #[error("failed to write story drift report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize story drift report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
enum StoryScope {
    SingleBook,
    SeriesBook { book_id: String },
    SharedSeries,
}

#[derive(Debug, Clone)]
struct BookStoryWorkspace {
    repo_root: PathBuf,
    book_id: String,
    story_root: PathBuf,
    shared_story_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryScenesDocument {
    #[serde(default)]
    scenes: Vec<StoryScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryScene {
    file: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StoryMapReport {
    book_id: String,
    story_root: String,
    scenes_file: String,
    scene_count: usize,
    file_count: usize,
    warnings: Vec<String>,
    scenes: Vec<StoryScene>,
}

#[derive(Debug, Clone, Serialize)]
struct StoryCheckReport {
    book_id: String,
    story_root: String,
    scenes_file: String,
    scene_count: usize,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
struct StoryDriftReport {
    book_id: String,
    story_root: String,
    shared_story_root: String,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum StoryEntityKind {
    Character,
    Location,
    Term,
    Faction,
}

#[derive(Debug, Default)]
struct StoryEntityCatalog {
    characters: HashMap<String, StoryEntityEntry>,
    locations: HashMap<String, StoryEntityEntry>,
    terms: HashMap<String, StoryEntityEntry>,
    factions: HashMap<String, StoryEntityEntry>,
}

#[derive(Debug, Clone)]
struct StoryEntityEntry {
    path: PathBuf,
    scope: StoryEntityScope,
    contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoryEntityScope {
    Shared,
    Book,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoryEntityCollisionMode {
    IgnoreCrossScope,
    ReportCrossScope,
}

pub fn story_scaffold(
    command: &CommandContext,
    options: StoryScaffoldOptions,
) -> Result<StoryScaffoldResult, StoryScaffoldError> {
    let context = repo::discover(&command.start_path, command.book_id.as_deref())?;
    let repo_root = context.repo_root.clone();

    if options.shared && command.book_id.is_some() {
        return Err(StoryScaffoldError::ConflictingScope);
    }

    let (story_root, scope) = if options.shared {
        if context.mode != RepoMode::Series {
            return Err(StoryScaffoldError::SharedRequiresSeries);
        }
        (
            context.repo_root.join("shared/metadata/story"),
            StoryScope::SharedSeries,
        )
    } else {
        match context.mode {
            RepoMode::SingleBook => (context.repo_root.join("story"), StoryScope::SingleBook),
            RepoMode::Series => {
                let context = repo::require_book_context(context)?;
                let book = context.book.expect("series book must be resolved");
                (
                    book.root.join("story"),
                    StoryScope::SeriesBook { book_id: book.id },
                )
            }
        }
    };

    let mut created = Vec::new();
    let mut kept = Vec::new();
    scaffold_story_workspace(
        &repo_root,
        &story_root,
        &scope,
        options.force,
        &mut created,
        &mut kept,
    )?;

    let mut lines = vec![format!(
        "story scaffold: initialized {} at {}",
        scope.label(),
        story_root.display()
    )];
    lines.extend(created.into_iter().map(|path| format!("- created {path}")));
    lines.extend(kept.into_iter().map(|path| format!("- kept {path}")));

    Ok(StoryScaffoldResult {
        summary: lines.join("\n"),
        story_root,
    })
}

pub fn story_map(
    command: &CommandContext,
    _options: StoryMapOptions,
) -> Result<StoryMapResult, StoryMapError> {
    let workspace = discover_book_story_workspace(command)?;
    let (scenes_path, document) = load_story_scenes_for_map(&workspace)?;

    let mut warnings = Vec::new();
    let mut validated_scenes = Vec::with_capacity(document.scenes.len());
    for scene in document.scenes {
        let repo_path = RepoPath::parse(scene.file.clone()).map_err(|source| {
            StoryMapError::InvalidSceneFile {
                value: scene.file.clone(),
                source,
            }
        })?;
        validated_scenes.push(StoryScene {
            file: repo_path.as_str().to_string(),
            title: scene.title,
        });
    }

    let files = unique_values(validated_scenes.iter().map(|scene| scene.file.as_str()));
    if files.len() != validated_scenes.len() {
        warnings.push("duplicate scene file entries are present in scenes.yml".to_string());
    }

    let report = StoryMapReport {
        book_id: workspace.book_id.clone(),
        story_root: relative_display(&workspace.repo_root, &workspace.story_root),
        scenes_file: relative_display(&workspace.repo_root, &scenes_path),
        scene_count: validated_scenes.len(),
        file_count: files.len(),
        warnings: warnings.clone(),
        scenes: validated_scenes.clone(),
    };
    let report_path = story_map_report_path(&workspace.repo_root, &workspace.book_id);
    write_story_map_report(&report_path, &report)?;

    let mut lines = vec![format!(
        "story map: {} scene(s) from {} (report: {})",
        report.scene_count,
        report.scenes_file,
        report_path.display()
    )];
    lines.extend(
        validated_scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| format!("- {}. {}", index + 1, scene_summary_line(scene))),
    );
    if !warnings.is_empty() {
        lines.push(format!("warnings: {}", warnings.len()));
    }

    Ok(StoryMapResult {
        summary: lines.join("\n"),
        report_path,
        scene_count: report.scene_count,
    })
}

pub fn story_check(
    command: &CommandContext,
    _options: StoryCheckOptions,
) -> Result<StoryCheckResult, StoryCheckError> {
    let workspace = discover_book_story_workspace_for_check(command)?;
    let (scenes_path, document) = load_story_scenes_for_check(&workspace)?;

    let mut issues = Vec::new();
    let catalog = collect_story_entity_catalog(
        &workspace,
        StoryEntityCollisionMode::IgnoreCrossScope,
        &mut issues,
    );
    let mut seen = HashSet::new();
    let mut duplicate_paths = HashSet::new();

    for scene in &document.scenes {
        match RepoPath::parse(scene.file.clone()) {
            Ok(repo_path) => {
                let normalized = repo_path.as_str().to_string();
                if !seen.insert(normalized.clone()) && duplicate_paths.insert(normalized.clone()) {
                    issues.push(
                        ValidationIssue::warning(
                            "story",
                            format!("duplicate scene file entry: {normalized}"),
                            "scene ごとの `file` は一意に保ち、重複した entry を整理してください。",
                        )
                        .at(scenes_path.clone()),
                    );
                }

                let file_path = join_repo_path(&workspace.repo_root, &repo_path);
                if !file_path.is_file() {
                    issues.push(
                        ValidationIssue::warning(
                            "story",
                            format!("scene file not found: {}", repo_path.as_str()),
                            "原稿ファイルを作成するか、`scenes.yml` の `file` を修正してください。",
                        )
                        .at(file_path),
                    );
                    continue;
                }

                check_scene_frontmatter(&file_path, &workspace, &catalog, &mut issues);
            }
            Err(source) => {
                issues.push(
                    ValidationIssue::error(
                        "story",
                        format!("invalid scene file `{}`: {}", scene.file, source),
                        "`file` は repo-relative かつ `/` 区切りの path にしてください。",
                    )
                    .at(scenes_path.clone()),
                );
            }
        }
    }

    let report = StoryCheckReport {
        book_id: workspace.book_id.clone(),
        story_root: relative_display(&workspace.repo_root, &workspace.story_root),
        scenes_file: relative_display(&workspace.repo_root, &scenes_path),
        scene_count: document.scenes.len(),
        issues: issues.clone(),
    };
    let report_path = story_check_report_path(&workspace.repo_root, &workspace.book_id);
    write_story_check_report(&report_path, &report)?;
    let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);

    Ok(StoryCheckResult {
        summary: format!(
            "story check completed for {} with {} scene(s), issues: {}, report: {}",
            report.book_id,
            report.scene_count,
            issues.len(),
            report_path.display()
        ),
        report_path,
        issue_count: issues.len(),
        has_errors,
    })
}

pub fn story_drift(
    command: &CommandContext,
    _options: StoryDriftOptions,
) -> Result<StoryDriftResult, StoryDriftError> {
    let workspace = discover_book_story_workspace_for_drift(command)?;
    let mut issues = Vec::new();
    let _catalog = collect_story_entity_catalog(
        &workspace,
        StoryEntityCollisionMode::ReportCrossScope,
        &mut issues,
    );

    let report = StoryDriftReport {
        book_id: workspace.book_id.clone(),
        story_root: relative_display(&workspace.repo_root, &workspace.story_root),
        shared_story_root: relative_display(
            &workspace.repo_root,
            workspace.shared_story_root.as_ref().expect("series only"),
        ),
        issues: issues.clone(),
    };
    let report_path = story_drift_report_path(&workspace.repo_root, &workspace.book_id);
    write_story_drift_report(&report_path, &report)?;
    let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);

    Ok(StoryDriftResult {
        summary: format!(
            "story drift completed for {} with issues: {}, report: {}",
            report.book_id,
            issues.len(),
            report_path.display()
        ),
        report_path,
        issue_count: issues.len(),
        has_errors,
    })
}

impl StoryScope {
    fn label(&self) -> String {
        match self {
            Self::SingleBook => "single-book story workspace".to_string(),
            Self::SeriesBook { book_id } => format!("story workspace for {book_id}"),
            Self::SharedSeries => "shared series canon workspace".to_string(),
        }
    }
}

fn scaffold_story_workspace(
    repo_root: &Path,
    story_root: &Path,
    scope: &StoryScope,
    force: bool,
    created: &mut Vec<String>,
    kept: &mut Vec<String>,
) -> Result<(), StoryScaffoldError> {
    ensure_dir(story_root)?;
    write_scaffold_file(
        repo_root,
        &story_root.join("README.md"),
        &story_root_readme(scope),
        force,
        created,
        kept,
    )?;

    for (dir_name, contents) in entity_readmes() {
        write_scaffold_file(
            repo_root,
            &story_root.join(dir_name).join("README.md"),
            contents,
            force,
            created,
            kept,
        )?;
    }

    if !matches!(scope, StoryScope::SharedSeries) {
        write_scaffold_file(
            repo_root,
            &story_root.join("scenes.yml"),
            scenes_yml_contents(),
            force,
            created,
            kept,
        )?;
    }

    Ok(())
}

fn entity_readmes() -> [(&'static str, &'static str); 4] {
    [
        (
            "characters",
            "# Characters\n\nUse one Markdown file per character.\nSuggested frontmatter: `id`, `aliases`, `role`, `first_appears`, `status`.\n`shosei story check` resolves character references against `id`, or the filename stem when `id` is omitted.\n",
        ),
        (
            "locations",
            "# Locations\n\nUse one Markdown file per location.\nSuggested frontmatter: `id`, `region`, `first_appears`, `tags`.\n`shosei story check` resolves location references against `id`, or the filename stem when `id` is omitted.\n",
        ),
        (
            "terms",
            "# Terms\n\nUse one Markdown file per term, item, rule, or institution.\nSuggested frontmatter: `id`, `aliases`, `category`, `first_appears`.\n`shosei story check` resolves term references against `id`, or the filename stem when `id` is omitted.\n",
        ),
        (
            "factions",
            "# Factions\n\nUse one Markdown file per faction or organization.\nSuggested frontmatter: `id`, `leaders`, `goals`, `first_appears`.\n`shosei story check` resolves faction references against `id`, or the filename stem when `id` is omitted.\n",
        ),
    ]
}

fn story_root_readme(scope: &StoryScope) -> String {
    match scope {
        StoryScope::SingleBook => "# Story Workspace\n\nThis workspace stores story support data for this single-book repo.\n\n- `characters/`, `locations/`, `terms/`, `factions/`: repo-native codex entries\n- `scenes.yml`: manual scene index for this book\n\nGuidelines:\n- Keep file names and IDs stable once scenes or notes refer to them.\n- Scene Markdown can optionally use YAML frontmatter with `characters`, `locations`, `terms`, and `factions` arrays.\n- Keep paths repo-relative and `/`-separated when you copy them into config or notes.\n- This scaffold is manual-first. Fill in only the parts you actually need.\n".to_string(),
        StoryScope::SeriesBook { book_id } => format!(
            "# Story Workspace\n\nThis workspace stores book-scoped story support data for `{book_id}`.\n\n- `characters/`, `locations/`, `terms/`, `factions/`: repo-native codex entries\n- `scenes.yml`: manual scene index for this book\n\nGuidelines:\n- Keep file names and IDs stable once scenes or notes refer to them.\n- Scene Markdown can optionally use YAML frontmatter with `characters`, `locations`, `terms`, and `factions` arrays.\n- In `series`, `shosei story check` resolves those references against both this workspace and `shared/metadata/story/`.\n- Keep paths repo-relative and `/`-separated when you copy them into config or notes.\n- This scaffold is manual-first. Fill in only the parts you actually need.\n"
        ),
        StoryScope::SharedSeries => "# Story Workspace\n\nThis workspace stores shared series canon that multiple books may reference.\n\n- `characters/`, `locations/`, `terms/`, `factions/`: repo-native codex entries shared across volumes\n\nGuidelines:\n- Keep file names and IDs stable once volume-specific notes refer to them.\n- In `series`, `shosei story check` resolves book scene references against these shared entries too.\n- Keep paths repo-relative and `/`-separated when you copy them into config or notes.\n- This scaffold is manual-first. Fill in only the parts you actually need.\n".to_string(),
    }
}

fn scenes_yml_contents() -> &'static str {
    "scenes: []\n"
}

fn ensure_dir(path: &Path) -> Result<(), StoryScaffoldError> {
    fs::create_dir_all(path).map_err(|source| StoryScaffoldError::CreateDir {
        path: path.display().to_string(),
        source,
    })
}

fn write_scaffold_file(
    repo_root: &Path,
    path: &Path,
    contents: &str,
    force: bool,
    created: &mut Vec<String>,
    kept: &mut Vec<String>,
) -> Result<(), StoryScaffoldError> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }

    let display_path = relative_display(repo_root, path);
    if path.exists() && !force {
        kept.push(display_path);
        return Ok(());
    }

    fs::write(path, contents).map_err(|source| StoryScaffoldError::WriteFile {
        path: path.display().to_string(),
        source,
    })?;
    created.push(display_path);
    Ok(())
}

fn relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn discover_book_story_workspace(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StoryMapError> {
    discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
        .map_err(StoryMapError::Repo)
}

fn discover_book_story_workspace_for_check(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StoryCheckError> {
    discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
        .map_err(StoryCheckError::Repo)
}

fn discover_book_story_workspace_for_drift(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StoryDriftError> {
    let workspace =
        discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
            .map_err(StoryDriftError::Repo)?;
    if workspace.shared_story_root.is_none() {
        return Err(StoryDriftError::SeriesOnly);
    }
    Ok(workspace)
}

fn discover_book_story_workspace_inner(
    start_path: &Path,
    book_id: Option<&str>,
) -> Result<BookStoryWorkspace, RepoError> {
    let context = repo::discover(start_path, book_id)?;
    match context.mode {
        RepoMode::SingleBook => Ok(BookStoryWorkspace {
            repo_root: context.repo_root.clone(),
            book_id: "default".to_string(),
            story_root: context.repo_root.join("story"),
            shared_story_root: None,
        }),
        RepoMode::Series => {
            let context = repo::require_book_context(context)?;
            let book = context.book.expect("series book must be resolved");
            Ok(BookStoryWorkspace {
                shared_story_root: Some(context.repo_root.join("shared/metadata/story")),
                repo_root: context.repo_root,
                book_id: book.id,
                story_root: book.root.join("story"),
            })
        }
    }
}

fn unique_values<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            ordered.push(trimmed.to_string());
        }
    }
    ordered
}

fn story_map_report_path(repo_root: &Path, book_id: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("reports")
        .join(format!("{book_id}-story-map.json"))
}

fn story_check_report_path(repo_root: &Path, book_id: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("reports")
        .join(format!("{book_id}-story-check.json"))
}

fn story_drift_report_path(repo_root: &Path, book_id: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("reports")
        .join(format!("{book_id}-story-drift.json"))
}

fn write_story_map_report(path: &Path, report: &StoryMapReport) -> Result<(), StoryMapError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StoryMapError::WriteReport {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents =
        serde_json::to_string_pretty(report).map_err(|source| StoryMapError::SerializeReport {
            path: path.to_path_buf(),
            source,
        })?;
    fs::write(path, contents).map_err(|source| StoryMapError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn write_story_check_report(path: &Path, report: &StoryCheckReport) -> Result<(), StoryCheckError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StoryCheckError::WriteReport {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents = serde_json::to_string_pretty(report).map_err(|source| {
        StoryCheckError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, contents).map_err(|source| StoryCheckError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn write_story_drift_report(path: &Path, report: &StoryDriftReport) -> Result<(), StoryDriftError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StoryDriftError::WriteReport {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents = serde_json::to_string_pretty(report).map_err(|source| {
        StoryDriftError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, contents).map_err(|source| StoryDriftError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn load_story_scenes_for_map(
    workspace: &BookStoryWorkspace,
) -> Result<(PathBuf, StoryScenesDocument), StoryMapError> {
    let scenes_path = workspace.story_root.join("scenes.yml");
    if !scenes_path.is_file() {
        return Err(StoryMapError::MissingScenesFile { path: scenes_path });
    }
    let contents =
        fs::read_to_string(&scenes_path).map_err(|source| StoryMapError::ReadScenesFile {
            path: scenes_path.clone(),
            source,
        })?;
    let document =
        serde_yaml::from_str(&contents).map_err(|source| StoryMapError::ParseScenesFile {
            path: scenes_path.clone(),
            source,
        })?;
    Ok((scenes_path, document))
}

fn load_story_scenes_for_check(
    workspace: &BookStoryWorkspace,
) -> Result<(PathBuf, StoryScenesDocument), StoryCheckError> {
    let scenes_path = workspace.story_root.join("scenes.yml");
    if !scenes_path.is_file() {
        return Err(StoryCheckError::MissingScenesFile { path: scenes_path });
    }
    let contents =
        fs::read_to_string(&scenes_path).map_err(|source| StoryCheckError::ReadScenesFile {
            path: scenes_path.clone(),
            source,
        })?;
    let document =
        serde_yaml::from_str(&contents).map_err(|source| StoryCheckError::ParseScenesFile {
            path: scenes_path.clone(),
            source,
        })?;
    Ok((scenes_path, document))
}

fn collect_story_entity_catalog(
    workspace: &BookStoryWorkspace,
    collision_mode: StoryEntityCollisionMode,
    issues: &mut Vec<ValidationIssue>,
) -> StoryEntityCatalog {
    let mut catalog = StoryEntityCatalog::default();

    if let Some(shared_story_root) = &workspace.shared_story_root {
        collect_story_entity_catalog_from_root(
            &workspace.repo_root,
            shared_story_root,
            StoryEntityScope::Shared,
            collision_mode,
            &mut catalog,
            issues,
        );
    }
    collect_story_entity_catalog_from_root(
        &workspace.repo_root,
        &workspace.story_root,
        StoryEntityScope::Book,
        collision_mode,
        &mut catalog,
        issues,
    );

    catalog
}

fn collect_story_entity_catalog_from_root(
    repo_root: &Path,
    story_root: &Path,
    scope: StoryEntityScope,
    collision_mode: StoryEntityCollisionMode,
    catalog: &mut StoryEntityCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    for kind in StoryEntityKind::ALL {
        let dir = story_root.join(kind.dir_name());
        if !dir.exists() {
            continue;
        }
        if !dir.is_dir() {
            issues.push(
                ValidationIssue::error(
                    "story",
                    format!("story {} directory is not a directory", kind.field_name()),
                    format!("`{}` は directory として置いてください。", kind.dir_name()),
                )
                .at(dir),
            );
            continue;
        }

        let Ok(files) = markdown_files_in_dir(&dir) else {
            issues.push(
                ValidationIssue::error(
                    "story",
                    format!("failed to scan story {}", kind.field_name()),
                    format!(
                        "`{}` 配下の Markdown file を読める状態にしてください。",
                        relative_display(repo_root, &dir)
                    ),
                )
                .at(dir),
            );
            continue;
        };

        for path in files {
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name.eq_ignore_ascii_case("README.md") {
                continue;
            }

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(_) => {
                    issues.push(
                        ValidationIssue::error(
                            "story",
                            format!("failed to read {} entry", kind.label()),
                            format!(
                                "`{}` を読める状態にしてください。",
                                relative_display(repo_root, &path)
                            ),
                        )
                        .at(path.clone()),
                    );
                    continue;
                }
            };

            let frontmatter = match parse_frontmatter(&contents) {
                Ok(frontmatter) => frontmatter,
                Err(source) => {
                    issues.push(frontmatter_issue(
                        "story",
                        &path,
                        format!("invalid {} frontmatter: {}", kind.label(), source),
                        "frontmatter は file 冒頭の YAML mapping として書いてください。",
                    ));
                    continue;
                }
            };

            let id = match story_entity_id(&path, frontmatter.as_ref()) {
                Ok(id) => id,
                Err(cause) => {
                    issues.push(
                        ValidationIssue::error(
                            "story",
                            format!("invalid {} id: {cause}", kind.label()),
                            "frontmatter の `id` を non-empty string にするか、filename stem を使ってください。",
                        )
                        .at(path.clone()),
                    );
                    continue;
                }
            };

            let entry = StoryEntityEntry {
                path: path.clone(),
                scope,
                contents,
            };
            if let Some(previous) = catalog.map_mut(kind).insert(id.clone(), entry.clone())
                && let Some(issue) = story_entity_collision_issue(
                    repo_root,
                    kind,
                    &id,
                    &previous,
                    &entry,
                    collision_mode,
                )
            {
                issues.push(issue);
            }
        }
    }
}

fn check_scene_frontmatter(
    scene_path: &Path,
    workspace: &BookStoryWorkspace,
    catalog: &StoryEntityCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let contents = match fs::read_to_string(scene_path) {
        Ok(contents) => contents,
        Err(_) => {
            issues.push(
                ValidationIssue::error(
                    "story",
                    "failed to read scene file frontmatter".to_string(),
                    "scene file を読める状態にしてください。",
                )
                .at(scene_path.to_path_buf()),
            );
            return;
        }
    };

    let frontmatter = match parse_frontmatter(&contents) {
        Ok(frontmatter) => frontmatter,
        Err(source) => {
            issues.push(frontmatter_issue(
                "story",
                scene_path,
                format!("invalid scene frontmatter: {source}"),
                "scene frontmatter は file 冒頭の YAML mapping として書いてください。",
            ));
            return;
        }
    };

    let Some(frontmatter) = frontmatter else {
        return;
    };

    for kind in StoryEntityKind::ALL {
        for id in scene_reference_ids(&frontmatter, kind, scene_path, issues) {
            if !catalog.contains(kind, &id) {
                issues.push(
                    ValidationIssue::warning(
                        "story",
                        format!("scene references unknown {} `{id}`", kind.label()),
                        missing_story_reference_remedy(workspace, kind),
                    )
                    .at(scene_path.to_path_buf()),
                );
            }
        }
    }
}

fn markdown_files_in_dir(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn story_entity_id(path: &Path, frontmatter: Option<&Mapping>) -> Result<String, &'static str> {
    if let Some(frontmatter) = frontmatter
        && let Some(value) = mapping_value(frontmatter, "id")
    {
        return match value {
            Value::String(id) if !id.trim().is_empty() => Ok(id.trim().to_string()),
            Value::String(_) => Err("`id` must not be empty"),
            _ => Err("`id` must be a string"),
        };
    }

    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Err("filename stem is not valid UTF-8");
    };
    let stem = stem.trim();
    if stem.is_empty() {
        Err("filename stem must not be empty")
    } else {
        Ok(stem.to_string())
    }
}

fn scene_reference_ids(
    frontmatter: &Mapping,
    kind: StoryEntityKind,
    scene_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let Some(value) = mapping_value(frontmatter, kind.field_name()) else {
        return Vec::new();
    };

    match value {
        Value::String(id) => normalize_story_ids([id.as_str()], kind, scene_path, issues),
        Value::Sequence(values) => {
            let mut ids = Vec::new();
            for value in values {
                match value {
                    Value::String(id) => {
                        ids.extend(normalize_story_ids([id.as_str()], kind, scene_path, issues))
                    }
                    _ => issues.push(
                        ValidationIssue::error(
                            "story",
                            format!(
                                "scene frontmatter `{}` must contain only strings",
                                kind.field_name()
                            ),
                            format!(
                                "`{}` は string か string sequence にしてください。",
                                kind.field_name()
                            ),
                        )
                        .at(scene_path.to_path_buf()),
                    ),
                }
            }
            ids
        }
        _ => {
            issues.push(
                ValidationIssue::error(
                    "story",
                    format!("scene frontmatter `{}` has invalid type", kind.field_name()),
                    format!(
                        "`{}` は string か string sequence にしてください。",
                        kind.field_name()
                    ),
                )
                .at(scene_path.to_path_buf()),
            );
            Vec::new()
        }
    }
}

fn normalize_story_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: StoryEntityKind,
    scene_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let mut normalized = Vec::new();
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            issues.push(
                ValidationIssue::error(
                    "story",
                    format!(
                        "scene frontmatter `{}` contains an empty id",
                        kind.field_name()
                    ),
                    format!(
                        "`{}` には non-empty string を入れてください。",
                        kind.field_name()
                    ),
                )
                .at(scene_path.to_path_buf()),
            );
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    normalized
}

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn frontmatter_issue(target: &str, path: &Path, cause: String, remedy: &str) -> ValidationIssue {
    ValidationIssue::error(target, cause, remedy).at(path.to_path_buf())
}

fn missing_story_reference_remedy(workspace: &BookStoryWorkspace, kind: StoryEntityKind) -> String {
    let book_path = relative_display(
        &workspace.repo_root,
        &workspace.story_root.join(kind.dir_name()),
    );
    if let Some(shared_story_root) = &workspace.shared_story_root {
        let shared_path = relative_display(
            &workspace.repo_root,
            &shared_story_root.join(kind.dir_name()),
        );
        format!(
            "`{book_path}/` か `{shared_path}/` に entry を追加するか、scene frontmatter を修正してください。"
        )
    } else {
        format!("`{book_path}/` に entry を追加するか、scene frontmatter を修正してください。")
    }
}

fn scene_summary_line(scene: &StoryScene) -> String {
    let title = scene.title.as_deref().unwrap_or("(untitled)");
    format!("{} - {}", scene.file, title)
}

impl StoryEntityKind {
    const ALL: [Self; 4] = [Self::Character, Self::Location, Self::Term, Self::Faction];

    fn dir_name(self) -> &'static str {
        self.field_name()
    }

    fn field_name(self) -> &'static str {
        match self {
            Self::Character => "characters",
            Self::Location => "locations",
            Self::Term => "terms",
            Self::Faction => "factions",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Location => "location",
            Self::Term => "term",
            Self::Faction => "faction",
        }
    }
}

impl StoryEntityCatalog {
    fn contains(&self, kind: StoryEntityKind, id: &str) -> bool {
        self.map(kind).contains_key(id)
    }

    fn map(&self, kind: StoryEntityKind) -> &HashMap<String, StoryEntityEntry> {
        match kind {
            StoryEntityKind::Character => &self.characters,
            StoryEntityKind::Location => &self.locations,
            StoryEntityKind::Term => &self.terms,
            StoryEntityKind::Faction => &self.factions,
        }
    }

    fn map_mut(&mut self, kind: StoryEntityKind) -> &mut HashMap<String, StoryEntityEntry> {
        match kind {
            StoryEntityKind::Character => &mut self.characters,
            StoryEntityKind::Location => &mut self.locations,
            StoryEntityKind::Term => &mut self.terms,
            StoryEntityKind::Faction => &mut self.factions,
        }
    }
}

fn story_entity_collision_issue(
    repo_root: &Path,
    kind: StoryEntityKind,
    id: &str,
    previous: &StoryEntityEntry,
    current: &StoryEntityEntry,
    collision_mode: StoryEntityCollisionMode,
) -> Option<ValidationIssue> {
    if previous.scope == current.scope {
        return Some(
            ValidationIssue::error(
                "story",
                format!("duplicate {} id `{id}`", kind.label()),
                format!(
                    "`{}` と `{}` で同じ `id` を使わないでください。",
                    relative_display(repo_root, &previous.path),
                    relative_display(repo_root, &current.path)
                ),
            )
            .at(current.path.clone()),
        );
    }

    match collision_mode {
        StoryEntityCollisionMode::IgnoreCrossScope => None,
        StoryEntityCollisionMode::ReportCrossScope if previous.contents == current.contents => {
            Some(
                ValidationIssue::warning(
                    "story",
                    format!("redundant shared/book {} copy for `{id}`", kind.label()),
                    format!(
                        "`{}` と `{}` は同じ内容なので、shared か book のどちらか一方に寄せてください。",
                        relative_display(repo_root, &previous.path),
                        relative_display(repo_root, &current.path)
                    ),
                )
                .at(current.path.clone()),
            )
        }
        StoryEntityCollisionMode::ReportCrossScope => Some(
            ValidationIssue::error(
                "story",
                format!("shared canon drift for {} `{id}`", kind.label()),
                format!(
                    "`{}` と `{}` の内容が分岐しています。shared canon を正とするか、book 側の差分を明示的に整理してください。",
                    relative_display(repo_root, &previous.path),
                    relative_display(repo_root, &current.path)
                ),
            )
            .at(current.path.clone()),
        ),
    }
}
