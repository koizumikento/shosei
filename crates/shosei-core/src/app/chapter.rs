use std::{
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
    #[error("chapter file `{path}` does not exist; pass --title to create a new stub")]
    MissingChapterFile { path: PathBuf },
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

    let mut book_config = config::load_book_config(&book.config_path)?;
    overwrite_chapters(&mut book_config.raw, &chapters);
    write_book_config(&book_config)?;

    let chapter_file_path = join_repo_path(&context.repo_root, &target);
    let file_created = ensure_chapter_file(&chapter_file_path, options.title.as_deref())?;

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

#[cfg(test)]
mod tests {
    use super::{parse_markdown_repo_path, placement_index};
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
}
