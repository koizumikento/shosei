use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions, Permissions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config::{self, BookConfig},
    domain::{ProjectType, RepoPath, RepoPathError},
    fs::join_repo_path,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct ChapterAddOptions {
    pub chapter_path: String,
    pub title: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChapterMoveOptions {
    pub chapter_path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChapterRemoveOptions {
    pub chapter_path: String,
    pub delete_file: bool,
}

#[derive(Debug, Clone)]
pub struct ChapterRenumberOptions {
    pub start_at: usize,
    pub width: usize,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ChapterResult {
    pub summary: String,
    pub config_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ChapterError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("chapter commands are only supported for prose projects, got {project_type}")]
    UnsupportedProjectType { project_type: ProjectType },
    #[error("invalid chapter path `{value}`: {source}")]
    InvalidChapterPath {
        value: String,
        #[source]
        source: RepoPathError,
    },
    #[error("chapter path `{value}` must reference a .md file")]
    ChapterPathMustBeMarkdown { value: String },
    #[error("use either --before or --after, not both")]
    ConflictingPlacement,
    #[error("chapter move requires exactly one of --before or --after")]
    MissingPlacement,
    #[error("chapter `{path}` is already present in manuscript")]
    ChapterAlreadyExists { path: String },
    #[error("chapter `{path}` was not found in manuscript.chapters")]
    ChapterNotFound { path: String },
    #[error("reference chapter `{path}` was not found in manuscript.chapters")]
    ReferenceChapterNotFound { path: String },
    #[error("reference chapter must differ from target chapter `{path}`")]
    ReferenceMatchesTarget { path: String },
    #[error("cannot remove the last remaining chapter")]
    CannotRemoveLastChapter,
    #[error("renumber width must be at least 1")]
    InvalidRenumberWidth,
    #[error("renumber start-at must be at least 1")]
    InvalidRenumberStartAt,
    #[error("chapter file `{path}` does not exist; pass --title to create a new stub")]
    MissingChapterFile { path: PathBuf },
    #[error("chapter source file `{path}` does not exist")]
    MissingChapterSourceFile { path: PathBuf },
    #[error("failed to create chapter file {path}: {source}")]
    CreateChapterFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write chapter config to {path}: {source}")]
    WriteConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize chapter config for {path}: {source}")]
    SerializeConfig {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to resolve chapter renumber path {path}: {source}")]
    ResolveRenumberPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("chapter config {path} resolves outside repository {repo_root}: {resolved_path}")]
    RenumberConfigOutsideRepository {
        path: PathBuf,
        resolved_path: PathBuf,
        repo_root: PathBuf,
    },
    #[error("failed to delete chapter file {path}: {source}")]
    DeleteChapterFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("chapter renumber would overwrite existing file {path}")]
    ChapterRenameConflict { path: PathBuf },
    #[error("failed to rename chapter file {from} -> {to}: {source}")]
    RenameChapterFile {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create chapter renumber staging directory {path}: {source}")]
    CreateRenumberStagingDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to rename chapter config {from} -> {to}: {source}")]
    RenameChapterConfig {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to remove chapter renumber temporary path {path}: {source}")]
    RemoveRenumberTemporaryPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("chapter renumber failed: {primary}; rollback was incomplete: {rollback_failures}")]
    RenumberRollbackFailed {
        #[source]
        primary: Box<ChapterError>,
        rollback_failures: String,
    },
}

pub fn chapter_add(
    command: &CommandContext,
    options: ChapterAddOptions,
) -> Result<ChapterResult, ChapterError> {
    if options.before.is_some() && options.after.is_some() {
        return Err(ChapterError::ConflictingPlacement);
    }

    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    ensure_prose_project(resolved.effective.project.project_type)?;
    let book = context.book.expect("selected book must exist");
    let target = parse_markdown_repo_path(&options.chapter_path)?;
    let mut chapters = resolved
        .effective
        .manuscript
        .as_ref()
        .expect("prose project must have manuscript")
        .chapters
        .clone();
    let all_manuscript_paths = resolved.manuscript_files();
    if all_manuscript_paths.iter().any(|path| path == &target) {
        return Err(ChapterError::ChapterAlreadyExists {
            path: target.as_str().to_string(),
        });
    }

    let insert_at = placement_index(
        &chapters,
        options.before.as_deref(),
        options.after.as_deref(),
    )?;
    chapters.insert(insert_at, target.clone());

    let chapter_file_path = join_repo_path(&context.repo_root, &target);
    let file_created = ensure_chapter_file(&chapter_file_path, options.title.as_deref())?;

    let mut book_config = config::load_book_config(&book.config_path)?;
    overwrite_chapters(&mut book_config.raw, &chapters);
    write_book_config(&book_config)?;

    Ok(ChapterResult {
        summary: format!(
            "chapter add: {} updated {}\n- inserted {} at position {}\n- file {}",
            book.id,
            book.config_path.display(),
            target.as_str(),
            insert_at + 1,
            if file_created {
                format!("created {}", chapter_file_path.display())
            } else {
                format!("kept {}", chapter_file_path.display())
            }
        ),
        config_path: book.config_path,
    })
}

pub fn chapter_move(
    command: &CommandContext,
    options: ChapterMoveOptions,
) -> Result<ChapterResult, ChapterError> {
    if options.before.is_some() && options.after.is_some() {
        return Err(ChapterError::ConflictingPlacement);
    }
    if options.before.is_none() && options.after.is_none() {
        return Err(ChapterError::MissingPlacement);
    }

    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    ensure_prose_project(resolved.effective.project.project_type)?;
    let book = context.book.expect("selected book must exist");
    let target = parse_markdown_repo_path(&options.chapter_path)?;
    let mut chapters = resolved
        .effective
        .manuscript
        .as_ref()
        .expect("prose project must have manuscript")
        .chapters
        .clone();

    let current_index = chapters
        .iter()
        .position(|path| path == &target)
        .ok_or_else(|| ChapterError::ChapterNotFound {
            path: target.as_str().to_string(),
        })?;
    chapters.remove(current_index);

    let reference = options
        .before
        .as_deref()
        .or(options.after.as_deref())
        .expect("placement is required");
    let reference = parse_markdown_repo_path(reference)?;
    if reference == target {
        return Err(ChapterError::ReferenceMatchesTarget {
            path: target.as_str().to_string(),
        });
    }

    let insert_at = placement_index(
        &chapters,
        options.before.as_deref(),
        options.after.as_deref(),
    )?;
    chapters.insert(insert_at, target.clone());

    let mut book_config = config::load_book_config(&book.config_path)?;
    overwrite_chapters(&mut book_config.raw, &chapters);
    write_book_config(&book_config)?;

    Ok(ChapterResult {
        summary: format!(
            "chapter move: {} updated {}\n- moved {} to position {}",
            book.id,
            book.config_path.display(),
            target.as_str(),
            insert_at + 1,
        ),
        config_path: book.config_path,
    })
}

pub fn chapter_remove(
    command: &CommandContext,
    options: ChapterRemoveOptions,
) -> Result<ChapterResult, ChapterError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    ensure_prose_project(resolved.effective.project.project_type)?;
    let book = context.book.expect("selected book must exist");
    let target = parse_markdown_repo_path(&options.chapter_path)?;
    let mut chapters = resolved
        .effective
        .manuscript
        .as_ref()
        .expect("prose project must have manuscript")
        .chapters
        .clone();

    let current_index = chapters
        .iter()
        .position(|path| path == &target)
        .ok_or_else(|| ChapterError::ChapterNotFound {
            path: target.as_str().to_string(),
        })?;
    if chapters.len() == 1 {
        return Err(ChapterError::CannotRemoveLastChapter);
    }
    chapters.remove(current_index);

    let mut book_config = config::load_book_config(&book.config_path)?;
    overwrite_chapters(&mut book_config.raw, &chapters);
    prune_sections_for_path(&mut book_config.raw, &resolved.raw, &target);
    write_book_config(&book_config)?;

    let chapter_file_path = join_repo_path(&context.repo_root, &target);
    let file_status = if options.delete_file {
        if chapter_file_path.exists() {
            fs::remove_file(&chapter_file_path).map_err(|source| {
                ChapterError::DeleteChapterFile {
                    path: chapter_file_path.clone(),
                    source,
                }
            })?;
            format!("deleted {}", chapter_file_path.display())
        } else {
            format!("already absent {}", chapter_file_path.display())
        }
    } else {
        format!("kept {}", chapter_file_path.display())
    };

    Ok(ChapterResult {
        summary: format!(
            "chapter remove: {} updated {}\n- removed {}\n- file {}",
            book.id,
            book.config_path.display(),
            target.as_str(),
            file_status
        ),
        config_path: book.config_path,
    })
}

pub fn chapter_renumber(
    command: &CommandContext,
    options: ChapterRenumberOptions,
) -> Result<ChapterResult, ChapterError> {
    chapter_renumber_with(command, options, &StdRenumberFileOps, serde_yaml::to_string)
}

fn chapter_renumber_with<O, S>(
    command: &CommandContext,
    options: ChapterRenumberOptions,
    file_ops: &O,
    serialize: S,
) -> Result<ChapterResult, ChapterError>
where
    O: RenumberFileOps,
    S: FnOnce(&Value) -> Result<String, serde_yaml::Error>,
{
    if options.width == 0 {
        return Err(ChapterError::InvalidRenumberWidth);
    }
    if options.start_at == 0 {
        return Err(ChapterError::InvalidRenumberStartAt);
    }

    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    ensure_prose_project(resolved.effective.project.project_type)?;
    let book = context.book.expect("selected book must exist");
    let chapters = resolved
        .effective
        .manuscript
        .as_ref()
        .expect("prose project must have manuscript")
        .chapters
        .clone();
    let plans = build_renumber_plan(
        &context.repo_root,
        &chapters,
        options.start_at,
        options.width,
    )?;

    if plans.iter().all(|plan| plan.from_repo == plan.to_repo) {
        return Ok(ChapterResult {
            summary: format!(
                "chapter renumber: {} no changes required in {}",
                book.id,
                book.config_path.display()
            ),
            config_path: book.config_path,
        });
    }

    validate_renumber_targets(&plans)?;

    if options.dry_run {
        return Ok(ChapterResult {
            summary: format!(
                "chapter renumber dry-run: {} would update {}\n{}",
                book.id,
                book.config_path.display(),
                render_renumber_lines(&plans, "would rename")
            ),
            config_path: book.config_path,
        });
    }

    let transaction_config_path =
        renumber_config_transaction_path(&context.repo_root, &book.config_path)?;
    let mut book_config = config::load_book_config(&transaction_config_path)?;
    overwrite_chapters(
        &mut book_config.raw,
        &plans
            .iter()
            .map(|plan| plan.to_repo.clone())
            .collect::<Vec<_>>(),
    );
    rewrite_sections_paths(&mut book_config.raw, &resolved.raw, &rename_map(&plans));
    let rendered_config = render_book_config_with(&book_config, serialize)?;

    apply_renumber_transaction(
        &plans,
        &book_config.path,
        rendered_config.as_bytes(),
        file_ops,
    )?;

    Ok(ChapterResult {
        summary: format!(
            "chapter renumber: {} updated {}\n{}",
            book.id,
            book.config_path.display(),
            render_renumber_lines(&plans, "renamed")
        ),
        config_path: book.config_path,
    })
}

fn ensure_prose_project(project_type: ProjectType) -> Result<(), ChapterError> {
    if !project_type.is_prose() {
        return Err(ChapterError::UnsupportedProjectType { project_type });
    }
    Ok(())
}

fn parse_markdown_repo_path(value: &str) -> Result<RepoPath, ChapterError> {
    let path =
        RepoPath::parse(value.to_string()).map_err(|source| ChapterError::InvalidChapterPath {
            value: value.to_string(),
            source,
        })?;
    if !path.as_str().ends_with(".md") {
        return Err(ChapterError::ChapterPathMustBeMarkdown {
            value: value.to_string(),
        });
    }
    Ok(path)
}

fn placement_index(
    chapters: &[RepoPath],
    before: Option<&str>,
    after: Option<&str>,
) -> Result<usize, ChapterError> {
    if let Some(path) = before {
        let reference = parse_markdown_repo_path(path)?;
        chapters
            .iter()
            .position(|chapter| chapter == &reference)
            .ok_or_else(|| ChapterError::ReferenceChapterNotFound {
                path: reference.as_str().to_string(),
            })
    } else if let Some(path) = after {
        let reference = parse_markdown_repo_path(path)?;
        chapters
            .iter()
            .position(|chapter| chapter == &reference)
            .map(|index| index + 1)
            .ok_or_else(|| ChapterError::ReferenceChapterNotFound {
                path: reference.as_str().to_string(),
            })
    } else {
        Ok(chapters.len())
    }
}

fn ensure_chapter_file(path: &Path, title: Option<&str>) -> Result<bool, ChapterError> {
    if path.exists() {
        return Ok(false);
    }
    let title = title.ok_or_else(|| ChapterError::MissingChapterFile {
        path: path.to_path_buf(),
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ChapterError::CreateChapterFile {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, format!("# {title}\n")).map_err(|source| ChapterError::CreateChapterFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}

fn overwrite_chapters(root: &mut Value, chapters: &[RepoPath]) {
    let root_mapping = ensure_mapping(root);
    let manuscript = root_mapping
        .entry(Value::String("manuscript".to_string()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let manuscript_mapping = ensure_mapping(manuscript);
    manuscript_mapping.insert(
        Value::String("chapters".to_string()),
        Value::Sequence(
            chapters
                .iter()
                .map(|path| Value::String(path.as_str().to_string()))
                .collect(),
        ),
    );
}

fn prune_sections_for_path(book_raw: &mut Value, resolved_raw: &Value, target: &RepoPath) {
    let Some(sections) = lookup(resolved_raw, &["sections"]).and_then(Value::as_sequence) else {
        return;
    };
    let pruned: Vec<Value> = sections
        .iter()
        .filter(|section| {
            lookup(section, &["file"])
                .and_then(Value::as_str)
                .map(|value| value != target.as_str())
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    if pruned.len() == sections.len() {
        return;
    }

    let root_mapping = ensure_mapping(book_raw);
    root_mapping.insert(
        Value::String("sections".to_string()),
        Value::Sequence(pruned),
    );
}

fn rewrite_sections_paths(
    book_raw: &mut Value,
    resolved_raw: &Value,
    rename_map: &HashMap<String, String>,
) {
    let Some(sections) = lookup(resolved_raw, &["sections"]).and_then(Value::as_sequence) else {
        return;
    };
    let mut changed = false;
    let rewritten: Vec<Value> = sections
        .iter()
        .map(|section| {
            let mut section = section.clone();
            if let Some(file) = lookup(&section, &["file"]).and_then(Value::as_str)
                && let Some(new_file) = rename_map.get(file)
                && let Some(mapping) = section.as_mapping_mut()
            {
                mapping.insert(
                    Value::String("file".to_string()),
                    Value::String(new_file.clone()),
                );
                changed = true;
            }
            section
        })
        .collect();
    if !changed {
        return;
    }

    let root_mapping = ensure_mapping(book_raw);
    root_mapping.insert(
        Value::String("sections".to_string()),
        Value::Sequence(rewritten),
    );
}

fn ensure_mapping(value: &mut Value) -> &mut Mapping {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    match value {
        Value::Mapping(mapping) => mapping,
        _ => unreachable!(),
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

fn write_book_config(book_config: &BookConfig) -> Result<(), ChapterError> {
    let rendered = render_book_config_with(book_config, serde_yaml::to_string)?;
    fs::write(&book_config.path, rendered).map_err(|source| ChapterError::WriteConfig {
        path: book_config.path.clone(),
        source,
    })
}

fn render_book_config_with<S>(
    book_config: &BookConfig,
    serialize: S,
) -> Result<String, ChapterError>
where
    S: FnOnce(&Value) -> Result<String, serde_yaml::Error>,
{
    let mut rendered =
        serialize(&book_config.raw).map_err(|source| ChapterError::SerializeConfig {
            path: book_config.path.clone(),
            source,
        })?;
    if let Some(stripped) = rendered.strip_prefix("---\n") {
        rendered = stripped.to_string();
    }
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

fn renumber_config_transaction_path(
    repo_root: &Path,
    config_path: &Path,
) -> Result<PathBuf, ChapterError> {
    let resolved_repo_root =
        fs::canonicalize(repo_root).map_err(|source| ChapterError::ResolveRenumberPath {
            path: repo_root.to_path_buf(),
            source,
        })?;
    let resolved_config_path =
        fs::canonicalize(config_path).map_err(|source| ChapterError::ResolveRenumberPath {
            path: config_path.to_path_buf(),
            source,
        })?;

    if !resolved_config_path.starts_with(&resolved_repo_root) {
        return Err(ChapterError::RenumberConfigOutsideRepository {
            path: config_path.to_path_buf(),
            resolved_path: resolved_config_path,
            repo_root: resolved_repo_root,
        });
    }

    Ok(resolved_config_path)
}

#[derive(Debug, Clone)]
struct RenumberPlan {
    from_repo: RepoPath,
    to_repo: RepoPath,
    from_fs: PathBuf,
    to_fs: PathBuf,
}

fn build_renumber_plan(
    repo_root: &Path,
    chapters: &[RepoPath],
    start_at: usize,
    width: usize,
) -> Result<Vec<RenumberPlan>, ChapterError> {
    chapters
        .iter()
        .enumerate()
        .map(|(index, chapter)| {
            let number = start_at + index;
            let to_repo = renumbered_repo_path(chapter, number, width)?;
            Ok(RenumberPlan {
                from_fs: join_repo_path(repo_root, chapter),
                to_fs: join_repo_path(repo_root, &to_repo),
                from_repo: chapter.clone(),
                to_repo,
            })
        })
        .collect()
}

fn renumbered_repo_path(
    chapter: &RepoPath,
    number: usize,
    width: usize,
) -> Result<RepoPath, ChapterError> {
    let path = Path::new(chapter.as_str());
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("repo path must have valid UTF-8 stem");
    let suffix = renumber_suffix(stem);
    let numbered = format!("{number:0width$}");
    let new_file_name = match suffix {
        Some(suffix) => format!("{numbered}-{suffix}.md"),
        None => format!("{numbered}.md"),
    };
    let new_path = match chapter.as_str().rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{new_file_name}"),
        None => new_file_name,
    };
    RepoPath::parse(new_path).map_err(|source| ChapterError::InvalidChapterPath {
        value: chapter.as_str().to_string(),
        source,
    })
}

fn renumber_suffix(stem: &str) -> Option<&str> {
    let digit_prefix_len = stem
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_prefix_len == stem.len() {
        return None;
    }
    if digit_prefix_len > 0
        && stem.as_bytes().get(digit_prefix_len) == Some(&b'-')
        && digit_prefix_len + 1 < stem.len()
    {
        return Some(&stem[digit_prefix_len + 1..]);
    }
    Some(stem)
}

fn validate_renumber_targets(plans: &[RenumberPlan]) -> Result<(), ChapterError> {
    let changing_plans: Vec<&RenumberPlan> = plans
        .iter()
        .filter(|plan| plan.from_repo != plan.to_repo)
        .collect();
    let changing_sources: HashSet<PathBuf> = changing_plans
        .iter()
        .map(|plan| plan.from_fs.clone())
        .collect();
    let mut seen_targets = HashSet::new();

    for plan in &changing_plans {
        if !plan.from_fs.exists() {
            return Err(ChapterError::MissingChapterSourceFile {
                path: plan.from_fs.clone(),
            });
        }
        if !seen_targets.insert(plan.to_fs.clone()) {
            return Err(ChapterError::ChapterRenameConflict {
                path: plan.to_fs.clone(),
            });
        }
        if plan.to_fs.exists() && !changing_sources.contains(&plan.to_fs) {
            return Err(ChapterError::ChapterRenameConflict {
                path: plan.to_fs.clone(),
            });
        }
    }
    Ok(())
}

const MAX_STAGING_DIRECTORY_ATTEMPTS: usize = 1_024;
static NEXT_STAGING_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenumberRenameStep {
    StageChapter,
    PublishChapter,
    RestageChapter,
    RestoreChapter,
    BackupConfig,
    InstallConfig,
    UninstallConfig,
    RestoreConfig,
}

trait RenumberFileOps {
    fn create_dir(&self, path: &Path) -> io::Result<()>;
    fn write_new_file(
        &self,
        path: &Path,
        contents: &[u8],
        permissions: Permissions,
    ) -> io::Result<()>;
    fn rename(&self, step: RenumberRenameStep, from: &Path, to: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn remove_dir(&self, path: &Path) -> io::Result<()>;
}

struct StdRenumberFileOps;

impl RenumberFileOps for StdRenumberFileOps {
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn write_new_file(
        &self,
        path: &Path,
        contents: &[u8],
        permissions: Permissions,
    ) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        file.set_permissions(permissions)
    }

    fn rename(&self, _step: RenumberRenameStep, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenumberEntryLocation {
    Source,
    Staged,
    Target,
}

struct RenumberEntry {
    plan: RenumberPlan,
    staged_path: PathBuf,
    location: RenumberEntryLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenumberConfigState {
    Original,
    BackedUp,
    Installed,
    Committed,
}

struct RenumberTransaction<'a, O> {
    file_ops: &'a O,
    entries: Vec<RenumberEntry>,
    staging_directories: Vec<PathBuf>,
    config_path: PathBuf,
    staged_config_path: PathBuf,
    backup_config_path: PathBuf,
    config_state: RenumberConfigState,
}

fn apply_renumber_transaction<O>(
    plans: &[RenumberPlan],
    config_path: &Path,
    rendered_config: &[u8],
    file_ops: &O,
) -> Result<(), ChapterError>
where
    O: RenumberFileOps,
{
    let config_permissions = fs::metadata(config_path)
        .map_err(|source| ChapterError::WriteConfig {
            path: config_path.to_path_buf(),
            source,
        })?
        .permissions();
    let mut staging_by_parent = HashMap::new();
    let mut staging_directories = Vec::new();
    let mut entries = Vec::new();

    for (index, plan) in plans
        .iter()
        .filter(|plan| plan.from_repo != plan.to_repo)
        .enumerate()
    {
        let parent = plan
            .from_fs
            .parent()
            .expect("chapter path should have a parent directory");
        let staging_directory = match staging_directory_for_parent(
            parent,
            &mut staging_by_parent,
            &mut staging_directories,
            file_ops,
        ) {
            Ok(path) => path,
            Err(primary) => {
                let rollback_failures =
                    remove_empty_staging_directories(file_ops, &staging_directories);
                return Err(with_rollback_failures(primary, rollback_failures));
            }
        };
        entries.push(RenumberEntry {
            plan: plan.clone(),
            staged_path: staging_directory.join(format!("chapter-{index}")),
            location: RenumberEntryLocation::Source,
        });
    }

    let config_parent = config_path
        .parent()
        .expect("book config path should have a parent directory");
    let config_staging_directory = match staging_directory_for_parent(
        config_parent,
        &mut staging_by_parent,
        &mut staging_directories,
        file_ops,
    ) {
        Ok(path) => path,
        Err(primary) => {
            let rollback_failures =
                remove_empty_staging_directories(file_ops, &staging_directories);
            return Err(with_rollback_failures(primary, rollback_failures));
        }
    };
    let staged_config_path = config_staging_directory.join("book-config.new");
    let backup_config_path = config_staging_directory.join("book-config.original");

    let mut transaction = RenumberTransaction {
        file_ops,
        entries,
        staging_directories,
        config_path: config_path.to_path_buf(),
        staged_config_path,
        backup_config_path,
        config_state: RenumberConfigState::Original,
    };

    if let Err(source) = transaction.file_ops.write_new_file(
        &transaction.staged_config_path,
        rendered_config,
        config_permissions,
    ) {
        let primary = ChapterError::WriteConfig {
            path: transaction.config_path.clone(),
            source,
        };
        return Err(transaction.rollback_after(primary));
    }

    transaction.run()
}

impl<O> RenumberTransaction<'_, O>
where
    O: RenumberFileOps,
{
    fn run(mut self) -> Result<(), ChapterError> {
        if let Err(primary) = self.stage_chapters() {
            return Err(self.rollback_after(primary));
        }
        if let Err(primary) = self.publish_chapters() {
            return Err(self.rollback_after(primary));
        }

        if let Err(source) = self.file_ops.rename(
            RenumberRenameStep::BackupConfig,
            &self.config_path,
            &self.backup_config_path,
        ) {
            let primary = ChapterError::RenameChapterConfig {
                from: self.config_path.clone(),
                to: self.backup_config_path.clone(),
                source,
            };
            return Err(self.rollback_after(primary));
        }
        self.config_state = RenumberConfigState::BackedUp;

        if let Err(source) = self.file_ops.rename(
            RenumberRenameStep::InstallConfig,
            &self.staged_config_path,
            &self.config_path,
        ) {
            let primary = ChapterError::RenameChapterConfig {
                from: self.staged_config_path.clone(),
                to: self.config_path.clone(),
                source,
            };
            return Err(self.rollback_after(primary));
        }
        self.config_state = RenumberConfigState::Installed;

        if let Err(source) = self.file_ops.remove_file(&self.backup_config_path) {
            let primary = ChapterError::RemoveRenumberTemporaryPath {
                path: self.backup_config_path.clone(),
                source,
            };
            return Err(self.rollback_after(primary));
        }
        self.config_state = RenumberConfigState::Committed;

        // The data transaction is committed once the original config backup is removed.
        // Empty-directory cleanup is best effort and never removes recursively, so a
        // recovery copy cannot be deleted by cleanup.
        let _ = remove_empty_staging_directories(self.file_ops, &self.staging_directories);
        Ok(())
    }

    fn stage_chapters(&mut self) -> Result<(), ChapterError> {
        for index in 0..self.entries.len() {
            let from = self.entries[index].plan.from_fs.clone();
            let to = self.entries[index].staged_path.clone();
            self.file_ops
                .rename(RenumberRenameStep::StageChapter, &from, &to)
                .map_err(|source| ChapterError::RenameChapterFile { from, to, source })?;
            self.entries[index].location = RenumberEntryLocation::Staged;
        }
        Ok(())
    }

    fn publish_chapters(&mut self) -> Result<(), ChapterError> {
        for index in 0..self.entries.len() {
            let from = self.entries[index].staged_path.clone();
            let to = self.entries[index].plan.to_fs.clone();
            self.file_ops
                .rename(RenumberRenameStep::PublishChapter, &from, &to)
                .map_err(|source| ChapterError::RenameChapterFile { from, to, source })?;
            self.entries[index].location = RenumberEntryLocation::Target;
        }
        Ok(())
    }

    fn rollback_after(&mut self, primary: ChapterError) -> ChapterError {
        let mut rollback_failures = Vec::new();
        self.rollback_config(&mut rollback_failures);
        self.rollback_chapters(&mut rollback_failures);

        if self.config_state == RenumberConfigState::Original {
            match self.file_ops.remove_file(&self.staged_config_path) {
                Ok(()) => {}
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(source) => rollback_failures.push(format!(
                    "remove {}: {source}",
                    self.staged_config_path.display()
                )),
            }
        }
        rollback_failures.extend(remove_empty_staging_directories(
            self.file_ops,
            &self.staging_directories,
        ));

        with_rollback_failures(primary, rollback_failures)
    }

    fn rollback_config(&mut self, rollback_failures: &mut Vec<String>) {
        if self.config_state == RenumberConfigState::Installed {
            match self.file_ops.rename(
                RenumberRenameStep::UninstallConfig,
                &self.config_path,
                &self.staged_config_path,
            ) {
                Ok(()) => self.config_state = RenumberConfigState::BackedUp,
                Err(source) => {
                    rollback_failures.push(format!(
                        "restore staged config {} -> {}: {source}",
                        self.config_path.display(),
                        self.staged_config_path.display()
                    ));
                    return;
                }
            }
        }

        if self.config_state == RenumberConfigState::BackedUp {
            match self.file_ops.rename(
                RenumberRenameStep::RestoreConfig,
                &self.backup_config_path,
                &self.config_path,
            ) {
                Ok(()) => self.config_state = RenumberConfigState::Original,
                Err(source) => rollback_failures.push(format!(
                    "restore config {} -> {}: {source}",
                    self.backup_config_path.display(),
                    self.config_path.display()
                )),
            }
        }
    }

    fn rollback_chapters(&mut self, rollback_failures: &mut Vec<String>) {
        // Targets must first return to their unique staging slots. Moving a target
        // directly to its source can collide when the rename plan contains a cycle.
        for index in 0..self.entries.len() {
            if self.entries[index].location != RenumberEntryLocation::Target {
                continue;
            }
            let from = self.entries[index].plan.to_fs.clone();
            let to = self.entries[index].staged_path.clone();
            match self
                .file_ops
                .rename(RenumberRenameStep::RestageChapter, &from, &to)
            {
                Ok(()) => self.entries[index].location = RenumberEntryLocation::Staged,
                Err(source) => rollback_failures.push(format!(
                    "restage chapter {} -> {}: {source}",
                    from.display(),
                    to.display()
                )),
            }
        }

        for index in (0..self.entries.len()).rev() {
            if self.entries[index].location != RenumberEntryLocation::Staged {
                continue;
            }
            let from = self.entries[index].staged_path.clone();
            let to = self.entries[index].plan.from_fs.clone();
            match self
                .file_ops
                .rename(RenumberRenameStep::RestoreChapter, &from, &to)
            {
                Ok(()) => self.entries[index].location = RenumberEntryLocation::Source,
                Err(source) => rollback_failures.push(format!(
                    "restore chapter {} -> {}: {source}",
                    from.display(),
                    to.display()
                )),
            }
        }
    }
}

fn staging_directory_for_parent<O>(
    parent: &Path,
    staging_by_parent: &mut HashMap<PathBuf, PathBuf>,
    staging_directories: &mut Vec<PathBuf>,
    file_ops: &O,
) -> Result<PathBuf, ChapterError>
where
    O: RenumberFileOps,
{
    if let Some(path) = staging_by_parent.get(parent) {
        return Ok(path.clone());
    }

    let path = reserve_staging_directory_with(parent, file_ops, || {
        NEXT_STAGING_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed)
    })?;
    staging_by_parent.insert(parent.to_path_buf(), path.clone());
    staging_directories.push(path.clone());
    Ok(path)
}

fn reserve_staging_directory_with<O, F>(
    parent: &Path,
    file_ops: &O,
    mut next_id: F,
) -> Result<PathBuf, ChapterError>
where
    O: RenumberFileOps,
    F: FnMut() -> u64,
{
    let mut last_candidate = None;
    for _ in 0..MAX_STAGING_DIRECTORY_ATTEMPTS {
        let candidate = parent.join(format!(
            ".shosei-renumber-{}-{}",
            std::process::id(),
            next_id()
        ));
        match file_ops.create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                last_candidate = Some(candidate);
            }
            Err(source) => {
                return Err(ChapterError::CreateRenumberStagingDirectory {
                    path: candidate,
                    source,
                });
            }
        }
    }

    let path = last_candidate.unwrap_or_else(|| parent.to_path_buf());
    Err(ChapterError::CreateRenumberStagingDirectory {
        path,
        source: io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a unique chapter renumber staging directory",
        ),
    })
}

fn remove_empty_staging_directories<O>(file_ops: &O, staging_directories: &[PathBuf]) -> Vec<String>
where
    O: RenumberFileOps,
{
    let mut failures = Vec::new();
    for path in staging_directories.iter().rev() {
        match file_ops.remove_dir(path) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => failures.push(format!("remove directory {}: {source}", path.display())),
        }
    }
    failures
}

fn with_rollback_failures(primary: ChapterError, rollback_failures: Vec<String>) -> ChapterError {
    if rollback_failures.is_empty() {
        primary
    } else {
        ChapterError::RenumberRollbackFailed {
            primary: Box::new(primary),
            rollback_failures: rollback_failures.join("; "),
        }
    }
}

fn rename_map(plans: &[RenumberPlan]) -> HashMap<String, String> {
    plans
        .iter()
        .filter(|plan| plan.from_repo != plan.to_repo)
        .map(|plan| {
            (
                plan.from_repo.as_str().to_string(),
                plan.to_repo.as_str().to_string(),
            )
        })
        .collect()
}

fn render_renumber_lines(plans: &[RenumberPlan], verb: &str) -> String {
    plans
        .iter()
        .filter(|plan| plan.from_repo != plan.to_repo)
        .map(|plan| {
            format!(
                "- {verb} {} -> {}",
                plan.from_repo.as_str(),
                plan.to_repo.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        fs, io,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{
        ChapterError, ChapterRenumberOptions, RenumberFileOps, RenumberRenameStep,
        StdRenumberFileOps, chapter_renumber_with, parse_markdown_repo_path, placement_index,
        renumber_suffix, renumbered_repo_path, reserve_staging_directory_with,
    };
    use crate::{cli_api::CommandContext, domain::RepoPath};

    static NEXT_TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct FaultingRenumberFileOps {
        fail_rename: Option<(RenumberRenameStep, usize)>,
        fail_staged_config_write: bool,
        rename_counts: RefCell<HashMap<RenumberRenameStep, usize>>,
    }

    impl FaultingRenumberFileOps {
        fn failing_rename(step: RenumberRenameStep, occurrence: usize) -> Self {
            Self {
                fail_rename: Some((step, occurrence)),
                ..Self::default()
            }
        }

        fn failing_staged_config_write() -> Self {
            Self {
                fail_staged_config_write: true,
                ..Self::default()
            }
        }
    }

    impl RenumberFileOps for FaultingRenumberFileOps {
        fn create_dir(&self, path: &Path) -> io::Result<()> {
            StdRenumberFileOps.create_dir(path)
        }

        fn write_new_file(
            &self,
            path: &Path,
            contents: &[u8],
            permissions: fs::Permissions,
        ) -> io::Result<()> {
            if self.fail_staged_config_write {
                return Err(io::Error::other("injected staged config write failure"));
            }
            StdRenumberFileOps.write_new_file(path, contents, permissions)
        }

        fn rename(&self, step: RenumberRenameStep, from: &Path, to: &Path) -> io::Result<()> {
            let occurrence = {
                let mut counts = self.rename_counts.borrow_mut();
                let count = counts.entry(step).or_insert(0);
                *count += 1;
                *count
            };
            if self.fail_rename == Some((step, occurrence)) {
                return Err(io::Error::other(format!(
                    "injected {step:?} rename failure"
                )));
            }
            StdRenumberFileOps.rename(step, from, to)
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            StdRenumberFileOps.remove_file(path)
        }

        fn remove_dir(&self, path: &Path) -> io::Result<()> {
            StdRenumberFileOps.remove_dir(path)
        }
    }

    fn transaction_test_dir(name: &str) -> PathBuf {
        let id = NEXT_TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "shosei-chapter-transaction-{name}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("manuscript")).unwrap();
        root
    }

    fn write_transaction_book(root: &Path, chapters: &[&str]) -> Vec<u8> {
        let chapter_lines = chapters
            .iter()
            .map(|chapter| format!("    - {chapter}"))
            .collect::<Vec<_>>()
            .join("\n");
        let book = format!(
            r#"project:
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
{chapter_lines}
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
git:
  lfs: true
"#
        );
        fs::write(root.join("book.yml"), book.as_bytes()).unwrap();
        book.into_bytes()
    }

    fn run_renumber_with_ops<O>(root: &Path, file_ops: &O) -> Result<(), ChapterError>
    where
        O: RenumberFileOps,
    {
        chapter_renumber_with(
            &CommandContext::new(root, None, None),
            ChapterRenumberOptions {
                start_at: 1,
                width: 2,
                dry_run: false,
            },
            file_ops,
            serde_yaml::to_string,
        )
        .map(|_| ())
    }

    fn assert_original_files(root: &Path, expected: &[(&str, &str)], original_config: &[u8]) {
        assert_eq!(fs::read(root.join("book.yml")).unwrap(), original_config);
        for (path, contents) in expected {
            assert_eq!(
                fs::read_to_string(root.join(path)).unwrap(),
                *contents,
                "unexpected contents for {path}"
            );
        }
        assert_no_staging_directories(root);
    }

    fn assert_no_staging_directories(root: &Path) {
        fn visit(path: &Path) {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let file_type = entry.file_type().unwrap();
                if !file_type.is_dir() {
                    continue;
                }
                assert!(
                    !entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".shosei-renumber-"),
                    "staging directory remained at {}",
                    entry.path().display()
                );
                visit(&entry.path());
            }
        }
        visit(root);
    }

    #[test]
    fn staging_directory_reservation_retries_without_touching_collision() {
        let root = transaction_test_dir("staging-collision");
        let first_id = 41;
        let collision = root.join(format!(
            ".shosei-renumber-{}-{first_id}",
            std::process::id()
        ));
        fs::create_dir(&collision).unwrap();
        fs::write(collision.join("sentinel"), "keep me").unwrap();
        let next_id = Cell::new(first_id);

        let reserved = reserve_staging_directory_with(&root, &StdRenumberFileOps, || {
            let id = next_id.get();
            next_id.set(id + 1);
            id
        })
        .unwrap();

        assert_ne!(reserved, collision);
        assert_eq!(
            fs::read_to_string(collision.join("sentinel")).unwrap(),
            "keep me"
        );
        fs::remove_dir(&reserved).unwrap();
        fs::remove_dir_all(&collision).unwrap();
    }

    #[test]
    fn renumber_rolls_back_first_phase_failure() {
        let root = transaction_test_dir("first-phase-failure");
        let original_config = write_transaction_book(
            &root,
            &["manuscript/chapter-a.md", "manuscript/chapter-b.md"],
        );
        fs::write(root.join("manuscript/chapter-a.md"), "chapter a").unwrap();
        fs::write(root.join("manuscript/chapter-b.md"), "chapter b").unwrap();
        let file_ops = FaultingRenumberFileOps::failing_rename(RenumberRenameStep::StageChapter, 2);

        let error = run_renumber_with_ops(&root, &file_ops).unwrap_err();

        assert!(matches!(error, ChapterError::RenameChapterFile { .. }));
        assert_original_files(
            &root,
            &[
                ("manuscript/chapter-a.md", "chapter a"),
                ("manuscript/chapter-b.md", "chapter b"),
            ],
            &original_config,
        );
        assert!(!root.join("manuscript/01-chapter-a.md").exists());
        assert!(!root.join("manuscript/02-chapter-b.md").exists());
    }

    #[test]
    fn renumber_rolls_back_second_phase_failure_through_staging() {
        let root = transaction_test_dir("second-phase-failure");
        let original_config =
            write_transaction_book(&root, &["manuscript/02-same.md", "manuscript/01-same.md"]);
        fs::write(root.join("manuscript/02-same.md"), "first chapter").unwrap();
        fs::write(root.join("manuscript/01-same.md"), "second chapter").unwrap();
        let file_ops =
            FaultingRenumberFileOps::failing_rename(RenumberRenameStep::PublishChapter, 2);

        let error = run_renumber_with_ops(&root, &file_ops).unwrap_err();

        assert!(matches!(error, ChapterError::RenameChapterFile { .. }));
        assert_original_files(
            &root,
            &[
                ("manuscript/02-same.md", "first chapter"),
                ("manuscript/01-same.md", "second chapter"),
            ],
            &original_config,
        );
    }

    #[test]
    fn renumber_rolls_back_config_install_failure_and_rename_cycle() {
        let root = transaction_test_dir("config-install-failure");
        let original_config =
            write_transaction_book(&root, &["manuscript/02-same.md", "manuscript/01-same.md"]);
        fs::write(root.join("manuscript/02-same.md"), "first chapter").unwrap();
        fs::write(root.join("manuscript/01-same.md"), "second chapter").unwrap();
        let file_ops =
            FaultingRenumberFileOps::failing_rename(RenumberRenameStep::InstallConfig, 1);

        let error = run_renumber_with_ops(&root, &file_ops).unwrap_err();

        assert!(matches!(error, ChapterError::RenameChapterConfig { .. }));
        assert_original_files(
            &root,
            &[
                ("manuscript/02-same.md", "first chapter"),
                ("manuscript/01-same.md", "second chapter"),
            ],
            &original_config,
        );
    }

    #[test]
    fn renumber_staged_config_write_failure_leaves_sources_unchanged() {
        let root = transaction_test_dir("config-write-failure");
        let original_config = write_transaction_book(&root, &["manuscript/chapter.md"]);
        fs::write(root.join("manuscript/chapter.md"), "chapter").unwrap();
        let file_ops = FaultingRenumberFileOps::failing_staged_config_write();

        let error = run_renumber_with_ops(&root, &file_ops).unwrap_err();

        assert!(matches!(error, ChapterError::WriteConfig { .. }));
        assert_original_files(
            &root,
            &[("manuscript/chapter.md", "chapter")],
            &original_config,
        );
        assert!(!root.join("manuscript/01-chapter.md").exists());
    }

    #[test]
    fn renumber_serialization_failure_leaves_sources_unchanged() {
        let root = transaction_test_dir("serialization-failure");
        let original_config = write_transaction_book(&root, &["manuscript/chapter.md"]);
        fs::write(root.join("manuscript/chapter.md"), "chapter").unwrap();
        let serialization_error = serde_yaml::from_str::<serde_yaml::Value>("[").unwrap_err();

        let error = chapter_renumber_with(
            &CommandContext::new(&root, None, None),
            ChapterRenumberOptions {
                start_at: 1,
                width: 2,
                dry_run: false,
            },
            &StdRenumberFileOps,
            move |_| Err(serialization_error),
        )
        .unwrap_err();

        assert!(matches!(error, ChapterError::SerializeConfig { .. }));
        assert_original_files(
            &root,
            &[("manuscript/chapter.md", "chapter")],
            &original_config,
        );
        assert!(!root.join("manuscript/01-chapter.md").exists());
    }

    #[test]
    fn placement_index_appends_without_reference() {
        let chapters = vec![RepoPath::parse("manuscript/01.md").unwrap()];
        assert_eq!(placement_index(&chapters, None, None).unwrap(), 1);
    }

    #[test]
    fn placement_index_resolves_after_reference() {
        let chapters = vec![
            RepoPath::parse("manuscript/01.md").unwrap(),
            RepoPath::parse("manuscript/02.md").unwrap(),
        ];
        assert_eq!(
            placement_index(&chapters, None, Some("manuscript/01.md")).unwrap(),
            1
        );
    }

    #[test]
    fn markdown_repo_path_rejects_non_markdown_files() {
        let error = parse_markdown_repo_path("manuscript/01.txt").unwrap_err();
        assert!(matches!(
            error,
            super::ChapterError::ChapterPathMustBeMarkdown { .. }
        ));
    }

    #[test]
    fn renumber_suffix_strips_numeric_prefix() {
        assert_eq!(renumber_suffix("01-chapter-1"), Some("chapter-1"));
        assert_eq!(renumber_suffix("01"), None);
        assert_eq!(renumber_suffix("intro"), Some("intro"));
    }

    #[test]
    fn renumbered_repo_path_preserves_parent_directory() {
        let chapter = RepoPath::parse("books/vol-01/manuscript/10-chapter.md").unwrap();
        let renumbered = renumbered_repo_path(&chapter, 3, 2).unwrap();
        assert_eq!(renumbered.as_str(), "books/vol-01/manuscript/03-chapter.md");
    }
}
