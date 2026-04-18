use serde_yaml::{Mapping, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrontmatterError {
    #[error("frontmatter is missing a closing delimiter")]
    MissingClosingDelimiter,
    #[error("frontmatter root must be a YAML mapping")]
    RootMustBeMapping,
    #[error("failed to parse frontmatter YAML: {source}")]
    Parse {
        #[source]
        source: serde_yaml::Error,
    },
}

pub fn parse_frontmatter(contents: &str) -> Result<Option<Mapping>, FrontmatterError> {
    let Some(body_start) = opening_delimiter_end(contents) else {
        return Ok(None);
    };

    let remaining = &contents[body_start..];
    let mut yaml = String::new();
    let mut closed = false;

    for segment in remaining.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if line == "---" || line == "..." {
            closed = true;
            break;
        }
        yaml.push_str(segment);
    }

    if !closed {
        return Err(FrontmatterError::MissingClosingDelimiter);
    }

    let value = serde_yaml::from_str::<Value>(&yaml)
        .map_err(|source| FrontmatterError::Parse { source })?;
    match value {
        Value::Mapping(mapping) => Ok(Some(mapping)),
        _ => Err(FrontmatterError::RootMustBeMapping),
    }
}

pub fn body_without_frontmatter(contents: &str) -> Result<&str, FrontmatterError> {
    let Some(body_start) = opening_delimiter_end(contents) else {
        return Ok(contents);
    };

    let remaining = &contents[body_start..];
    let mut consumed = 0usize;

    for segment in remaining.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        consumed += segment.len();
        if line == "---" || line == "..." {
            return Ok(&remaining[consumed..]);
        }
    }

    Err(FrontmatterError::MissingClosingDelimiter)
}

fn opening_delimiter_end(contents: &str) -> Option<usize> {
    if let Some(rest) = contents.strip_prefix("---\r\n") {
        Some(contents.len() - rest.len())
    } else {
        contents
            .strip_prefix("---\n")
            .map(|rest| contents.len() - rest.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_frontmatter_is_absent() {
        assert!(parse_frontmatter("# Chapter\n").unwrap().is_none());
    }

    #[test]
    fn parses_yaml_mapping_frontmatter() {
        let frontmatter =
            parse_frontmatter("---\ncharacters:\n  - hero\nstatus: draft\n---\n# Chapter\n")
                .unwrap()
                .unwrap();

        assert_eq!(
            frontmatter.get(Value::String("status".to_string())),
            Some(&Value::String("draft".to_string()))
        );
    }

    #[test]
    fn rejects_non_mapping_frontmatter() {
        let error = parse_frontmatter("---\n- hero\n---\n").unwrap_err();
        assert!(matches!(error, FrontmatterError::RootMustBeMapping));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let error = parse_frontmatter("---\nid: hero\n").unwrap_err();
        assert!(matches!(error, FrontmatterError::MissingClosingDelimiter));
    }

    #[test]
    fn returns_body_without_frontmatter() {
        let body = body_without_frontmatter("---\nid: hero\n---\n# Chapter\n本文\n").unwrap();
        assert_eq!(body, "# Chapter\n本文\n");
    }
}
