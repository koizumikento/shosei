use std::{fs, path::Path};

use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::{
    domain::{RepoContext, RepoMode, RepoPath, RepoPathError},
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
    pub effective: Value,
    pub shared: SharedPaths,
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
}

impl ResolvedBookConfig {
    pub fn outputs(&self) -> Vec<String> {
        let mut outputs = Vec::new();
        if output_enabled(&self.effective, "kindle") {
            outputs.push(
                output_target(&self.effective, "kindle")
                    .unwrap_or("kindle")
                    .to_string(),
            );
        }
        if output_enabled(&self.effective, "print") {
            outputs.push(
                output_target(&self.effective, "print")
                    .unwrap_or("print")
                    .to_string(),
            );
        }
        outputs
    }

    pub fn manuscript_files(&self) -> Vec<RepoPath> {
        let mut files = Vec::new();
        for key in ["frontmatter", "chapters", "backmatter"] {
            files.extend(repo_path_list_at(
                &self.effective,
                &["manuscript", key],
                &self.repo.repo_root,
            ));
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

    let (effective, shared) = match context.mode {
        RepoMode::SingleBook => {
            validate_repo_paths(&book_config.raw, &book_config.path)?;
            (book_config.raw.clone(), SharedPaths::default())
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
                merged,
                shared_paths(&series_config.raw, &series_config.path)?,
            )
        }
    };

    Ok(ResolvedBookConfig {
        repo: context.clone(),
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

fn repo_path_list_at(raw: &Value, path: &[&str], config_path: &Path) -> Vec<RepoPath> {
    parse_repo_path_values(raw, path, config_path).unwrap_or_default()
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

fn output_enabled(raw: &Value, output: &str) -> bool {
    lookup(raw, &["outputs", output, "enabled"])
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn output_target<'a>(raw: &'a Value, output: &str) -> Option<&'a str> {
    lookup(raw, &["outputs", output, "target"]).and_then(Value::as_str)
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

        assert_eq!(
            lookup(&resolved.effective, &["book", "language"])
                .and_then(Value::as_str)
                .unwrap(),
            "en"
        );
        assert_eq!(resolved.outputs(), vec!["kindle-ja"]);
        assert_eq!(resolved.shared.assets[0].as_str(), "shared/assets");
        assert!(
            lookup(&resolved.effective, &["validation", "strict"])
                .and_then(Value::as_bool)
                .unwrap()
        );
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
}
