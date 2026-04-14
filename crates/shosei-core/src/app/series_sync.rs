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
    domain::RepoMode,
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
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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
    let catalog = parse_series_catalog(&series.raw, &series.path)?;

    let metadata_dir = context.repo_root.join("shared").join("metadata");
    fs::create_dir_all(&metadata_dir).map_err(|source| SeriesSyncError::CreateDir {
        path: metadata_dir.clone(),
        source,
    })?;
    let catalog_yaml_path = metadata_dir.join("series-catalog.yml");
    let catalog_markdown_path = metadata_dir.join("series-catalog.md");

    write_yaml(&catalog_yaml_path, &catalog)?;
    write_markdown(&catalog_markdown_path, &render_catalog_markdown(&catalog))?;

    let mut updated_books = Vec::new();
    for book in &catalog.books {
        let book_config_path = context.repo_root.join(&book.path).join("book.yml");
        if sync_generated_backmatter(&book_config_path)? {
            updated_books.push(book.id.clone());
        }
    }

    let report_path = context
        .repo_root
        .join("dist")
        .join("reports")
        .join("series-sync.json");
    let report = SeriesSyncReport {
        generated_files: vec![
            relative_to(&context.repo_root, &catalog_yaml_path),
            relative_to(&context.repo_root, &catalog_markdown_path),
        ],
        updated_books: updated_books.clone(),
    };
    write_json(&report_path, &report)?;

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

fn parse_series_catalog(raw: &Value, path: &Path) -> Result<SeriesCatalog, SeriesSyncError> {
    let series = mapping_at(raw, &["series"], path)?;
    let header = SeriesCatalogHeader {
        id: string_at(series, "id", path)?,
        title: string_at(series, "title", path)?,
        language: string_at(series, "language", path)?,
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
            Ok(SeriesCatalogBook {
                id: string_at(mapping, "id", path)?,
                path: string_at(mapping, "path", path)?,
                number: mapping
                    .get(Value::String("number".to_string()))
                    .and_then(Value::as_u64),
                title: string_at(mapping, "title", path)?,
            })
        })
        .collect::<Result<Vec<_>, SeriesSyncError>>()?;

    Ok(SeriesCatalog {
        series: header,
        books,
    })
}

fn sync_generated_backmatter(book_config_path: &Path) -> Result<bool, SeriesSyncError> {
    if !book_config_path.is_file() {
        return Ok(false);
    }

    let contents =
        fs::read_to_string(book_config_path).map_err(|source| SeriesSyncError::Read {
            path: book_config_path.to_path_buf(),
            source,
        })?;
    let mut raw: Value =
        serde_yaml::from_str(&contents).map_err(|source| SeriesSyncError::SerializeYaml {
            path: book_config_path.to_path_buf(),
            source,
        })?;
    let project_type = lookup(&raw, &["project", "type"])
        .and_then(Value::as_str)
        .unwrap_or("novel")
        .to_string();
    let Some(root) = raw.as_mapping_mut() else {
        return Ok(false);
    };

    if project_type == "manga" {
        return Ok(false);
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
        return Ok(false);
    }
    backmatter.push(Value::String(GENERATED_BACKMATTER_PATH.to_string()));

    let serialized =
        serde_yaml::to_string(&raw).map_err(|source| SeriesSyncError::SerializeYaml {
            path: book_config_path.to_path_buf(),
            source,
        })?;
    fs::write(book_config_path, serialized).map_err(|source| SeriesSyncError::Write {
        path: book_config_path.to_path_buf(),
        source,
    })?;
    Ok(true)
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

fn write_yaml(path: &Path, value: &impl Serialize) -> Result<(), SeriesSyncError> {
    let contents =
        serde_yaml::to_string(value).map_err(|source| SeriesSyncError::SerializeYaml {
            path: path.to_path_buf(),
            source,
        })?;
    write_markdown(path, &contents)
}

fn write_markdown(path: &Path, contents: &str) -> Result<(), SeriesSyncError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SeriesSyncError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| SeriesSyncError::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), SeriesSyncError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SeriesSyncError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents =
        serde_json::to_string_pretty(value).map_err(|source| SeriesSyncError::SerializeJson {
            path: path.to_path_buf(),
            source,
        })?;
    fs::write(path, contents).map_err(|source| SeriesSyncError::Write {
        path: path.to_path_buf(),
        source,
    })
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
}
