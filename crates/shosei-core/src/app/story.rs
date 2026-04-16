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

const STORY_CHARACTER_TEMPLATE: &str = include_str!("../../templates/story/character-template.md");
const STORY_LOCATION_TEMPLATE: &str = include_str!("../../templates/story/location-template.md");
const STORY_TERM_TEMPLATE: &str = include_str!("../../templates/story/term-template.md");
const STORY_FACTION_TEMPLATE: &str = include_str!("../../templates/story/faction-template.md");
const STORY_SCENE_TEMPLATE: &str = include_str!("../../templates/story/scene-template.md");
const STORY_STRUCTURES_README: &str = include_str!("../../templates/story/structures-readme.md");
const STORY_KISHOTENKETSU_TEMPLATE: &str =
    include_str!("../../templates/story/kishotenketsu-template.md");
const STORY_THREE_ACT_TEMPLATE: &str = include_str!("../../templates/story/three-act-template.md");
const STORY_SAVE_THE_CAT_TEMPLATE: &str =
    include_str!("../../templates/story/save-the-cat-template.md");
const STORY_HEROES_JOURNEY_TEMPLATE: &str =
    include_str!("../../templates/story/heroes-journey-template.md");

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
pub struct StorySeedOptions {
    pub template: String,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct StorySeedResult {
    pub summary: String,
    pub template_path: PathBuf,
    pub scenes_path: PathBuf,
    pub scene_count: usize,
    pub created_note_count: usize,
}

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

#[derive(Debug, Clone)]
pub struct StorySyncOptions {
    pub source: Option<String>,
    pub destination: Option<String>,
    pub kind: Option<String>,
    pub id: Option<String>,
    pub report: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct StorySyncResult {
    pub summary: String,
    pub target_path: Option<PathBuf>,
    pub changed: bool,
    pub changed_count: usize,
    pub requested_count: usize,
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
pub enum StorySeedError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("invalid story template name `{value}`")]
    InvalidTemplateName { value: String },
    #[error("story structure template not found: {path}")]
    MissingTemplate { path: PathBuf },
    #[error("failed to read story structure template {path}: {source}")]
    ReadTemplate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse story structure template frontmatter {path}: {detail}")]
    ParseTemplate { path: PathBuf, detail: String },
    #[error("story structure template is missing `scene_seeds`: {path}")]
    MissingSceneSeeds { path: PathBuf },
    #[error("story structure template `scene_seeds` must not be empty: {path}")]
    EmptySceneSeeds { path: PathBuf },
    #[error("invalid story scene seed #{index} in {path}: {detail}")]
    InvalidSceneSeed {
        path: PathBuf,
        index: usize,
        detail: String,
    },
    #[error("duplicate story scene seed file `{value}` in {path}")]
    DuplicateSeedFile { path: PathBuf, value: String },
    #[error("invalid story scene seed file `{value}` in {path}: {source}")]
    InvalidSceneFile {
        path: PathBuf,
        value: String,
        #[source]
        source: RepoPathError,
    },
    #[error("story scenes file already contains entries; rerun with --force to replace: {path}")]
    ScenesRequireForce { path: PathBuf },
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
    #[error("failed to serialize story seed YAML for {path}: {source}")]
    SerializeYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
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

#[derive(Debug, Error)]
pub enum StorySyncError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("story sync is only supported in series repositories")]
    SeriesOnly,
    #[error("use exactly one of `--from` or `--to`")]
    InvalidDirection,
    #[error("unsupported story sync source `{value}`")]
    UnsupportedSource { value: String },
    #[error("unsupported story sync destination `{value}`")]
    UnsupportedDestination { value: String },
    #[error("unsupported story entity kind `{value}`")]
    UnsupportedKind { value: String },
    #[error("use either `--report` or both `--kind` and `--id`")]
    InvalidSelection,
    #[error("`story sync --report` requires `--force`")]
    ReportSyncRequiresForce,
    #[error("failed to read story drift report {path}: {source}")]
    ReadReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse story drift report {path}: {source}")]
    ParseReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("story drift report was created for `{actual}`, not `{expected}`")]
    ReportBookMismatch { expected: String, actual: String },
    #[error("story drift report contains duplicate entry for {kind} `{id}`")]
    DuplicateReportEntry { kind: String, id: String },
    #[error("story drift report contains invalid repo path `{value}`: {source}")]
    InvalidReportPath {
        value: String,
        #[source]
        source: RepoPathError,
    },
    #[error("shared story entity not found for {kind} `{id}`")]
    MissingSharedEntity { kind: String, id: String },
    #[error("book story entity not found for {kind} `{id}`")]
    MissingBookEntity { kind: String, id: String },
    #[error(
        "book story entity `{id}` already exists with different content; rerun with --force to overwrite"
    )]
    BookEntityConflict { id: String },
    #[error(
        "shared story entity `{id}` already exists with different content; rerun with --force to overwrite"
    )]
    SharedEntityConflict { id: String },
    #[error("target path already exists for another story entry: {path}")]
    TargetPathConflict { path: PathBuf },
    #[error("failed to scan {path}: {source}")]
    ScanDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse story entity frontmatter {path}: {detail}")]
    ParseEntity { path: PathBuf, detail: String },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoryScenesDocument {
    #[serde(default)]
    scenes: Vec<StoryScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoryScene {
    file: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Clone)]
struct StoryStructureSceneSeed {
    title: String,
    file: String,
    beat: Option<String>,
    summary: Option<String>,
    characters: Vec<String>,
    locations: Vec<String>,
    terms: Vec<String>,
    factions: Vec<String>,
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
    drifts: Vec<StoryDriftEntry>,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StoryDriftStatus {
    RedundantCopy,
    Drift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoryDriftEntry {
    kind: String,
    id: String,
    status: StoryDriftStatus,
    shared_path: String,
    book_path: String,
}

#[derive(Debug, Clone, Deserialize)]
struct StorySyncReportInput {
    book_id: String,
    #[serde(default)]
    drifts: Vec<StoryDriftEntry>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorySyncDirection {
    FromShared,
    ToShared,
}

#[derive(Debug, Clone)]
enum StorySyncTarget {
    Single { kind: StoryEntityKind, id: String },
    Report { path: PathBuf },
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
        story_root: relative_repo_path(&workspace.repo_root, &workspace.story_root),
        scenes_file: relative_repo_path(&workspace.repo_root, &scenes_path),
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

pub fn story_seed(
    command: &CommandContext,
    options: StorySeedOptions,
) -> Result<StorySeedResult, StorySeedError> {
    let workspace = discover_book_story_workspace_for_seed(command)?;
    let (template_path, seeds) = load_story_structure_scene_seeds(&workspace, &options.template)?;
    let scenes_path = workspace.story_root.join("scenes.yml");
    let scenes_document = StoryScenesDocument {
        scenes: seeds
            .iter()
            .map(|seed| StoryScene {
                file: seed.file.clone(),
                title: Some(seed.title.clone()),
            })
            .collect(),
    };

    let scenes_action = seed_story_scenes_file(&scenes_path, &scenes_document, options.force)?;
    let template_label = template_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("structure")
        .to_string();
    let (created, overwritten, kept) =
        seed_story_note_files(&workspace, &template_label, &seeds, options.force)?;

    let mut lines = vec![format!(
        "story seed: applied {} seed(s) from {}",
        scenes_document.scenes.len(),
        relative_display(&workspace.repo_root, &template_path)
    )];
    lines.push(format!(
        "- {} {}",
        match scenes_action {
            StorySeedWriteAction::Created => "created",
            StorySeedWriteAction::Updated => "updated",
            StorySeedWriteAction::Kept => "kept",
        },
        relative_display(&workspace.repo_root, &scenes_path)
    ));
    lines.extend(created.iter().map(|path| {
        format!(
            "- created scene note {}",
            relative_display(&workspace.repo_root, path)
        )
    }));
    lines.extend(overwritten.iter().map(|path| {
        format!(
            "- updated scene note {}",
            relative_display(&workspace.repo_root, path)
        )
    }));
    lines.extend(kept.iter().map(|path| {
        format!(
            "- kept scene note {}",
            relative_display(&workspace.repo_root, path)
        )
    }));

    Ok(StorySeedResult {
        summary: lines.join("\n"),
        template_path,
        scenes_path,
        scene_count: scenes_document.scenes.len(),
        created_note_count: created.len(),
    })
}

pub fn story_check(
    command: &CommandContext,
    _options: StoryCheckOptions,
) -> Result<StoryCheckResult, StoryCheckError> {
    let workspace = discover_book_story_workspace_for_check(command)?;
    let (scenes_path, document) = load_story_scenes_for_check(&workspace)?;

    let mut issues = Vec::new();
    let mut drifts = Vec::new();
    let catalog = collect_story_entity_catalog(
        &workspace,
        StoryEntityCollisionMode::IgnoreCrossScope,
        &mut issues,
        &mut drifts,
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
        story_root: relative_repo_path(&workspace.repo_root, &workspace.story_root),
        scenes_file: relative_repo_path(&workspace.repo_root, &scenes_path),
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
    let mut drifts = Vec::new();
    let _catalog = collect_story_entity_catalog(
        &workspace,
        StoryEntityCollisionMode::ReportCrossScope,
        &mut issues,
        &mut drifts,
    );

    let report = StoryDriftReport {
        book_id: workspace.book_id.clone(),
        story_root: relative_repo_path(&workspace.repo_root, &workspace.story_root),
        shared_story_root: relative_repo_path(
            &workspace.repo_root,
            workspace.shared_story_root.as_ref().expect("series only"),
        ),
        drifts,
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

pub fn story_sync(
    command: &CommandContext,
    options: StorySyncOptions,
) -> Result<StorySyncResult, StorySyncError> {
    let workspace = discover_book_story_workspace_for_sync(command)?;
    let direction = story_sync_direction(&options)?;
    match story_sync_target(&options)? {
        StorySyncTarget::Single { kind, id } => {
            let shared_story_root = workspace.shared_story_root.as_ref().expect("series only");
            let book_story_root = &workspace.story_root;

            match direction {
                StorySyncDirection::FromShared => sync_story_entity(
                    &workspace,
                    kind,
                    &id,
                    options.force,
                    SyncEndpoints {
                        source_root: shared_story_root,
                        source_scope: StoryEntityScope::Shared,
                        destination_root: book_story_root,
                        destination_scope: StoryEntityScope::Book,
                    },
                ),
                StorySyncDirection::ToShared => sync_story_entity(
                    &workspace,
                    kind,
                    &id,
                    options.force,
                    SyncEndpoints {
                        source_root: book_story_root,
                        source_scope: StoryEntityScope::Book,
                        destination_root: shared_story_root,
                        destination_scope: StoryEntityScope::Shared,
                    },
                ),
            }
        }
        StorySyncTarget::Report { path } => sync_story_report(&workspace, direction, &path),
    }
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

    for (dir_name, readme_contents, template_contents) in story_entity_scaffolds() {
        write_scaffold_file(
            repo_root,
            &story_root.join(dir_name).join("README.md"),
            readme_contents,
            force,
            created,
            kept,
        )?;
        write_scaffold_file(
            repo_root,
            &story_root.join(dir_name).join("_template.md"),
            template_contents,
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
        write_scaffold_file(
            repo_root,
            &story_root.join("scene-template.md"),
            STORY_SCENE_TEMPLATE,
            force,
            created,
            kept,
        )?;
        for (file_name, contents) in story_structure_scaffolds() {
            write_scaffold_file(
                repo_root,
                &story_root.join("structures").join(file_name),
                contents,
                force,
                created,
                kept,
            )?;
        }
    }

    Ok(())
}

fn story_entity_scaffolds() -> [(&'static str, &'static str, &'static str); 4] {
    [
        (
            "characters",
            "# Characters\n\n1 file 1 character で管理する。\n\n- `_template.md` を複製して使う\n- `shosei story check` が参照解決に使う key は frontmatter の `id`\n- `id` を省略した場合は filename stem を使う\n- 値や本文は日本語でよい\n- `id` 以外の frontmatter key は自由メモとして日本語で書いてよい\n",
            STORY_CHARACTER_TEMPLATE,
        ),
        (
            "locations",
            "# Locations\n\n1 file 1 location で管理する。\n\n- `_template.md` を複製して使う\n- `shosei story check` が参照解決に使う key は frontmatter の `id`\n- `id` を省略した場合は filename stem を使う\n- 値や本文は日本語でよい\n- `id` 以外の frontmatter key は自由メモとして日本語で書いてよい\n",
            STORY_LOCATION_TEMPLATE,
        ),
        (
            "terms",
            "# Terms\n\n1 file 1 term で管理する。item、rule、institution もここでよい。\n\n- `_template.md` を複製して使う\n- `shosei story check` が参照解決に使う key は frontmatter の `id`\n- `id` を省略した場合は filename stem を使う\n- 値や本文は日本語でよい\n- `id` 以外の frontmatter key は自由メモとして日本語で書いてよい\n",
            STORY_TERM_TEMPLATE,
        ),
        (
            "factions",
            "# Factions\n\n1 file 1 faction で管理する。組織や陣営もここでよい。\n\n- `_template.md` を複製して使う\n- `shosei story check` が参照解決に使う key は frontmatter の `id`\n- `id` を省略した場合は filename stem を使う\n- 値や本文は日本語でよい\n- `id` 以外の frontmatter key は自由メモとして日本語で書いてよい\n",
            STORY_FACTION_TEMPLATE,
        ),
    ]
}

fn story_structure_scaffolds() -> [(&'static str, &'static str); 5] {
    [
        ("README.md", STORY_STRUCTURES_README),
        ("kishotenketsu.md", STORY_KISHOTENKETSU_TEMPLATE),
        ("three-act.md", STORY_THREE_ACT_TEMPLATE),
        ("save-the-cat.md", STORY_SAVE_THE_CAT_TEMPLATE),
        ("heroes-journey.md", STORY_HEROES_JOURNEY_TEMPLATE),
    ]
}

fn story_root_readme(scope: &StoryScope) -> String {
    match scope {
        StoryScope::SingleBook => "# Story Workspace\n\nこの workspace は single-book repo 用の物語補助データを置く。\n\n- `characters/`, `locations/`, `terms/`, `factions/`: 作品内の codex entry\n- `scenes.yml`: この本の scene index\n- `scene-template.md`: scene Markdown frontmatter の記入例\n- `structures/`: 起承転結、三幕構成、Save the Cat!、ヒーローズ・ジャーニーの構成メモ\n- `scene-notes/`: `shosei story seed` が必要に応じて作る scene note 下書き\n\n運用メモ:\n- file 名と識別子は scene や note から参照し始めたら安定させる\n- CLI が読む key は `id`, `characters`, `locations`, `terms`, `factions`, `scenes`, `file`, `title` のように英語のまま使う\n- `structures/` 配下は自由記述の構成メモで、`scene_seeds` frontmatter を置けば `shosei story seed` の入力にもできる\n- `scene-notes/` は下書き置き場で、v0.1 の CLI はそこに本文生成まではしない\n- 値や本文、見出し、補足説明は日本語でよい\n- config や note に path を写すときは repo-relative かつ `/` 区切りにする\n- この scaffold は manual-first。必要なところから埋める\n".to_string(),
        StoryScope::SeriesBook { book_id } => format!(
            "# Story Workspace\n\nこの workspace は `{book_id}` 用の巻固有 story data を置く。\n\n- `characters/`, `locations/`, `terms/`, `factions/`: 巻固有の codex entry\n- `scenes.yml`: この巻の scene index\n- `scene-template.md`: scene Markdown frontmatter の記入例\n- `structures/`: 起承転結、三幕構成、Save the Cat!、ヒーローズ・ジャーニーの構成メモ\n- `scene-notes/`: `shosei story seed` が必要に応じて作る scene note 下書き\n\n運用メモ:\n- file 名と識別子は scene や note から参照し始めたら安定させる\n- CLI が読む key は `id`, `characters`, `locations`, `terms`, `factions`, `scenes`, `file`, `title` のように英語のまま使う\n- `structures/` 配下は自由記述の構成メモで、`scene_seeds` frontmatter を置けば `shosei story seed` の入力にもできる\n- `scene-notes/` は下書き置き場で、v0.1 の CLI はそこに本文生成まではしない\n- 値や本文、見出し、補足説明は日本語でよい\n- `series` では `shosei story check` がこの workspace と `shared/metadata/story/` の両方を参照解決に使う\n- config や note に path を写すときは repo-relative かつ `/` 区切りにする\n- この scaffold は manual-first。必要なところから埋める\n"
        ),
        StoryScope::SharedSeries => "# Story Workspace\n\nこの workspace は複数巻から参照される shared canon を置く。\n\n- `characters/`, `locations/`, `terms/`, `factions/`: 巻をまたいで共有する codex entry\n\n運用メモ:\n- file 名と識別子は各巻から参照し始めたら安定させる\n- CLI が読む key は `id`, `characters`, `locations`, `terms`, `factions`, `scenes`, `file`, `title` のように英語のまま使う\n- 値や本文、見出し、補足説明は日本語でよい\n- `series` では `shosei story check` が book scene からこれら shared entry も参照解決に使う\n- config や note に path を写すときは repo-relative かつ `/` 区切りにする\n- この scaffold は manual-first。必要なところから埋める\n".to_string(),
    }
}

fn scenes_yml_contents() -> &'static str {
    "# 例:\n# scenes:\n#   - file: manuscript/01-chapter-1.md\n#     title: 導入\nscenes: []\n"
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

fn relative_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn discover_book_story_workspace(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StoryMapError> {
    discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
        .map_err(StoryMapError::Repo)
}

fn discover_book_story_workspace_for_seed(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StorySeedError> {
    discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
        .map_err(StorySeedError::Repo)
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

fn discover_book_story_workspace_for_sync(
    command: &CommandContext,
) -> Result<BookStoryWorkspace, StorySyncError> {
    let workspace =
        discover_book_story_workspace_inner(&command.start_path, command.book_id.as_deref())
            .map_err(StorySyncError::Repo)?;
    if workspace.shared_story_root.is_none() {
        return Err(StorySyncError::SeriesOnly);
    }
    Ok(workspace)
}

#[derive(Debug, Clone, Copy)]
struct SyncEndpoints<'a> {
    source_root: &'a Path,
    source_scope: StoryEntityScope,
    destination_root: &'a Path,
    destination_scope: StoryEntityScope,
}

#[derive(Debug, Clone, Copy)]
enum StorySeedWriteAction {
    Created,
    Updated,
    Kept,
}

#[derive(Debug, Clone)]
struct StorySyncReportPlan {
    target_path: PathBuf,
    contents_to_write: Option<String>,
}

fn story_sync_direction(options: &StorySyncOptions) -> Result<StorySyncDirection, StorySyncError> {
    match (options.source.as_deref(), options.destination.as_deref()) {
        (Some("shared"), None) => Ok(StorySyncDirection::FromShared),
        (None, Some("shared")) => Ok(StorySyncDirection::ToShared),
        (Some(value), None) => Err(StorySyncError::UnsupportedSource {
            value: value.to_string(),
        }),
        (None, Some(value)) => Err(StorySyncError::UnsupportedDestination {
            value: value.to_string(),
        }),
        _ => Err(StorySyncError::InvalidDirection),
    }
}

fn story_sync_target(options: &StorySyncOptions) -> Result<StorySyncTarget, StorySyncError> {
    match (
        &options.report,
        options.kind.as_deref(),
        options.id.as_deref(),
    ) {
        (Some(_), None, None) => {
            if !options.force {
                return Err(StorySyncError::ReportSyncRequiresForce);
            }
            Ok(StorySyncTarget::Report {
                path: options.report.clone().expect("checked above"),
            })
        }
        (Some(_), _, _) => Err(StorySyncError::InvalidSelection),
        (None, Some(kind), Some(id)) => {
            let kind =
                StoryEntityKind::from_cli(kind).ok_or_else(|| StorySyncError::UnsupportedKind {
                    value: kind.to_string(),
                })?;
            Ok(StorySyncTarget::Single {
                kind,
                id: id.to_string(),
            })
        }
        _ => Err(StorySyncError::InvalidSelection),
    }
}

fn sync_story_entity(
    workspace: &BookStoryWorkspace,
    kind: StoryEntityKind,
    id: &str,
    force: bool,
    endpoints: SyncEndpoints<'_>,
) -> Result<StorySyncResult, StorySyncError> {
    let source_entry =
        find_story_entity_by_id(endpoints.source_root, endpoints.source_scope, kind, id)?
            .ok_or_else(|| missing_story_entity_error(endpoints.source_scope, kind, id))?;

    let destination_dir = endpoints.destination_root.join(kind.dir_name());
    let destination_entry = find_story_entity_by_id(
        endpoints.destination_root,
        endpoints.destination_scope,
        kind,
        id,
    )?;

    let target_path = if let Some(entry) = &destination_entry {
        entry.path.clone()
    } else {
        destination_dir.join(
            source_entry
                .path
                .file_name()
                .expect("story entity file name must exist"),
        )
    };

    if let Some(entry) = &destination_entry {
        if entry.contents == source_entry.contents {
            return Ok(StorySyncResult {
                summary: format!(
                    "story sync: `{}` already matches {} at {}",
                    id,
                    endpoints.source_scope.label(),
                    relative_display(&workspace.repo_root, &entry.path)
                ),
                target_path: Some(entry.path.clone()),
                changed: false,
                changed_count: 0,
                requested_count: 1,
            });
        }
        if !force {
            return Err(conflicting_story_entity_error(
                endpoints.destination_scope,
                id,
            ));
        }
    } else if target_path.exists() {
        return Err(StorySyncError::TargetPathConflict { path: target_path });
    }

    fs::create_dir_all(&destination_dir).map_err(|source| StorySyncError::CreateDir {
        path: destination_dir.clone(),
        source,
    })?;
    fs::write(&target_path, &source_entry.contents).map_err(|source| {
        StorySyncError::WriteFile {
            path: target_path.clone(),
            source,
        }
    })?;

    Ok(StorySyncResult {
        summary: format!(
            "story sync: copied {} {} `{}` to {}",
            endpoints.source_scope.label(),
            kind.label(),
            id,
            relative_display(&workspace.repo_root, &target_path)
        ),
        target_path: Some(target_path),
        changed: true,
        changed_count: 1,
        requested_count: 1,
    })
}

fn sync_story_report(
    workspace: &BookStoryWorkspace,
    direction: StorySyncDirection,
    report_path: &Path,
) -> Result<StorySyncResult, StorySyncError> {
    let report = load_story_sync_report(report_path)?;
    if report.book_id != workspace.book_id {
        return Err(StorySyncError::ReportBookMismatch {
            expected: workspace.book_id.clone(),
            actual: report.book_id,
        });
    }

    let mut seen = HashSet::new();
    let mut plans = Vec::new();
    for drift in report.drifts {
        let kind = StoryEntityKind::from_cli(&drift.kind).ok_or_else(|| {
            StorySyncError::UnsupportedKind {
                value: drift.kind.clone(),
            }
        })?;
        let key = (kind, drift.id.clone());
        if !seen.insert(key) {
            return Err(StorySyncError::DuplicateReportEntry {
                kind: drift.kind,
                id: drift.id,
            });
        }
        plans.push(prepare_story_sync_report_plan(
            workspace, direction, drift, kind,
        )?);
    }

    let changed_count = plans
        .iter()
        .filter(|plan| plan.contents_to_write.is_some())
        .count();
    for plan in &plans {
        if let Some(contents) = &plan.contents_to_write {
            if let Some(parent) = plan.target_path.parent() {
                fs::create_dir_all(parent).map_err(|source| StorySyncError::CreateDir {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::write(&plan.target_path, contents).map_err(|source| StorySyncError::WriteFile {
                path: plan.target_path.clone(),
                source,
            })?;
        }
    }

    Ok(StorySyncResult {
        summary: format!(
            "story sync: applied {} report entries from {} (changed: {}, unchanged: {})",
            plans.len(),
            report_path.display(),
            changed_count,
            plans.len().saturating_sub(changed_count)
        ),
        target_path: Some(report_path.to_path_buf()),
        changed: changed_count > 0,
        changed_count,
        requested_count: plans.len(),
    })
}

fn load_story_sync_report(report_path: &Path) -> Result<StorySyncReportInput, StorySyncError> {
    let contents =
        fs::read_to_string(report_path).map_err(|source| StorySyncError::ReadReport {
            path: report_path.to_path_buf(),
            source,
        })?;
    serde_json::from_str(&contents).map_err(|source| StorySyncError::ParseReport {
        path: report_path.to_path_buf(),
        source,
    })
}

fn prepare_story_sync_report_plan(
    workspace: &BookStoryWorkspace,
    direction: StorySyncDirection,
    drift: StoryDriftEntry,
    _kind: StoryEntityKind,
) -> Result<StorySyncReportPlan, StorySyncError> {
    let (source_value, destination_value) = match direction {
        StorySyncDirection::FromShared => (drift.shared_path, drift.book_path),
        StorySyncDirection::ToShared => (drift.book_path, drift.shared_path),
    };
    let source_repo_path = RepoPath::parse(source_value.clone()).map_err(|source| {
        StorySyncError::InvalidReportPath {
            value: source_value.clone(),
            source,
        }
    })?;
    let destination_repo_path = RepoPath::parse(destination_value.clone()).map_err(|source| {
        StorySyncError::InvalidReportPath {
            value: destination_value.clone(),
            source,
        }
    })?;

    let source_path = join_repo_path(&workspace.repo_root, &source_repo_path);
    let target_path = join_repo_path(&workspace.repo_root, &destination_repo_path);
    let source_contents =
        fs::read_to_string(&source_path).map_err(|source| StorySyncError::ReadFile {
            path: source_path.clone(),
            source,
        })?;
    let contents_to_write = match fs::read_to_string(&target_path) {
        Ok(existing) if existing == source_contents => None,
        Ok(_) => Some(source_contents),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Some(source_contents),
        Err(source) => {
            return Err(StorySyncError::ReadFile {
                path: target_path.clone(),
                source,
            });
        }
    };

    Ok(StorySyncReportPlan {
        target_path,
        contents_to_write,
    })
}

fn missing_story_entity_error(
    scope: StoryEntityScope,
    kind: StoryEntityKind,
    id: &str,
) -> StorySyncError {
    match scope {
        StoryEntityScope::Shared => StorySyncError::MissingSharedEntity {
            kind: kind.label().to_string(),
            id: id.to_string(),
        },
        StoryEntityScope::Book => StorySyncError::MissingBookEntity {
            kind: kind.label().to_string(),
            id: id.to_string(),
        },
    }
}

fn conflicting_story_entity_error(scope: StoryEntityScope, id: &str) -> StorySyncError {
    match scope {
        StoryEntityScope::Shared => StorySyncError::SharedEntityConflict { id: id.to_string() },
        StoryEntityScope::Book => StorySyncError::BookEntityConflict { id: id.to_string() },
    }
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

fn load_story_structure_scene_seeds(
    workspace: &BookStoryWorkspace,
    template_name: &str,
) -> Result<(PathBuf, Vec<StoryStructureSceneSeed>), StorySeedError> {
    let template_path =
        resolve_story_structure_template_path(&workspace.story_root, template_name)?;
    if !template_path.is_file() {
        return Err(StorySeedError::MissingTemplate {
            path: template_path,
        });
    }

    let contents =
        fs::read_to_string(&template_path).map_err(|source| StorySeedError::ReadTemplate {
            path: template_path.clone(),
            source,
        })?;
    let frontmatter =
        parse_frontmatter(&contents).map_err(|source| StorySeedError::ParseTemplate {
            path: template_path.clone(),
            detail: source.to_string(),
        })?;
    let Some(frontmatter) = frontmatter else {
        return Err(StorySeedError::MissingSceneSeeds {
            path: template_path,
        });
    };
    let Some(scene_seeds) = mapping_value(&frontmatter, "scene_seeds") else {
        return Err(StorySeedError::MissingSceneSeeds {
            path: template_path,
        });
    };
    let Value::Sequence(values) = scene_seeds else {
        return Err(StorySeedError::InvalidSceneSeed {
            path: template_path,
            index: 1,
            detail: "`scene_seeds` must be a sequence".to_string(),
        });
    };
    if values.is_empty() {
        return Err(StorySeedError::EmptySceneSeeds {
            path: template_path,
        });
    }

    let width = values.len().to_string().len().max(2);
    let mut seen_files = HashSet::new();
    let mut seeds = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let Value::Mapping(mapping) = value else {
            return Err(StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail: "each scene seed must be a mapping".to_string(),
            });
        };

        let title = story_seed_required_string(mapping, "title").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let beat = story_seed_optional_string(mapping, "beat").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let summary = story_seed_optional_string(mapping, "summary").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let file = match story_seed_optional_string(mapping, "file").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })? {
            Some(file) => file,
            None => default_story_scene_seed_file(workspace, index + 1, width),
        };
        let repo_path =
            RepoPath::parse(file.clone()).map_err(|source| StorySeedError::InvalidSceneFile {
                path: template_path.clone(),
                value: file.clone(),
                source,
            })?;
        if !repo_path.as_str().ends_with(".md") {
            return Err(StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail: "`file` must point to a Markdown file".to_string(),
            });
        }
        if !seen_files.insert(repo_path.as_str().to_string()) {
            return Err(StorySeedError::DuplicateSeedFile {
                path: template_path.clone(),
                value: repo_path.as_str().to_string(),
            });
        }

        let characters = story_seed_id_list(mapping, "characters").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let locations = story_seed_id_list(mapping, "locations").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let terms = story_seed_id_list(mapping, "terms").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;
        let factions = story_seed_id_list(mapping, "factions").map_err(|detail| {
            StorySeedError::InvalidSceneSeed {
                path: template_path.clone(),
                index: index + 1,
                detail,
            }
        })?;

        seeds.push(StoryStructureSceneSeed {
            title,
            file: repo_path.as_str().to_string(),
            beat,
            summary,
            characters,
            locations,
            terms,
            factions,
        });
    }

    Ok((template_path, seeds))
}

fn resolve_story_structure_template_path(
    story_root: &Path,
    template_name: &str,
) -> Result<PathBuf, StorySeedError> {
    let trimmed = template_name.trim();
    if trimmed.is_empty() {
        return Err(StorySeedError::InvalidTemplateName {
            value: template_name.to_string(),
        });
    }

    let candidate = Path::new(trimmed);
    if candidate.components().count() != 1 {
        return Err(StorySeedError::InvalidTemplateName {
            value: template_name.to_string(),
        });
    }

    let file_name = if candidate.extension().is_some() {
        candidate.as_os_str().to_string_lossy().into_owned()
    } else {
        format!("{trimmed}.md")
    };
    Ok(story_root.join("structures").join(file_name))
}

fn default_story_scene_seed_file(
    workspace: &BookStoryWorkspace,
    sequence: usize,
    width: usize,
) -> String {
    relative_repo_path(
        &workspace.repo_root,
        &workspace
            .story_root
            .join("scene-notes")
            .join(format!("{sequence:0width$}-scene.md")),
    )
}

fn story_seed_required_string(mapping: &Mapping, key: &str) -> Result<String, String> {
    let Some(value) = mapping_value(mapping, key) else {
        return Err(format!("`{key}` is required"));
    };
    match value {
        Value::String(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        Value::String(_) => Err(format!("`{key}` must not be empty")),
        _ => Err(format!("`{key}` must be a string")),
    }
}

fn story_seed_optional_string(mapping: &Mapping, key: &str) -> Result<Option<String>, String> {
    let Some(value) = mapping_value(mapping, key) else {
        return Ok(None);
    };
    match value {
        Value::String(value) if !value.trim().is_empty() => Ok(Some(value.trim().to_string())),
        Value::String(_) => Err(format!("`{key}` must not be empty when present")),
        _ => Err(format!("`{key}` must be a string")),
    }
}

fn story_seed_id_list(mapping: &Mapping, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = mapping_value(mapping, key) else {
        return Ok(Vec::new());
    };
    match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(format!("`{key}` must not contain an empty id"))
            } else {
                Ok(vec![trimmed.to_string()])
            }
        }
        Value::Sequence(values) => {
            let mut ids = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    Value::String(value) if !value.trim().is_empty() => {
                        ids.push(value.trim().to_string())
                    }
                    Value::String(_) => {
                        return Err(format!("`{key}` must not contain an empty id"));
                    }
                    _ => return Err(format!("`{key}` must contain only strings")),
                }
            }
            Ok(ids)
        }
        _ => Err(format!("`{key}` must be a string or string sequence")),
    }
}

fn seed_story_scenes_file(
    scenes_path: &Path,
    document: &StoryScenesDocument,
    force: bool,
) -> Result<StorySeedWriteAction, StorySeedError> {
    if scenes_path.is_file() {
        let contents =
            fs::read_to_string(scenes_path).map_err(|source| StorySeedError::ReadScenesFile {
                path: scenes_path.to_path_buf(),
                source,
            })?;
        let existing =
            serde_yaml::from_str::<StoryScenesDocument>(&contents).map_err(|source| {
                StorySeedError::ParseScenesFile {
                    path: scenes_path.to_path_buf(),
                    source,
                }
            })?;
        if existing == *document {
            return Ok(StorySeedWriteAction::Kept);
        }
        if !existing.scenes.is_empty() && !force {
            return Err(StorySeedError::ScenesRequireForce {
                path: scenes_path.to_path_buf(),
            });
        }
        write_story_scenes_file_for_seed(scenes_path, document)?;
        return Ok(StorySeedWriteAction::Updated);
    }

    write_story_scenes_file_for_seed(scenes_path, document)?;
    Ok(StorySeedWriteAction::Created)
}

fn write_story_scenes_file_for_seed(
    scenes_path: &Path,
    document: &StoryScenesDocument,
) -> Result<(), StorySeedError> {
    if let Some(parent) = scenes_path.parent() {
        fs::create_dir_all(parent).map_err(|source| StorySeedError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents =
        serde_yaml::to_string(document).map_err(|source| StorySeedError::SerializeYaml {
            path: scenes_path.to_path_buf(),
            source,
        })?;
    fs::write(scenes_path, contents).map_err(|source| StorySeedError::WriteFile {
        path: scenes_path.to_path_buf(),
        source,
    })
}

fn seed_story_note_files(
    workspace: &BookStoryWorkspace,
    template_name: &str,
    seeds: &[StoryStructureSceneSeed],
    force: bool,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>), StorySeedError> {
    let mut created = Vec::new();
    let mut overwritten = Vec::new();
    let mut kept = Vec::new();

    for seed in seeds {
        let repo_path = RepoPath::parse(seed.file.clone()).map_err(|source| {
            StorySeedError::InvalidSceneFile {
                path: workspace.story_root.join("structures"),
                value: seed.file.clone(),
                source,
            }
        })?;
        let note_path = join_repo_path(&workspace.repo_root, &repo_path);
        if note_path.exists() {
            if !note_path.is_file() {
                return Err(StorySeedError::WriteFile {
                    path: note_path,
                    source: std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "story seed destination exists and is not a file",
                    ),
                });
            }
            if !force {
                kept.push(note_path);
                continue;
            }
        }

        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorySeedError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let contents = render_story_scene_seed_note(repo_path.as_str(), template_name, seed)
            .map_err(|source| StorySeedError::SerializeYaml {
                path: note_path.clone(),
                source,
            })?;
        let existed = note_path.is_file();
        fs::write(&note_path, contents).map_err(|source| StorySeedError::WriteFile {
            path: note_path.clone(),
            source,
        })?;
        if existed {
            overwritten.push(note_path);
        } else {
            created.push(note_path);
        }
    }

    Ok((created, overwritten, kept))
}

fn render_story_scene_seed_note(
    note_repo_path: &str,
    template_name: &str,
    seed: &StoryStructureSceneSeed,
) -> Result<String, serde_yaml::Error> {
    let mut frontmatter = Mapping::new();
    frontmatter.insert(
        Value::String("structure_template".to_string()),
        Value::String(template_name.to_string()),
    );
    if let Some(beat) = &seed.beat {
        frontmatter.insert(
            Value::String("structure_beat".to_string()),
            Value::String(beat.clone()),
        );
    }
    insert_story_seed_ids(&mut frontmatter, "characters", &seed.characters);
    insert_story_seed_ids(&mut frontmatter, "locations", &seed.locations);
    insert_story_seed_ids(&mut frontmatter, "terms", &seed.terms);
    insert_story_seed_ids(&mut frontmatter, "factions", &seed.factions);

    let yaml = serde_yaml::to_string(&frontmatter)?;
    let mut lines = vec![
        "---".to_string(),
        yaml.trim_end().to_string(),
        "---".to_string(),
        format!("# {}", seed.title),
        String::new(),
        "## Structure Memo".to_string(),
        format!("- template: {template_name}"),
    ];
    if let Some(beat) = &seed.beat {
        lines.push(format!("- beat: {beat}"));
    }
    if let Some(summary) = &seed.summary {
        lines.push(format!("- summary: {summary}"));
    }
    lines.push(format!("- note: {note_repo_path}"));
    lines.push(String::new());
    lines.push("## Purpose".to_string());
    lines.push("-".to_string());
    lines.push(String::new());
    lines.push("## Conflict".to_string());
    lines.push("-".to_string());
    lines.push(String::new());
    lines.push("## Change".to_string());
    lines.push("-".to_string());
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn insert_story_seed_ids(frontmatter: &mut Mapping, key: &str, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    frontmatter.insert(
        Value::String(key.to_string()),
        Value::Sequence(ids.iter().cloned().map(Value::String).collect::<Vec<_>>()),
    );
}

fn collect_story_entity_catalog(
    workspace: &BookStoryWorkspace,
    collision_mode: StoryEntityCollisionMode,
    issues: &mut Vec<ValidationIssue>,
    drifts: &mut Vec<StoryDriftEntry>,
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
            drifts,
        );
    }
    collect_story_entity_catalog_from_root(
        &workspace.repo_root,
        &workspace.story_root,
        StoryEntityScope::Book,
        collision_mode,
        &mut catalog,
        issues,
        drifts,
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
    drifts: &mut Vec<StoryDriftEntry>,
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
            if let Some(previous) = catalog.map_mut(kind).insert(id.clone(), entry.clone()) {
                if let Some(issue) = story_entity_collision_issue(
                    repo_root,
                    kind,
                    &id,
                    &previous,
                    &entry,
                    collision_mode,
                ) {
                    issues.push(issue);
                }
                if let Some(drift) = story_entity_collision_drift_entry(
                    repo_root,
                    kind,
                    &id,
                    &previous,
                    &entry,
                    collision_mode,
                ) {
                    drifts.push(drift);
                }
            }
        }
    }
}

fn find_story_entity_by_id(
    story_root: &Path,
    scope: StoryEntityScope,
    kind: StoryEntityKind,
    id: &str,
) -> Result<Option<StoryEntityEntry>, StorySyncError> {
    let dir = story_root.join(kind.dir_name());
    if !dir.is_dir() {
        return Ok(None);
    }

    let files = markdown_files_in_dir(&dir).map_err(|source| StorySyncError::ScanDir {
        path: dir.clone(),
        source,
    })?;

    for path in files {
        let contents = fs::read_to_string(&path).map_err(|source| StorySyncError::ReadFile {
            path: path.clone(),
            source,
        })?;
        let frontmatter =
            parse_frontmatter(&contents).map_err(|source| StorySyncError::ParseEntity {
                path: path.clone(),
                detail: source.to_string(),
            })?;
        if story_entity_id(&path, frontmatter.as_ref()).ok().as_deref() == Some(id) {
            return Ok(Some(StoryEntityEntry {
                path,
                scope,
                contents,
            }));
        }
    }

    Ok(None)
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
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map_or(true, |value| !is_reserved_story_markdown(value))
        })
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

fn is_reserved_story_markdown(file_name: &str) -> bool {
    file_name.eq_ignore_ascii_case("README.md") || file_name.eq_ignore_ascii_case("_template.md")
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

    fn from_cli(value: &str) -> Option<Self> {
        match value {
            "character" => Some(Self::Character),
            "location" => Some(Self::Location),
            "term" => Some(Self::Term),
            "faction" => Some(Self::Faction),
            _ => None,
        }
    }

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

impl StoryEntityScope {
    fn label(self) -> &'static str {
        match self {
            StoryEntityScope::Shared => "shared canon",
            StoryEntityScope::Book => "book story data",
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

fn story_entity_collision_drift_entry(
    repo_root: &Path,
    kind: StoryEntityKind,
    id: &str,
    previous: &StoryEntityEntry,
    current: &StoryEntityEntry,
    collision_mode: StoryEntityCollisionMode,
) -> Option<StoryDriftEntry> {
    if previous.scope == current.scope
        || !matches!(collision_mode, StoryEntityCollisionMode::ReportCrossScope)
    {
        return None;
    }

    let (shared, book) = match (previous.scope, current.scope) {
        (StoryEntityScope::Shared, StoryEntityScope::Book) => (previous, current),
        (StoryEntityScope::Book, StoryEntityScope::Shared) => (current, previous),
        _ => return None,
    };

    Some(StoryDriftEntry {
        kind: kind.label().to_string(),
        id: id.to_string(),
        status: if shared.contents == book.contents {
            StoryDriftStatus::RedundantCopy
        } else {
            StoryDriftStatus::Drift
        },
        shared_path: relative_repo_path(repo_root, &shared.path),
        book_path: relative_repo_path(repo_root, &book.path),
    })
}
