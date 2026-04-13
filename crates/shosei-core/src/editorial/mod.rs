use std::{fs, path::PathBuf};

use serde::Deserialize;
use thiserror::Error;
use time::{Date, OffsetDateTime, format_description::FormatItem, macros::format_description};

use crate::{config::ResolvedBookConfig, domain::RepoPath, fs::join_repo_path};

#[derive(Debug, Clone, Default)]
pub struct EditorialBundle {
    pub style: Option<LoadedStyleGuide>,
    pub claims: Option<LoadedClaimLedger>,
    pub figures: Option<LoadedFigureLedger>,
    pub freshness: Option<LoadedFreshnessLedger>,
}

impl EditorialBundle {
    pub fn is_empty(&self) -> bool {
        self.style.is_none()
            && self.claims.is_none()
            && self.figures.is_none()
            && self.freshness.is_none()
    }

    pub fn style_rule_count(&self) -> usize {
        self.style
            .as_ref()
            .map(|loaded| loaded.data.preferred_terms.len() + loaded.data.banned_terms.len())
            .unwrap_or(0)
    }

    pub fn claim_count(&self) -> usize {
        self.claims
            .as_ref()
            .map(|loaded| loaded.data.claims.len())
            .unwrap_or(0)
    }

    pub fn figure_count(&self) -> usize {
        self.figures
            .as_ref()
            .map(|loaded| loaded.data.figures.len())
            .unwrap_or(0)
    }

    pub fn freshness_count(&self) -> usize {
        self.freshness
            .as_ref()
            .map(|loaded| loaded.data.tracked.len())
            .unwrap_or(0)
    }

    pub fn reviewer_notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if let Some(claims) = &self.claims {
            for claim in &claims.data.claims {
                if let Some(note) = claim
                    .reviewer_note
                    .as_ref()
                    .filter(|value| !value.is_empty())
                {
                    notes.push(format!("claim {}: {}", claim.id, note));
                }
            }
        }
        if let Some(figures) = &self.figures {
            for figure in &figures.data.figures {
                if let Some(note) = figure
                    .reviewer_note
                    .as_ref()
                    .filter(|value| !value.is_empty())
                {
                    notes.push(format!("figure {}: {}", figure.id, note));
                }
            }
        }
        if let Some(freshness) = &self.freshness {
            for item in &freshness.data.tracked {
                if let Some(note) = item.note.as_ref().filter(|value| !value.is_empty()) {
                    notes.push(format!(
                        "freshness {} {}: {}",
                        item.kind.as_str(),
                        item.id,
                        note
                    ));
                }
            }
        }
        notes
    }
}

#[derive(Debug, Clone)]
pub struct LoadedStyleGuide {
    pub data: StyleGuide,
}

#[derive(Debug, Clone)]
pub struct LoadedClaimLedger {
    pub path: PathBuf,
    pub data: ClaimLedger,
}

#[derive(Debug, Clone)]
pub struct LoadedFigureLedger {
    pub path: PathBuf,
    pub data: FigureLedger,
}

#[derive(Debug, Clone)]
pub struct LoadedFreshnessLedger {
    pub path: PathBuf,
    pub data: FreshnessLedger,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct StyleGuide {
    pub preferred_terms: Vec<PreferredTermRule>,
    pub banned_terms: Vec<BannedTermRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreferredTermRule {
    pub preferred: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub severity: RuleSeverity,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannedTermRule {
    pub term: String,
    #[serde(default)]
    pub severity: RuleSeverity,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    #[default]
    Warn,
    Error,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ClaimLedger {
    pub claims: Vec<ClaimRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRecord {
    pub id: String,
    pub summary: String,
    pub section: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub reviewer_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct FigureLedger {
    pub figures: Vec<FigureRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureRecord {
    pub id: String,
    pub path: String,
    pub caption: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub rights: Option<String>,
    #[serde(default)]
    pub reviewer_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct FreshnessLedger {
    pub tracked: Vec<FreshnessRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshnessRecord {
    pub kind: FreshnessKind,
    pub id: String,
    pub last_verified: String,
    pub review_due_on: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FreshnessKind {
    Claim,
    Figure,
}

impl FreshnessKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Figure => "figure",
        }
    }
}

#[derive(Debug, Error)]
pub enum EditorialError {
    #[error("failed to read editorial file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse editorial YAML in {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
}

pub fn load_bundle(resolved: &ResolvedBookConfig) -> Result<EditorialBundle, EditorialError> {
    let editorial = &resolved.effective.editorial;
    Ok(EditorialBundle {
        style: load_optional::<StyleGuide>(resolved, editorial.style.as_ref())?
            .map(|(_, data)| LoadedStyleGuide { data }),
        claims: load_optional::<ClaimLedger>(resolved, editorial.claims.as_ref())?
            .map(|(path, data)| LoadedClaimLedger { path, data }),
        figures: load_optional::<FigureLedger>(resolved, editorial.figures.as_ref())?
            .map(|(path, data)| LoadedFigureLedger { path, data }),
        freshness: load_optional::<FreshnessLedger>(resolved, editorial.freshness.as_ref())?
            .map(|(path, data)| LoadedFreshnessLedger { path, data }),
    })
}

pub fn configured_files(resolved: &ResolvedBookConfig) -> Vec<(RepoPath, PathBuf)> {
    let editorial = &resolved.effective.editorial;
    [
        editorial.style.as_ref(),
        editorial.claims.as_ref(),
        editorial.figures.as_ref(),
        editorial.freshness.as_ref(),
    ]
    .into_iter()
    .flatten()
    .map(|path| (path.clone(), join_repo_path(&resolved.repo.repo_root, path)))
    .collect()
}

pub fn parse_iso_date(value: &str) -> Option<Date> {
    static DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");
    Date::parse(value, DATE_FORMAT).ok()
}

pub fn today_local() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

fn load_optional<T>(
    resolved: &ResolvedBookConfig,
    path: Option<&RepoPath>,
) -> Result<Option<(PathBuf, T)>, EditorialError>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(path) = path else {
        return Ok(None);
    };
    let fs_path = join_repo_path(&resolved.repo.repo_root, path);
    let display = fs_path.display().to_string();
    let contents = fs::read_to_string(&fs_path).map_err(|source| EditorialError::Read {
        path: display.clone(),
        source,
    })?;
    let data = serde_yaml::from_str(&contents).map_err(|source| EditorialError::Parse {
        path: display,
        source,
    })?;
    Ok(Some((fs_path, data)))
}
