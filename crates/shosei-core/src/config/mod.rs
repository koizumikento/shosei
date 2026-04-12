use std::{fs, path::Path};

use serde_yaml::Value;
use thiserror::Error;

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
