use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config,
    domain::{RepoMode, RepoPath},
    fs::join_repo_path,
    repo::{self, RepoError},
};

const GENERATED_BACKMATTER_PATH: &str = "shared/metadata/series-catalog.md";

#[derive(Debug, Clone)]
pub struct SeriesSyncResult {
    pub summary: String,
    pub catalog_yaml_path: PathBuf,
    pub catalog_markdown_path: PathBuf,
    pub report_path: PathBuf,
    pub updated_books: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SeriesSyncError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("series sync requires a series repository, but discovered {mode} at {path}")]
    NotSeriesRepo { mode: &'static str, path: PathBuf },
    #[error("missing required field `{field}` in {path}")]
    MissingField { path: PathBuf, field: String },
    #[error("field `{field}` in {path} must be {expected}")]
    InvalidFieldType {
        path: PathBuf,
        field: String,
        expected: &'static str,
    },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("series book `{book_id}` points to {path}, but book.yml was not found")]
    MissingBookConfig { book_id: String, path: PathBuf },
    #[error("failed to resolve {path}: {source}")]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to inspect series sync output target {path}: {source}")]
    InspectOutputTarget {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("series sync output target {path} must be {expected}")]
    InvalidOutputTarget {
        path: PathBuf,
        expected: &'static str,
    },
    #[error(
        "series sync output target {path} resolves outside repository {repo_root}: {resolved_path}"
    )]
    OutputOutsideRepository {
        path: PathBuf,
        resolved_path: PathBuf,
        repo_root: PathBuf,
    },
    #[error("series book `{book_id}` resolves outside the repository: {path}")]
    BookOutsideRepository { book_id: String, path: PathBuf },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize YAML for {path}: {source}")]
    SerializeYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to serialize JSON for {path}: {source}")]
    SerializeJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Serialize)]
struct SeriesCatalog {
    series: SeriesCatalogHeader,
    books: Vec<SeriesCatalogBook>,
}

#[derive(Debug, Clone, Serialize)]
struct SeriesCatalogHeader {
    id: String,
    title: String,
    language: String,
    project_type: String,
}

#[derive(Debug, Clone, Serialize)]
struct SeriesCatalogBook {
    id: String,
    path: String,
    number: Option<u64>,
    title: String,
}

#[derive(Debug, Clone)]
struct SeriesCatalogSource {
    series: SeriesCatalogHeader,
    books: Vec<SeriesCatalogBookSource>,
}

#[derive(Debug, Clone)]
struct SeriesCatalogBookSource {
    id: String,
    path: RepoPath,
    number: Option<u64>,
    title: Option<String>,
}

#[derive(Debug)]
struct BookUpdatePlan {
    id: String,
    config_path: PathBuf,
    contents: Option<String>,
}

#[derive(Debug)]
struct PreparedWrite {
    path: PathBuf,
    target_path: PathBuf,
    contents: String,
}

#[derive(Debug)]
struct SeriesSyncPlan {
    catalog_yaml_path: PathBuf,
    catalog_markdown_path: PathBuf,
    report_path: PathBuf,
    writes: Vec<PreparedWrite>,
    updated_books: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SeriesSyncReport {
    generated_files: Vec<String>,
    updated_books: Vec<String>,
}

pub fn series_sync(command: &CommandContext) -> Result<SeriesSyncResult, SeriesSyncError> {
    let context = repo::discover(&command.start_path, None)?;
    if context.mode != RepoMode::Series {
        return Err(SeriesSyncError::NotSeriesRepo {
            mode: match context.mode {
                RepoMode::SingleBook => "single-book",
                RepoMode::Series => "series",
            },
            path: context.repo_root,
        });
    }

    let series_path = context.repo_root.join("series.yml");
    let series = config::load_series_config(&series_path)?;
    let source = parse_series_catalog(&series.raw, &series.path)?;
    let (canonical_repo_root, catalog, book_updates) =
        preflight_series_sync(&context.repo_root, source)?;
    let plan = plan_series_sync(
        &context.repo_root,
        &canonical_repo_root,
        &catalog,
        book_updates,
    )?;
    apply_series_sync_plan(&plan)?;

    let SeriesSyncPlan {
        catalog_yaml_path,
        catalog_markdown_path,
        report_path,
        updated_books,
        ..
    } = plan;

    Ok(SeriesSyncResult {
        summary: format!(
            "series sync completed at {}: generated {}, {}; updated prose backmatter in {} book(s); report: {}",
            context.repo_root.display(),
            catalog_yaml_path.display(),
            catalog_markdown_path.display(),
            updated_books.len(),
            report_path.display()
        ),
        catalog_yaml_path,
        catalog_markdown_path,
        report_path,
        updated_books,
    })
}

fn plan_series_sync(
    repo_root: &Path,
    canonical_repo_root: &Path,
    catalog: &SeriesCatalog,
    book_updates: Vec<BookUpdatePlan>,
) -> Result<SeriesSyncPlan, SeriesSyncError> {
    let metadata_dir = repo_root.join("shared").join("metadata");
    let catalog_yaml_path = metadata_dir.join("series-catalog.yml");
    let catalog_markdown_path = metadata_dir.join("series-catalog.md");
    let report_path = repo_root
        .join("dist")
        .join("reports")
        .join("series-sync.json");
    let canonical_metadata_dir = canonical_repo_root.join("shared").join("metadata");
    let catalog_yaml_target_path = canonical_metadata_dir.join("series-catalog.yml");
    let catalog_markdown_target_path = canonical_metadata_dir.join("series-catalog.md");
    let report_target_path = canonical_repo_root
        .join("dist")
        .join("reports")
        .join("series-sync.json");

    let mut writes = vec![
        PreparedWrite {
            path: catalog_yaml_path.clone(),
            target_path: catalog_yaml_target_path,
            contents: serialize_yaml(&catalog_yaml_path, catalog)?,
        },
        PreparedWrite {
            path: catalog_markdown_path.clone(),
            target_path: catalog_markdown_target_path,
            contents: render_catalog_markdown(catalog),
        },
    ];
    let mut updated_books = Vec::new();
    for update in book_updates {
        if let Some(contents) = update.contents {
            updated_books.push(update.id);
            writes.push(PreparedWrite {
                target_path: update.config_path.clone(),
                path: update.config_path,
                contents,
            });
        }
    }

    let report = SeriesSyncReport {
        generated_files: vec![
            relative_to(repo_root, &catalog_yaml_path),
            relative_to(repo_root, &catalog_markdown_path),
        ],
        updated_books: updated_books.clone(),
    };
    writes.push(PreparedWrite {
        path: report_path.clone(),
        target_path: report_target_path,
        contents: serialize_json(&report_path, &report)?,
    });

    preflight_write_targets(&writes, canonical_repo_root)?;

    Ok(SeriesSyncPlan {
        catalog_yaml_path,
        catalog_markdown_path,
        report_path,
        writes,
        updated_books,
    })
}

fn apply_series_sync_plan(plan: &SeriesSyncPlan) -> Result<(), SeriesSyncError> {
    for write in &plan.writes {
        if let Some(target_parent) = write.target_path.parent() {
            fs::create_dir_all(target_parent).map_err(|source| SeriesSyncError::CreateDir {
                path: write.path.parent().unwrap_or(target_parent).to_path_buf(),
                source,
            })?;
        }
    }
    for write in &plan.writes {
        fs::write(&write.target_path, &write.contents).map_err(|source| {
            SeriesSyncError::Write {
                path: write.path.clone(),
                source,
            }
        })?;
    }
    Ok(())
}

fn parse_series_catalog(raw: &Value, path: &Path) -> Result<SeriesCatalogSource, SeriesSyncError> {
    let series = mapping_at(raw, &["series"], path)?;
    let header = SeriesCatalogHeader {
        id: string_at(series, "id", path)?,
        title: string_at(series, "title", path)?,
        language: optional_string_at(series, "language", path)?.unwrap_or_else(|| "ja".to_string()),
        project_type: string_at(series, "type", path)?,
    };
    let books_value = lookup(raw, &["books"]).ok_or_else(|| SeriesSyncError::MissingField {
        path: path.to_path_buf(),
        field: "books".to_string(),
    })?;
    let books = books_value
        .as_sequence()
        .ok_or_else(|| SeriesSyncError::InvalidFieldType {
            path: path.to_path_buf(),
            field: "books".to_string(),
            expected: "a sequence",
        })?
        .iter()
        .map(|entry| {
            let mapping = entry
                .as_mapping()
                .ok_or_else(|| SeriesSyncError::InvalidFieldType {
                    path: path.to_path_buf(),
                    field: "books[]".to_string(),
                    expected: "a mapping",
                })?;
            let path_value = string_at(mapping, "path", path)?;
            let repo_path = RepoPath::parse(path_value.clone()).map_err(|source| {
                config::ConfigError::InvalidRepoPath {
                    path: path.display().to_string(),
                    value: path_value,
                    source,
                }
            })?;
            Ok(SeriesCatalogBookSource {
                id: string_at(mapping, "id", path)?,
                path: repo_path,
                number: optional_u64_at(mapping, "number", path)?,
                title: optional_string_at(mapping, "title", path)?,
            })
        })
        .collect::<Result<Vec<_>, SeriesSyncError>>()?;

    Ok(SeriesCatalogSource {
        series: header,
        books,
    })
}

fn preflight_series_sync(
    repo_root: &Path,
    source: SeriesCatalogSource,
) -> Result<(PathBuf, SeriesCatalog, Vec<BookUpdatePlan>), SeriesSyncError> {
    let canonical_repo_root =
        fs::canonicalize(repo_root).map_err(|source| SeriesSyncError::Canonicalize {
            path: repo_root.to_path_buf(),
            source,
        })?;
    let mut books = Vec::with_capacity(source.books.len());
    let mut updates = Vec::with_capacity(source.books.len());

    for book in source.books {
        let book_root = join_repo_path(repo_root, &book.path);
        let book_config_path = book_root.join("book.yml");
        if !book_config_path.is_file() {
            return Err(SeriesSyncError::MissingBookConfig {
                book_id: book.id,
                path: book_config_path,
            });
        }
        let canonical_book_config = fs::canonicalize(&book_config_path).map_err(|source| {
            SeriesSyncError::Canonicalize {
                path: book_config_path.clone(),
                source,
            }
        })?;
        if !canonical_book_config.starts_with(&canonical_repo_root) {
            return Err(SeriesSyncError::BookOutsideRepository {
                book_id: book.id,
                path: canonical_book_config,
            });
        }

        let book_config = config::load_book_config(&canonical_book_config)?;
        let title = match book.title {
            Some(title) => title,
            None => lookup(&book_config.raw, &["book", "title"])
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| SeriesSyncError::MissingField {
                    path: book_config_path.clone(),
                    field: "book.title".to_string(),
                })?,
        };
        let contents = plan_generated_backmatter(book_config.raw, &canonical_book_config)?;
        let path = book.path.to_string();
        books.push(SeriesCatalogBook {
            id: book.id.clone(),
            path,
            number: book.number,
            title,
        });
        updates.push(BookUpdatePlan {
            id: book.id,
            config_path: canonical_book_config,
            contents,
        });
    }

    Ok((
        canonical_repo_root,
        SeriesCatalog {
            series: source.series,
            books,
        },
        updates,
    ))
}

fn plan_generated_backmatter(
    mut raw: Value,
    book_config_path: &Path,
) -> Result<Option<String>, SeriesSyncError> {
    let project_type = lookup(&raw, &["project", "type"])
        .and_then(Value::as_str)
        .unwrap_or("novel")
        .to_string();
    let Some(root) = raw.as_mapping_mut() else {
        return Ok(None);
    };

    if project_type == "manga" {
        return Ok(None);
    }

    let manuscript = ensure_optional_mapping(root, "manuscript", book_config_path)?;
    let backmatter = ensure_optional_sequence(
        manuscript,
        "backmatter",
        "manuscript.backmatter",
        book_config_path,
    )?;
    let already_present = backmatter
        .iter()
        .any(|entry| entry.as_str() == Some(GENERATED_BACKMATTER_PATH));
    if already_present {
        return Ok(None);
    }
    backmatter.push(Value::String(GENERATED_BACKMATTER_PATH.to_string()));

    let serialized =
        serde_yaml::to_string(&raw).map_err(|source| SeriesSyncError::SerializeYaml {
            path: book_config_path.to_path_buf(),
            source,
        })?;
    Ok(Some(serialized))
}

fn render_catalog_markdown(catalog: &SeriesCatalog) -> String {
    let mut lines = vec![
        format!("# {}", catalog.series.title),
        String::new(),
        "## 既刊一覧".to_string(),
        String::new(),
    ];
    for book in &catalog.books {
        let prefix = book
            .number
            .map(|number| format!("{number}. "))
            .unwrap_or_default();
        lines.push(format!("- {}{} ({})", prefix, book.title, book.id));
    }
    lines.push(String::new());
    lines.push("> generated by `shosei series sync`".to_string());
    lines.push(String::new());
    lines.join("\n")
}

fn serialize_yaml(path: &Path, value: &impl Serialize) -> Result<String, SeriesSyncError> {
    serde_yaml::to_string(value).map_err(|source| SeriesSyncError::SerializeYaml {
        path: path.to_path_buf(),
        source,
    })
}

fn serialize_json(path: &Path, value: &impl Serialize) -> Result<String, SeriesSyncError> {
    serde_json::to_string_pretty(value).map_err(|source| SeriesSyncError::SerializeJson {
        path: path.to_path_buf(),
        source,
    })
}

fn preflight_write_targets(
    writes: &[PreparedWrite],
    canonical_repo_root: &Path,
) -> Result<(), SeriesSyncError> {
    for write in writes {
        preflight_write_target(&write.target_path, &write.path, canonical_repo_root)?;
    }
    Ok(())
}

fn preflight_write_target(
    target_path: &Path,
    reported_path: &Path,
    canonical_repo_root: &Path,
) -> Result<(), SeriesSyncError> {
    let mut ancestor = target_path.parent();
    while let Some(parent) = ancestor {
        match fs::symlink_metadata(parent) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                let reported_parent = reported_ancestor(target_path, reported_path, parent);
                let resolved_parent = fs::canonicalize(parent).map_err(|source| {
                    SeriesSyncError::InspectOutputTarget {
                        path: reported_parent.clone(),
                        source,
                    }
                })?;
                if !resolved_parent.starts_with(canonical_repo_root) {
                    return Err(SeriesSyncError::OutputOutsideRepository {
                        path: reported_parent,
                        resolved_path: resolved_parent,
                        repo_root: canonical_repo_root.to_path_buf(),
                    });
                }
                break;
            }
            Ok(_) => {
                return Err(SeriesSyncError::InvalidOutputTarget {
                    path: reported_ancestor(target_path, reported_path, parent),
                    expected: "a directory",
                });
            }
            Err(source)
                if matches!(
                    source.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                ancestor = parent.parent();
            }
            Err(source) => {
                return Err(SeriesSyncError::InspectOutputTarget {
                    path: reported_ancestor(target_path, reported_path, parent),
                    source,
                });
            }
        }
    }

    match fs::symlink_metadata(target_path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(SeriesSyncError::InvalidOutputTarget {
            path: reported_path.to_path_buf(),
            expected: "a regular file or absent",
        }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SeriesSyncError::InspectOutputTarget {
            path: reported_path.to_path_buf(),
            source,
        }),
    }
}

fn reported_ancestor(target_path: &Path, reported_path: &Path, ancestor: &Path) -> PathBuf {
    let distance = target_path
        .strip_prefix(ancestor)
        .map(|suffix| suffix.components().count())
        .unwrap_or(0);
    reported_path
        .ancestors()
        .nth(distance)
        .unwrap_or(reported_path)
        .to_path_buf()
}

fn lookup<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current
            .as_mapping()?
            .get(Value::String((*segment).to_string()))?;
    }
    Some(current)
}

fn mapping_at<'a>(
    value: &'a Value,
    path: &[&str],
    file_path: &Path,
) -> Result<&'a Mapping, SeriesSyncError> {
    lookup(value, path)
        .and_then(Value::as_mapping)
        .ok_or_else(|| SeriesSyncError::InvalidFieldType {
            path: file_path.to_path_buf(),
            field: path.join("."),
            expected: "a mapping",
        })
}

fn string_at(mapping: &Mapping, key: &str, file_path: &Path) -> Result<String, SeriesSyncError> {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SeriesSyncError::MissingField {
            path: file_path.to_path_buf(),
            field: key.to_string(),
        })
}

fn optional_string_at(
    mapping: &Mapping,
    key: &str,
    file_path: &Path,
) -> Result<Option<String>, SeriesSyncError> {
    let Some(value) = mapping.get(Value::String(key.to_string())) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(str::to_string)
        .map(Some)
        .ok_or_else(|| SeriesSyncError::InvalidFieldType {
            path: file_path.to_path_buf(),
            field: key.to_string(),
            expected: "a string",
        })
}

fn optional_u64_at(
    mapping: &Mapping,
    key: &str,
    file_path: &Path,
) -> Result<Option<u64>, SeriesSyncError> {
    let Some(value) = mapping.get(Value::String(key.to_string())) else {
        return Ok(None);
    };
    value
        .as_u64()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or_else(|| SeriesSyncError::InvalidFieldType {
            path: file_path.to_path_buf(),
            field: key.to_string(),
            expected: "a positive integer",
        })
}

fn ensure_optional_mapping<'a>(
    mapping: &'a mut Mapping,
    key: &str,
    file_path: &Path,
) -> Result<&'a mut Mapping, SeriesSyncError> {
    let value = mapping
        .entry(Value::String(key.to_string()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    if !matches!(value, Value::Mapping(_)) {
        return Err(SeriesSyncError::InvalidFieldType {
            path: file_path.to_path_buf(),
            field: key.to_string(),
            expected: "a mapping",
        });
    }
    Ok(value.as_mapping_mut().expect("mapping inserted above"))
}

fn ensure_optional_sequence<'a>(
    mapping: &'a mut Mapping,
    key: &str,
    field: &str,
    file_path: &Path,
) -> Result<&'a mut Vec<Value>, SeriesSyncError> {
    let value = mapping
        .entry(Value::String(key.to_string()))
        .or_insert_with(|| Value::Sequence(Vec::new()));
    if !matches!(value, Value::Sequence(_)) {
        return Err(SeriesSyncError::InvalidFieldType {
            path: file_path.to_path_buf(),
            field: field.to_string(),
            expected: "a sequence",
        });
    }
    Ok(value.as_sequence_mut().expect("sequence inserted above"))
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-series-sync-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_single_book_series(root: &Path) -> String {
        fs::create_dir_all(root.join("books/vol-01")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: vol-01
    path: books/vol-01
    title: "Volume 1"
"#,
        )
        .unwrap();
        let original = r#"
project:
  type: novel
book:
  title: "Volume 1"
"#;
        fs::write(root.join("books/vol-01/book.yml"), original).unwrap();
        original.to_string()
    }

    #[test]
    fn series_sync_generates_catalog_and_updates_backmatter() {
        let root = temp_dir("catalog");
        fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
        fs::create_dir_all(root.join("books/vol-02/manuscript")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  language: ja
  type: novel
shared:
  metadata:
    - shared/metadata
books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "Volume 1"
  - id: vol-02
    path: books/vol-02
    number: 2
    title: "Volume 2"
"#,
        )
        .unwrap();
        for book_id in ["vol-01", "vol-02"] {
            fs::write(
                root.join(format!("books/{book_id}/book.yml")),
                format!(
                    r#"
project:
  type: novel
  vcs: git
book:
  title: "{book_id}"
  authors:
    - "Author"
manuscript:
  chapters:
    - books/{book_id}/manuscript/01.md
"#
                ),
            )
            .unwrap();
            fs::write(
                root.join(format!("books/{book_id}/manuscript/01.md")),
                "# Chapter 1\n",
            )
            .unwrap();
        }

        let result = series_sync(&CommandContext::new(&root, None, None)).unwrap();

        assert_eq!(result.updated_books.len(), 2);
        assert!(result.catalog_yaml_path.is_file());
        assert!(result.catalog_markdown_path.is_file());
        let book_contents = fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap();
        assert!(book_contents.contains("shared/metadata/series-catalog.md"));
    }

    #[test]
    fn series_sync_is_idempotent_for_generated_backmatter() {
        let root = temp_dir("idempotent");
        fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  language: ja
  type: novel
books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "Volume 1"
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
  title: "Volume 1"
  authors:
    - "Author"
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
  backmatter:
    - shared/metadata/series-catalog.md
"#,
        )
        .unwrap();
        fs::write(root.join("books/vol-01/manuscript/01.md"), "# Chapter 1\n").unwrap();

        let result = series_sync(&CommandContext::new(&root, None, None)).unwrap();

        assert!(result.updated_books.is_empty());
    }

    #[test]
    fn series_sync_rejects_non_sequence_backmatter_without_rewriting_book() {
        let root = temp_dir("invalid-backmatter");
        fs::create_dir_all(root.join("books/vol-01/manuscript")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  language: ja
  type: novel
books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "Volume 1"
"#,
        )
        .unwrap();
        let original = r#"
project:
  type: novel
  vcs: git
book:
  title: "Volume 1"
  authors:
    - "Author"
manuscript:
  chapters:
    - books/vol-01/manuscript/01.md
  backmatter: shared/metadata/existing.md
"#;
        fs::write(root.join("books/vol-01/book.yml"), original).unwrap();
        fs::write(root.join("books/vol-01/manuscript/01.md"), "# Chapter 1\n").unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::InvalidFieldType { ref field, .. } if field == "manuscript.backmatter"
        ));
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original
        );
    }

    #[test]
    fn series_sync_defaults_language_and_uses_book_title() {
        let root = temp_dir("schema-defaults");
        fs::create_dir_all(root.join("books/vol-01")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: vol-01
    path: books/vol-01
    number: 1
"#,
        )
        .unwrap();
        fs::write(
            root.join("books/vol-01/book.yml"),
            r#"
project:
  type: novel
book:
  title: "Title from book.yml"
"#,
        )
        .unwrap();

        let result = series_sync(&CommandContext::new(&root, None, None)).unwrap();
        let catalog: Value =
            serde_yaml::from_str(&fs::read_to_string(result.catalog_yaml_path).unwrap()).unwrap();

        assert_eq!(
            lookup(&catalog, &["series", "language"]).and_then(Value::as_str),
            Some("ja")
        );
        assert_eq!(
            lookup(&catalog, &["books"])
                .and_then(Value::as_sequence)
                .and_then(|books| books.first())
                .and_then(|book| lookup(book, &["title"]))
                .and_then(Value::as_str),
            Some("Title from book.yml")
        );
    }

    #[test]
    fn series_sync_preflights_every_book_before_writing() {
        let root = temp_dir("preflight-all-books");
        fs::create_dir_all(root.join("books/vol-01")).unwrap();
        fs::create_dir_all(root.join("shared/metadata")).unwrap();
        fs::create_dir_all(root.join("dist/reports")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: vol-01
    path: books/vol-01
  - id: vol-02
    path: books/vol-02
"#,
        )
        .unwrap();
        let original = r#"
project:
  type: novel
book:
  title: "Volume 1"
"#;
        fs::write(root.join("books/vol-01/book.yml"), original).unwrap();
        fs::write(
            root.join("shared/metadata/series-catalog.yml"),
            "old yaml\n",
        )
        .unwrap();
        fs::write(
            root.join("shared/metadata/series-catalog.md"),
            "old markdown\n",
        )
        .unwrap();
        fs::write(root.join("dist/reports/series-sync.json"), "old report\n").unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::MissingBookConfig { ref book_id, .. } if book_id == "vol-02"
        ));
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original
        );
        assert_eq!(
            fs::read_to_string(root.join("shared/metadata/series-catalog.yml")).unwrap(),
            "old yaml\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("shared/metadata/series-catalog.md")).unwrap(),
            "old markdown\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("dist/reports/series-sync.json")).unwrap(),
            "old report\n"
        );
    }

    #[test]
    fn series_sync_preflights_catalog_targets_before_writing() {
        let root = temp_dir("preflight-catalog-targets");
        let original_book = write_single_book_series(&root);
        let metadata_dir = root.join("shared/metadata");
        let report_path = root.join("dist/reports/series-sync.json");
        let catalog_yaml_path = metadata_dir.join("series-catalog.yml");
        let catalog_markdown_path = metadata_dir.join("series-catalog.md");
        fs::create_dir_all(&metadata_dir).unwrap();
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&catalog_yaml_path, "old yaml\n").unwrap();
        fs::create_dir(&catalog_markdown_path).unwrap();
        fs::write(&report_path, "old report\n").unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::InvalidOutputTarget { ref path, .. }
                if path == &catalog_markdown_path
        ));
        assert_eq!(
            fs::read_to_string(&catalog_yaml_path).unwrap(),
            "old yaml\n"
        );
        assert!(catalog_markdown_path.is_dir());
        assert_eq!(fs::read_to_string(&report_path).unwrap(), "old report\n");
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original_book
        );
    }

    #[test]
    fn series_sync_preflights_report_target_before_writing() {
        let root = temp_dir("preflight-report-target");
        let original_book = write_single_book_series(&root);
        let metadata_dir = root.join("shared/metadata");
        let report_path = root.join("dist/reports/series-sync.json");
        let catalog_yaml_path = metadata_dir.join("series-catalog.yml");
        let catalog_markdown_path = metadata_dir.join("series-catalog.md");
        fs::create_dir_all(&metadata_dir).unwrap();
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&catalog_yaml_path, "old yaml\n").unwrap();
        fs::write(&catalog_markdown_path, "old markdown\n").unwrap();
        fs::create_dir(&report_path).unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::InvalidOutputTarget { ref path, .. } if path == &report_path
        ));
        assert_eq!(
            fs::read_to_string(&catalog_yaml_path).unwrap(),
            "old yaml\n"
        );
        assert_eq!(
            fs::read_to_string(&catalog_markdown_path).unwrap(),
            "old markdown\n"
        );
        assert!(report_path.is_dir());
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original_book
        );
    }

    #[test]
    fn series_sync_preflights_output_parent_types_before_writing() {
        let root = temp_dir("preflight-output-parent");
        let original_book = write_single_book_series(&root);
        let metadata_dir = root.join("shared/metadata");
        let catalog_yaml_path = metadata_dir.join("series-catalog.yml");
        let catalog_markdown_path = metadata_dir.join("series-catalog.md");
        let invalid_parent = root.join("dist");
        fs::create_dir_all(&metadata_dir).unwrap();
        fs::write(&catalog_yaml_path, "old yaml\n").unwrap();
        fs::write(&catalog_markdown_path, "old markdown\n").unwrap();
        fs::write(&invalid_parent, "not a directory\n").unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::InvalidOutputTarget { ref path, .. } if path == &invalid_parent
        ));
        assert_eq!(
            fs::read_to_string(&catalog_yaml_path).unwrap(),
            "old yaml\n"
        );
        assert_eq!(
            fs::read_to_string(&catalog_markdown_path).unwrap(),
            "old markdown\n"
        );
        assert_eq!(
            fs::read_to_string(&invalid_parent).unwrap(),
            "not a directory\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original_book
        );
    }

    #[test]
    fn series_sync_rejects_parent_traversal_without_touching_external_book() {
        let root = temp_dir("reject-traversal");
        let external = root.parent().unwrap().join(format!(
            "shosei-series-sync-external-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&external);
        fs::create_dir_all(&external).unwrap();
        let original = "book:\n  title: External\n";
        fs::write(external.join("book.yml"), original).unwrap();
        fs::write(
            root.join("series.yml"),
            format!(
                r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: external
    path: ../{}
"#,
                external.file_name().unwrap().to_string_lossy()
            ),
        )
        .unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::Config(config::ConfigError::InvalidRepoPath { .. })
        ));
        assert_eq!(
            fs::read_to_string(external.join("book.yml")).unwrap(),
            original
        );
        assert!(!root.join("shared/metadata").exists());
        let _ = fs::remove_dir_all(external);
    }

    #[test]
    fn series_sync_rejects_absolute_and_windows_drive_paths_before_writing() {
        for (name, path) in [("absolute", "/tmp/outside"), ("drive", "C:/outside")] {
            let root = temp_dir(name);
            fs::write(
                root.join("series.yml"),
                format!(
                    r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: external
    path: "{path}"
"#
                ),
            )
            .unwrap();

            let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

            assert!(matches!(
                error,
                SeriesSyncError::Config(config::ConfigError::InvalidRepoPath { .. })
            ));
            assert!(!root.join("shared/metadata").exists());
            assert!(!root.join("dist/reports/series-sync.json").exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn series_sync_via_repo_root_symlink_writes_missing_outputs_inside_repository() {
        use std::os::unix::fs::symlink;

        let workspace = temp_dir("repo-root-symlink");
        let root = workspace.join("repo");
        fs::create_dir(&root).unwrap();
        write_single_book_series(&root);
        let linked_root = workspace.join("repo-link");
        symlink(&root, &linked_root).unwrap();

        assert!(!root.join("shared").exists());
        assert!(!root.join("dist").exists());

        let result = series_sync(&CommandContext::new(&linked_root, None, None)).unwrap();

        assert_eq!(
            result.catalog_yaml_path,
            linked_root.join("shared/metadata/series-catalog.yml")
        );
        assert_eq!(
            result.catalog_markdown_path,
            linked_root.join("shared/metadata/series-catalog.md")
        );
        assert_eq!(
            result.report_path,
            linked_root.join("dist/reports/series-sync.json")
        );
        assert!(result.catalog_yaml_path.is_file());
        assert!(result.catalog_markdown_path.is_file());
        assert!(result.report_path.is_file());
        assert!(
            fs::read_to_string(root.join("books/vol-01/book.yml"))
                .unwrap()
                .contains(GENERATED_BACKMATTER_PATH)
        );
    }

    #[cfg(unix)]
    #[test]
    fn series_sync_rejects_generated_output_parent_symlink_outside_repository() {
        use std::os::unix::fs::symlink;

        let workspace = temp_dir("external-output-parent-symlink");
        let root = workspace.join("repo");
        let external = workspace.join("external");
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(external.join("metadata")).unwrap();
        let original_book = write_single_book_series(&root);
        let external_yaml = external.join("metadata/series-catalog.yml");
        let external_markdown = external.join("metadata/series-catalog.md");
        let report_path = root.join("dist/reports/series-sync.json");
        fs::write(&external_yaml, "external yaml\n").unwrap();
        fs::write(&external_markdown, "external markdown\n").unwrap();
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&report_path, "old report\n").unwrap();
        symlink(&external, root.join("shared")).unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::OutputOutsideRepository { .. }
        ));
        assert_eq!(
            fs::read_to_string(&external_yaml).unwrap(),
            "external yaml\n"
        );
        assert_eq!(
            fs::read_to_string(&external_markdown).unwrap(),
            "external markdown\n"
        );
        assert_eq!(fs::read_to_string(&report_path).unwrap(), "old report\n");
        assert_eq!(
            fs::read_to_string(root.join("books/vol-01/book.yml")).unwrap(),
            original_book
        );
    }

    #[cfg(unix)]
    #[test]
    fn series_sync_rejects_book_config_reached_through_external_symlink() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("reject-external-symlink");
        let external = root.parent().unwrap().join(format!(
            "shosei-series-sync-symlink-external-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&external);
        fs::create_dir_all(root.join("books")).unwrap();
        fs::create_dir_all(&external).unwrap();
        let original = "book:\n  title: External\n";
        fs::write(external.join("book.yml"), original).unwrap();
        symlink(&external, root.join("books/external")).unwrap();
        fs::write(
            root.join("series.yml"),
            r#"
series:
  id: demo
  title: "Demo Series"
  type: novel
books:
  - id: external
    path: books/external
"#,
        )
        .unwrap();

        let error = series_sync(&CommandContext::new(&root, None, None)).unwrap_err();

        assert!(matches!(
            error,
            SeriesSyncError::BookOutsideRepository { ref book_id, .. } if book_id == "external"
        ));
        assert_eq!(
            fs::read_to_string(external.join("book.yml")).unwrap(),
            original
        );
        assert!(!root.join("shared/metadata").exists());
        let _ = fs::remove_dir_all(external);
    }
}
