use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path: None,
        }
    }

    pub fn at(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{} [{}]", self.message, path.display())
        } else {
            f.write_str(&self.message)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub target: String,
    pub location: Option<PathBuf>,
    pub cause: String,
    pub remedy: String,
}

impl ValidationIssue {
    pub fn error(
        target: impl Into<String>,
        cause: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            target: target.into(),
            location: None,
            cause: cause.into(),
            remedy: remedy.into(),
        }
    }

    pub fn warning(
        target: impl Into<String>,
        cause: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            target: target.into(),
            location: None,
            cause: cause.into(),
            remedy: remedy.into(),
        }
    }

    pub fn at(mut self, path: impl Into<PathBuf>) -> Self {
        self.location = Some(path.into());
        self
    }
}
