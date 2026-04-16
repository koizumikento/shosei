use std::{fs, path::Path};

use crate::{
    app::CONFIG_REFERENCE_URL,
    cli_api::CommandContext,
    config::{self, ExplainedConfig},
    domain::RepoMode,
    editorial,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct ExplainConfigResult {
    pub summary: String,
    pub explained: ExplainedConfig,
    pub snapshot: ExplainConfigSnapshot,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshot {
    pub book_id: String,
    pub repo_mode: String,
    pub repo_root: String,
    pub book_root: String,
    pub config_path: String,
    pub project_type: String,
    pub title: String,
    pub language: String,
    pub profile: String,
    pub writing_mode: String,
    pub reading_direction: String,
    pub binding: String,
    pub outputs: Vec<String>,
    pub values: Vec<ExplainConfigSnapshotValue>,
    pub manuscript: Option<ExplainConfigSnapshotManuscript>,
    pub editorial: Option<ExplainConfigSnapshotEditorial>,
    pub references: ExplainConfigSnapshotReferences,
    pub story: ExplainConfigSnapshotStory,
    pub manga: Option<ExplainConfigSnapshotManga>,
    pub shared_paths: Option<ExplainConfigSnapshotSharedPaths>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotValue {
    pub field: String,
    pub value: String,
    pub origin: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotManuscript {
    pub frontmatter: Vec<String>,
    pub chapters: Vec<String>,
    pub backmatter: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotEditorial {
    pub style_path: Option<String>,
    pub claims_path: Option<String>,
    pub figures_path: Option<String>,
    pub freshness_path: Option<String>,
    pub style_rule_count: usize,
    pub claim_count: usize,
    pub figure_count: usize,
    pub freshness_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotReferences {
    pub current: ExplainConfigSnapshotReferenceWorkspace,
    pub shared: Option<ExplainConfigSnapshotReferenceWorkspace>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotReferenceWorkspace {
    pub scope: String,
    pub references_root: String,
    pub entries_root: String,
    pub initialized: bool,
    pub readme_path: Option<String>,
    pub entries_readme_path: Option<String>,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotStory {
    pub current: ExplainConfigSnapshotStoryWorkspace,
    pub shared: Option<ExplainConfigSnapshotStoryWorkspace>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotStoryWorkspace {
    pub scope: String,
    pub story_root: String,
    pub initialized: bool,
    pub readme_path: Option<String>,
    pub scenes_path: Option<String>,
    pub scene_notes: Option<ExplainConfigSnapshotStorySceneNotes>,
    pub structures: Option<ExplainConfigSnapshotStoryStructures>,
    pub characters: ExplainConfigSnapshotStoryKind,
    pub locations: ExplainConfigSnapshotStoryKind,
    pub terms: ExplainConfigSnapshotStoryKind,
    pub factions: ExplainConfigSnapshotStoryKind,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotStorySceneNotes {
    pub root: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotStoryStructures {
    pub root: String,
    pub readme_path: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotStoryKind {
    pub kind: String,
    pub root: String,
    pub readme_path: Option<String>,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotManga {
    pub reading_direction: String,
    pub default_page_side: String,
    pub spread_policy_for_kindle: String,
    pub front_color_pages: u64,
    pub body_mode: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExplainConfigSnapshotSharedPaths {
    pub assets: Vec<String>,
    pub styles: Vec<String>,
    pub fonts: Vec<String>,
    pub metadata: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExplainConfigError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Editorial(#[from] editorial::EditorialError),
}

pub fn explain_config(command: &CommandContext) -> Result<ExplainConfigResult, ExplainConfigError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let explained = config::explain_book_config(&context)?;
    let editorial = if explained.resolved.effective.project.project_type.is_prose() {
        Some(editorial::load_bundle(&explained.resolved)?)
    } else {
        None
    };
    let book = context.book.as_ref().expect("selected book must exist");
    let mode = match context.mode {
        RepoMode::SingleBook => "single-book",
        RepoMode::Series => "series",
    };
    let snapshot = build_snapshot(
        book.id.as_str(),
        mode,
        &context,
        &explained,
        editorial.as_ref(),
    );
    let has_initialized_references = snapshot.references.current.initialized
        || snapshot
            .references
            .shared
            .as_ref()
            .is_some_and(|workspace| workspace.initialized);
    let has_initialized_story = snapshot.story.current.initialized
        || snapshot
            .story
            .shared
            .as_ref()
            .is_some_and(|workspace| workspace.initialized);

    let mut lines = vec![
        format!("explain for {}", book.id),
        format!("repo mode: {mode}"),
        format!("repo root: {}", context.repo_root.display()),
        format!("book root: {}", book.root.display()),
        format!("config path: {}", book.config_path.display()),
    ];

    let outputs = explained.resolved.outputs();
    lines.push(format!(
        "effective outputs: {}",
        if outputs.is_empty() {
            "none".to_string()
        } else {
            outputs.join(", ")
        }
    ));
    lines.push("".to_string());
    lines.push("resolved values:".to_string());
    for value in &explained.values {
        lines.push(format!(
            "- {} = {} [{}]",
            value.field, value.value, value.origin
        ));
    }

    if let Some(editorial) = &editorial
        && !editorial.is_empty()
    {
        lines.push("".to_string());
        lines.push("editorial summary:".to_string());
        lines.push(format!("- style rules = {}", editorial.style_rule_count()));
        lines.push(format!("- claims = {}", editorial.claim_count()));
        lines.push(format!("- figures = {}", editorial.figure_count()));
        lines.push(format!(
            "- freshness items = {}",
            editorial.freshness_count()
        ));
    }

    if has_initialized_references {
        lines.push("".to_string());
        lines.push("reference summary:".to_string());
        if snapshot.references.current.initialized {
            lines.push(reference_workspace_summary_line(
                if context.mode == RepoMode::Series {
                    "book references"
                } else {
                    "references"
                },
                &snapshot.references.current,
            ));
        }
        if let Some(shared) = &snapshot.references.shared
            && shared.initialized
        {
            lines.push(reference_workspace_summary_line(
                "shared references",
                shared,
            ));
        }
    }

    if has_initialized_story {
        lines.push("".to_string());
        lines.push("story summary:".to_string());
        if snapshot.story.current.initialized {
            lines.push(story_workspace_summary_line(
                if context.mode == RepoMode::Series {
                    "book story"
                } else {
                    "story"
                },
                &snapshot.story.current,
            ));
        }
        if let Some(shared) = &snapshot.story.shared
            && shared.initialized
        {
            lines.push(story_workspace_summary_line("shared story", shared));
        }
    }

    if context.mode == RepoMode::Series {
        lines.push("".to_string());
        lines.push("shared search paths:".to_string());
        lines.push(format!(
            "- assets = {}",
            display_repo_paths(&explained.resolved.shared.assets)
        ));
        lines.push(format!(
            "- styles = {}",
            display_repo_paths(&explained.resolved.shared.styles)
        ));
        lines.push(format!(
            "- fonts = {}",
            display_repo_paths(&explained.resolved.shared.fonts)
        ));
        lines.push(format!(
            "- metadata = {}",
            display_repo_paths(&explained.resolved.shared.metadata)
        ));
    }

    lines.push("".to_string());
    lines.push(format!("config reference: {CONFIG_REFERENCE_URL}"));

    Ok(ExplainConfigResult {
        summary: lines.join("\n"),
        explained,
        snapshot,
    })
}

fn display_repo_paths(paths: &[crate::domain::RepoPath]) -> String {
    if paths.is_empty() {
        "none".to_string()
    } else {
        paths
            .iter()
            .map(|path| path.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn build_snapshot(
    book_id: &str,
    mode: &str,
    context: &crate::domain::RepoContext,
    explained: &ExplainedConfig,
    editorial: Option<&editorial::EditorialBundle>,
) -> ExplainConfigSnapshot {
    let effective = &explained.resolved.effective;
    ExplainConfigSnapshot {
        book_id: book_id.to_string(),
        repo_mode: mode.to_string(),
        repo_root: context.repo_root.display().to_string(),
        book_root: context
            .book
            .as_ref()
            .expect("selected book must exist")
            .root
            .display()
            .to_string(),
        config_path: context
            .book
            .as_ref()
            .expect("selected book must exist")
            .config_path
            .display()
            .to_string(),
        project_type: effective.project.project_type.as_str().to_string(),
        title: effective.book.title.clone(),
        language: effective.book.language.clone(),
        profile: effective.book.profile.clone(),
        writing_mode: match effective.book.writing_mode {
            config::WritingMode::HorizontalLtr => "horizontal-ltr",
            config::WritingMode::VerticalRl => "vertical-rl",
        }
        .to_string(),
        reading_direction: effective.book.reading_direction.as_str().to_string(),
        binding: match effective.layout.binding {
            config::Binding::Left => "left",
            config::Binding::Right => "right",
        }
        .to_string(),
        outputs: explained.resolved.outputs(),
        values: explained
            .values
            .iter()
            .map(|value| ExplainConfigSnapshotValue {
                field: value.field.clone(),
                value: value.value.clone(),
                origin: value.origin.to_string(),
            })
            .collect(),
        manuscript: effective.manuscript.as_ref().map(|manuscript| {
            ExplainConfigSnapshotManuscript {
                frontmatter: manuscript
                    .frontmatter
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
                chapters: manuscript
                    .chapters
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
                backmatter: manuscript
                    .backmatter
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
            }
        }),
        editorial: Some(ExplainConfigSnapshotEditorial {
            style_path: effective
                .editorial
                .style
                .as_ref()
                .map(|path| path.as_str().to_string()),
            claims_path: effective
                .editorial
                .claims
                .as_ref()
                .map(|path| path.as_str().to_string()),
            figures_path: effective
                .editorial
                .figures
                .as_ref()
                .map(|path| path.as_str().to_string()),
            freshness_path: effective
                .editorial
                .freshness
                .as_ref()
                .map(|path| path.as_str().to_string()),
            style_rule_count: editorial
                .map(|bundle| bundle.style_rule_count())
                .unwrap_or(0),
            claim_count: editorial.map(|bundle| bundle.claim_count()).unwrap_or(0),
            figure_count: editorial.map(|bundle| bundle.figure_count()).unwrap_or(0),
            freshness_count: editorial
                .map(|bundle| bundle.freshness_count())
                .unwrap_or(0),
        }),
        references: build_reference_snapshot(context),
        story: build_story_snapshot(context),
        manga: effective
            .manga
            .as_ref()
            .map(|manga| ExplainConfigSnapshotManga {
                reading_direction: manga.reading_direction.as_str().to_string(),
                default_page_side: match manga.default_page_side {
                    config::MangaPageSide::Left => "left",
                    config::MangaPageSide::Right => "right",
                }
                .to_string(),
                spread_policy_for_kindle: match manga.spread_policy_for_kindle {
                    config::SpreadPolicyForKindle::Split => "split",
                    config::SpreadPolicyForKindle::SinglePage => "single-page",
                    config::SpreadPolicyForKindle::Skip => "skip",
                }
                .to_string(),
                front_color_pages: manga.front_color_pages,
                body_mode: match manga.body_mode {
                    config::MangaBodyMode::Monochrome => "monochrome",
                    config::MangaBodyMode::Color => "color",
                    config::MangaBodyMode::Mixed => "mixed",
                }
                .to_string(),
            }),
        shared_paths: if context.mode == RepoMode::Series {
            Some(ExplainConfigSnapshotSharedPaths {
                assets: explained
                    .resolved
                    .shared
                    .assets
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
                styles: explained
                    .resolved
                    .shared
                    .styles
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
                fonts: explained
                    .resolved
                    .shared
                    .fonts
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
                metadata: explained
                    .resolved
                    .shared
                    .metadata
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
            })
        } else {
            None
        },
    }
}

fn build_reference_snapshot(
    context: &crate::domain::RepoContext,
) -> ExplainConfigSnapshotReferences {
    let book = context.book.as_ref().expect("selected book must exist");
    let current = match context.mode {
        RepoMode::SingleBook => build_reference_workspace_snapshot(
            &context.repo_root,
            &context.repo_root.join("references"),
            "single-book",
        ),
        RepoMode::Series => build_reference_workspace_snapshot(
            &context.repo_root,
            &book.root.join("references"),
            "book",
        ),
    };
    let shared = if context.mode == RepoMode::Series {
        Some(build_reference_workspace_snapshot(
            &context.repo_root,
            &context.repo_root.join("shared/metadata/references"),
            "shared",
        ))
    } else {
        None
    };

    ExplainConfigSnapshotReferences { current, shared }
}

fn build_story_snapshot(context: &crate::domain::RepoContext) -> ExplainConfigSnapshotStory {
    let book = context.book.as_ref().expect("selected book must exist");
    let current = match context.mode {
        RepoMode::SingleBook => build_story_workspace_snapshot(
            &context.repo_root,
            &context.repo_root.join("story"),
            "single-book",
        ),
        RepoMode::Series => {
            build_story_workspace_snapshot(&context.repo_root, &book.root.join("story"), "book")
        }
    };
    let shared = if context.mode == RepoMode::Series {
        Some(build_story_workspace_snapshot(
            &context.repo_root,
            &context.repo_root.join("shared/metadata/story"),
            "shared",
        ))
    } else {
        None
    };

    ExplainConfigSnapshotStory { current, shared }
}

fn build_reference_workspace_snapshot(
    repo_root: &Path,
    references_root: &Path,
    scope: &str,
) -> ExplainConfigSnapshotReferenceWorkspace {
    let entries_root = references_root.join("entries");
    let initialized = references_root.is_dir();

    ExplainConfigSnapshotReferenceWorkspace {
        scope: scope.to_string(),
        references_root: relative_snapshot_path(repo_root, references_root),
        entries_root: relative_snapshot_path(repo_root, &entries_root),
        initialized,
        readme_path: optional_snapshot_path(repo_root, &references_root.join("README.md")),
        entries_readme_path: optional_snapshot_path(repo_root, &entries_root.join("README.md")),
        entries: collect_reference_entry_snapshot_paths(repo_root, &entries_root),
    }
}

fn build_story_workspace_snapshot(
    repo_root: &Path,
    story_root: &Path,
    scope: &str,
) -> ExplainConfigSnapshotStoryWorkspace {
    let scene_notes =
        (scope != "shared").then(|| build_story_scene_notes_snapshot(repo_root, story_root));
    let structures =
        (scope != "shared").then(|| build_story_structure_snapshot(repo_root, story_root));

    ExplainConfigSnapshotStoryWorkspace {
        scope: scope.to_string(),
        story_root: relative_snapshot_path(repo_root, story_root),
        initialized: story_root.is_dir(),
        readme_path: optional_snapshot_path(repo_root, &story_root.join("README.md")),
        scenes_path: optional_snapshot_path(repo_root, &story_root.join("scenes.yml")),
        scene_notes,
        structures,
        characters: build_story_kind_snapshot(repo_root, story_root, "characters"),
        locations: build_story_kind_snapshot(repo_root, story_root, "locations"),
        terms: build_story_kind_snapshot(repo_root, story_root, "terms"),
        factions: build_story_kind_snapshot(repo_root, story_root, "factions"),
    }
}

fn build_story_scene_notes_snapshot(
    repo_root: &Path,
    story_root: &Path,
) -> ExplainConfigSnapshotStorySceneNotes {
    let root = story_root.join("scene-notes");

    ExplainConfigSnapshotStorySceneNotes {
        root: relative_snapshot_path(repo_root, &root),
        files: collect_markdown_snapshot_paths(repo_root, &root),
    }
}

fn build_story_structure_snapshot(
    repo_root: &Path,
    story_root: &Path,
) -> ExplainConfigSnapshotStoryStructures {
    let root = story_root.join("structures");

    ExplainConfigSnapshotStoryStructures {
        root: relative_snapshot_path(repo_root, &root),
        readme_path: optional_snapshot_path(repo_root, &root.join("README.md")),
        files: collect_markdown_snapshot_paths(repo_root, &root),
    }
}

fn build_story_kind_snapshot(
    repo_root: &Path,
    story_root: &Path,
    kind: &str,
) -> ExplainConfigSnapshotStoryKind {
    let root = story_root.join(kind);

    ExplainConfigSnapshotStoryKind {
        kind: kind.to_string(),
        root: relative_snapshot_path(repo_root, &root),
        readme_path: optional_snapshot_path(repo_root, &root.join("README.md")),
        entries: collect_story_markdown_snapshot_paths(repo_root, &root),
    }
}

fn collect_reference_entry_snapshot_paths(repo_root: &Path, entries_root: &Path) -> Vec<String> {
    collect_markdown_snapshot_paths(repo_root, entries_root)
}

fn collect_story_markdown_snapshot_paths(repo_root: &Path, root: &Path) -> Vec<String> {
    collect_markdown_snapshot_paths_with_skip(repo_root, root, &["README.md", "_template.md"])
}

fn collect_markdown_snapshot_paths(repo_root: &Path, root: &Path) -> Vec<String> {
    collect_markdown_snapshot_paths_with_skip(repo_root, root, &["README.md"])
}

fn collect_markdown_snapshot_paths_with_skip(
    repo_root: &Path,
    root: &Path,
    skipped_names: &[&str],
) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if skipped_names
            .iter()
            .any(|name| file_name.eq_ignore_ascii_case(name))
        {
            continue;
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("md"))
        {
            paths.push(relative_snapshot_path(repo_root, &path));
        }
    }
    paths.sort();
    paths
}

fn optional_snapshot_path(repo_root: &Path, path: &Path) -> Option<String> {
    path.is_file()
        .then(|| relative_snapshot_path(repo_root, path))
}

fn relative_snapshot_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn reference_workspace_summary_line(
    label: &str,
    workspace: &ExplainConfigSnapshotReferenceWorkspace,
) -> String {
    format!(
        "- {} = {} entry(s) at {}",
        label,
        workspace.entries.len(),
        workspace.entries_root
    )
}

fn story_workspace_summary_line(
    label: &str,
    workspace: &ExplainConfigSnapshotStoryWorkspace,
) -> String {
    let entity_count = workspace.characters.entries.len()
        + workspace.locations.entries.len()
        + workspace.terms.entries.len()
        + workspace.factions.entries.len();
    let scene_note_suffix = workspace
        .scene_notes
        .as_ref()
        .filter(|scene_notes| !scene_notes.files.is_empty())
        .map(|scene_notes| format!(", scene notes: {}", scene_notes.files.len()))
        .unwrap_or_default();
    let structure_suffix = workspace
        .structures
        .as_ref()
        .filter(|structures| !structures.files.is_empty())
        .map(|structures| format!(", structure files: {}", structures.files.len()))
        .unwrap_or_default();
    match &workspace.scenes_path {
        Some(scenes_path) => format!(
            "- {} = {} entity file(s) at {}, scenes: {}{}{}",
            label,
            entity_count,
            workspace.story_root,
            scenes_path,
            scene_note_suffix,
            structure_suffix
        ),
        None => format!(
            "- {} = {} entity file(s) at {}{}{}",
            label, entity_count, workspace.story_root, scene_note_suffix, structure_suffix
        ),
    }
}
