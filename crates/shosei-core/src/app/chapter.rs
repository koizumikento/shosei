use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
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

    apply_renames(&plans)?;

    let mut book_config = config::load_book_config(&book.config_path)?;
    overwrite_chapters(
        &mut book_config.raw,
        &plans
            .iter()
            .map(|plan| plan.to_repo.clone())
            .collect::<Vec<_>>(),
    );
    rewrite_sections_paths(&mut book_config.raw, &resolved.raw, &rename_map(&plans));
    write_book_config(&book_config)?;

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
    let mut rendered = serde_yaml::to_string(&book_config.raw).map_err(|source| {
        ChapterError::SerializeConfig {
            path: book_config.path.clone(),
            source,
        }
    })?;
    if let Some(stripped) = rendered.strip_prefix("---\n") {
        rendered = stripped.to_string();
    }
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    fs::write(&book_config.path, rendered).map_err(|source| ChapterError::WriteConfig {
        path: book_config.path.clone(),
        source,
    })
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

fn apply_renames(plans: &[RenumberPlan]) -> Result<(), ChapterError> {
    let changing_plans: Vec<&RenumberPlan> = plans
        .iter()
        .filter(|plan| plan.from_repo != plan.to_repo)
        .collect();
    if changing_plans.is_empty() {
        return Ok(());
    }

    let staged_paths: Vec<(PathBuf, &RenumberPlan)> = changing_plans
        .iter()
        .enumerate()
        .map(|(index, plan)| (temporary_rename_path(&plan.from_fs, index), *plan))
        .collect();

    for (temporary_path, plan) in &staged_paths {
        fs::rename(&plan.from_fs, temporary_path).map_err(|source| {
            ChapterError::RenameChapterFile {
                from: plan.from_fs.clone(),
                to: temporary_path.clone(),
                source,
            }
        })?;
    }

    for (temporary_path, plan) in staged_paths {
        fs::rename(&temporary_path, &plan.to_fs).map_err(|source| {
            ChapterError::RenameChapterFile {
                from: temporary_path,
                to: plan.to_fs.clone(),
                source,
            }
        })?;
    }

    Ok(())
}

fn temporary_rename_path(original: &Path, index: usize) -> PathBuf {
    let mut candidate = original.to_path_buf();
    let file_name = original
        .file_name()
        .expect("chapter path should have a file name")
        .to_string_lossy();
    candidate.set_file_name(format!("{file_name}.shosei-renumber-{index}.tmp"));
    candidate
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
    use super::{parse_markdown_repo_path, placement_index, renumber_suffix, renumbered_repo_path};
    use crate::domain::RepoPath;

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
