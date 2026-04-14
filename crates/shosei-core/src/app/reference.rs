use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config,
    diagnostics::{IssueLocation, Severity, ValidationIssue},
    domain::{RepoMode, RepoPath, RepoPathError},
    editorial,
    fs::join_repo_path,
    markdown::parse_frontmatter,
    repo::{self, RepoError},
};

const CLAIM_SOURCE_REFERENCE_PREFIX: &str = "ref:";

#[derive(Debug, Clone)]
pub struct ReferenceScaffoldOptions {
    pub shared: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceScaffoldResult {
    pub summary: String,
    pub references_root: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceMapOptions {
    pub shared: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceMapResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceCheckOptions {
    pub shared: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceCheckResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub issue_count: usize,
    pub issues: Vec<ValidationIssue>,
    pub has_errors: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceDriftOptions {}

#[derive(Debug, Clone)]
pub struct ReferenceDriftResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceSyncOptions {
    pub source: Option<String>,
    pub destination: Option<String>,
    pub id: Option<String>,
    pub report: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceSyncResult {
    pub summary: String,
    pub target_path: Option<PathBuf>,
    pub changed: bool,
    pub changed_count: usize,
    pub skipped_count: usize,
    pub requested_count: usize,
}

#[derive(Debug, Error)]
pub enum ReferenceScaffoldError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("reference shared scaffold is only supported in series repositories")]
    SharedRequiresSeries,
    #[error("use either --shared or --book, not both")]
    ConflictingScope,
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum ReferenceMapError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("reference shared map is only supported in series repositories")]
    SharedRequiresSeries,
    #[error("use either --shared or --book, not both")]
    ConflictingScope,
    #[error("reference entries directory not found: {path}")]
    MissingEntriesDir { path: PathBuf },
    #[error("failed to scan reference entries directory {path}: {source}")]
    ScanEntriesDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read reference entry {path}: {source}")]
    ReadEntry {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse reference entry frontmatter {path}: {source}")]
    ParseEntryFrontmatter {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("reference entry `{path}` has invalid frontmatter: {detail}")]
    InvalidEntryFrontmatter { path: PathBuf, detail: String },
    #[error("reference entry `{path}` must have a non-empty `id` or filename stem")]
    InvalidEntryId { path: PathBuf },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write reference map report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize reference map report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum ReferenceCheckError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("reference shared check is only supported in series repositories")]
    SharedRequiresSeries,
    #[error("use either --shared or --book, not both")]
    ConflictingScope,
    #[error("reference entries directory not found: {path}")]
    MissingEntriesDir { path: PathBuf },
    #[error("failed to scan reference entries directory {path}: {source}")]
    ScanEntriesDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read reference entry {path}: {source}")]
    ReadEntry {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write reference check report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize reference check report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum ReferenceDriftError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("reference drift is only supported in series repositories")]
    SeriesOnly,
    #[error("failed to scan reference entries directory {path}: {source}")]
    ScanEntriesDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read reference entry {path}: {source}")]
    ReadEntry {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write reference drift report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize reference drift report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum ReferenceSyncError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("reference sync is only supported in series repositories")]
    SeriesOnly,
    #[error("unsupported source `{value}`")]
    UnsupportedSource { value: String },
    #[error("unsupported destination `{value}`")]
    UnsupportedDestination { value: String },
    #[error("use exactly one of --from shared or --to shared")]
    InvalidDirection,
    #[error("use either --id <id> or --report <path>")]
    InvalidSelection,
    #[error("report sync requires --force")]
    ReportSyncRequiresForce,
    #[error("shared reference `{id}` was not found")]
    MissingSharedEntry { id: String },
    #[error("book reference `{id}` was not found")]
    MissingBookEntry { id: String },
    #[error(
        "shared reference `{id}` differs from the selected source; rerun with --force to overwrite"
    )]
    SharedEntryConflict { id: String },
    #[error(
        "book reference `{id}` differs from the selected source; rerun with --force to overwrite"
    )]
    BookEntryConflict { id: String },
    #[error("reference sync found duplicate {scope} entry for `{id}`")]
    DuplicateEntry { scope: String, id: String },
    #[error("failed to scan reference entries directory {path}: {source}")]
    ScanDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse reference entry {path}: {detail}")]
    ParseEntry { path: PathBuf, detail: String },
    #[error("reference entry `{path}` must have a non-empty `id` or filename stem")]
    InvalidEntryId { path: PathBuf },
    #[error("target path already exists and does not match the selected reference id: {path}")]
    TargetPathConflict { path: PathBuf },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read reference drift report {path}: {source}")]
    ReadReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse reference drift report {path}: {source}")]
    ParseReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("reference drift report was created for `{actual}`, not `{expected}`")]
    ReportBookMismatch { expected: String, actual: String },
    #[error("reference drift report contains duplicate entry for `{id}`")]
    DuplicateReportEntry { id: String },
    #[error("reference drift report contains invalid repo path `{value}`: {source}")]
    InvalidReportPath {
        value: String,
        #[source]
        source: RepoPathError,
    },
}

#[derive(Debug, Clone)]
enum ReferenceScope {
    SingleBook,
    SeriesBook { book_id: String },
    SharedSeries,
}

#[derive(Debug, Clone)]
struct ReferenceWorkspace {
    repo_root: PathBuf,
    references_root: PathBuf,
    entries_root: PathBuf,
    scope: ReferenceScope,
}

#[derive(Debug, Clone)]
struct BookReferenceWorkspace {
    repo_root: PathBuf,
    book_id: String,
    book_references_root: PathBuf,
    book_entries_root: PathBuf,
    shared_references_root: PathBuf,
    shared_entries_root: PathBuf,
}

#[derive(Debug)]
enum ReferenceScopeFailure {
    Repo(RepoError),
    SharedRequiresSeries,
    ConflictingScope,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct ReferenceEntryFrontmatter {
    id: Option<String>,
    title: Option<String>,
    links: Vec<String>,
    tags: Vec<String>,
    related_sections: Vec<String>,
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReferenceMapEntry {
    file: String,
    id: String,
    title: Option<String>,
    links: Vec<String>,
    link_count: usize,
    tags: Vec<String>,
    related_sections: Vec<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReferenceMapReport {
    scope: String,
    book_id: Option<String>,
    references_root: String,
    entries_root: String,
    entry_count: usize,
    entries: Vec<ReferenceMapEntry>,
}

#[derive(Debug, Serialize)]
struct ReferenceCheckReport {
    scope: String,
    book_id: Option<String>,
    references_root: String,
    entries_root: String,
    entry_count: usize,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug)]
struct ReferenceCheckScan {
    entry_count: usize,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Copy)]
struct ReferenceTargetValidation<'a> {
    field_name: &'a str,
    label: &'a str,
    invalid_remedy: &'a str,
    missing_remedy: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ReferenceDriftStatus {
    RedundantCopy,
    Drift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReferenceDriftEntry {
    id: String,
    status: ReferenceDriftStatus,
    shared_path: String,
    book_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ReferenceGapStatus {
    SharedOnly,
    BookOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReferenceGapEntry {
    id: String,
    status: ReferenceGapStatus,
    path: String,
}

#[derive(Debug, Serialize)]
struct ReferenceDriftReport {
    book_id: String,
    shared_references_root: String,
    book_references_root: String,
    shared_entry_count: usize,
    book_entry_count: usize,
    drifts: Vec<ReferenceDriftEntry>,
    gaps: Vec<ReferenceGapEntry>,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceEntryScope {
    Shared,
    Book,
}

#[derive(Debug, Clone)]
struct ReferenceDriftCandidate {
    path: PathBuf,
    scope: ReferenceEntryScope,
    contents: String,
}

#[derive(Debug, Clone, Copy)]
enum ReferenceSyncDirection {
    FromShared,
    ToShared,
}

#[derive(Debug, Clone)]
enum ReferenceSyncTarget {
    Single { id: String },
    Report { path: PathBuf },
}

#[derive(Debug, Clone, Deserialize)]
struct ReferenceSyncReportInput {
    book_id: String,
    #[serde(default)]
    drifts: Vec<ReferenceDriftEntry>,
    #[serde(default)]
    gaps: Vec<ReferenceGapEntry>,
}

#[derive(Debug)]
struct ReferenceSyncPlan {
    target_path: PathBuf,
    contents_to_write: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ReferenceSyncEndpoints<'a> {
    source_entries_root: &'a Path,
    source_scope: ReferenceEntryScope,
    destination_entries_root: &'a Path,
    destination_scope: ReferenceEntryScope,
}

pub fn reference_scaffold(
    command: &CommandContext,
    options: ReferenceScaffoldOptions,
) -> Result<ReferenceScaffoldResult, ReferenceScaffoldError> {
    let workspace = discover_reference_workspace(
        &command.start_path,
        command.book_id.as_deref(),
        options.shared,
    )
    .map_err(map_reference_scope_error_for_scaffold)?;

    let mut created = Vec::new();
    let mut kept = Vec::new();
    scaffold_reference_workspace(
        &workspace.repo_root,
        &workspace.references_root,
        &workspace.scope,
        options.force,
        &mut created,
        &mut kept,
    )?;

    let mut lines = vec![format!(
        "reference scaffold: initialized {} at {}",
        workspace.scope.label(),
        workspace.references_root.display()
    )];
    lines.extend(created.into_iter().map(|path| format!("- created {path}")));
    lines.extend(kept.into_iter().map(|path| format!("- kept {path}")));

    Ok(ReferenceScaffoldResult {
        summary: lines.join("\n"),
        references_root: workspace.references_root,
    })
}

pub fn reference_map(
    command: &CommandContext,
    options: ReferenceMapOptions,
) -> Result<ReferenceMapResult, ReferenceMapError> {
    let workspace = discover_reference_workspace(
        &command.start_path,
        command.book_id.as_deref(),
        options.shared,
    )
    .map_err(map_reference_scope_error_for_map)?;

    if !workspace.entries_root.is_dir() {
        return Err(ReferenceMapError::MissingEntriesDir {
            path: workspace.entries_root.clone(),
        });
    }

    let entries = collect_reference_entries(&workspace)?;
    let report = ReferenceMapReport {
        scope: workspace.scope.report_scope().to_string(),
        book_id: workspace.scope.report_book_id(),
        references_root: relative_repo_path(&workspace.repo_root, &workspace.references_root),
        entries_root: relative_repo_path(&workspace.repo_root, &workspace.entries_root),
        entry_count: entries.len(),
        entries: entries.clone(),
    };
    let report_path = reference_map_report_path(&workspace.repo_root, &workspace.scope);
    write_reference_map_report(&report_path, &report)?;

    let mut lines = vec![format!(
        "reference map: {} entry(s) from {} (report: {})",
        report.entry_count,
        report.entries_root,
        report_path.display()
    )];
    lines.extend(
        entries
            .iter()
            .enumerate()
            .map(|(index, entry)| format!("- {}. {}", index + 1, reference_entry_summary(entry))),
    );

    Ok(ReferenceMapResult {
        summary: lines.join("\n"),
        report_path,
        entry_count: report.entry_count,
    })
}

pub fn reference_check(
    command: &CommandContext,
    options: ReferenceCheckOptions,
) -> Result<ReferenceCheckResult, ReferenceCheckError> {
    let workspace = discover_reference_workspace(
        &command.start_path,
        command.book_id.as_deref(),
        options.shared,
    )
    .map_err(map_reference_scope_error_for_check)?;

    if !workspace.entries_root.is_dir() {
        return Err(ReferenceCheckError::MissingEntriesDir {
            path: workspace.entries_root.clone(),
        });
    }

    let scan = collect_reference_check_issues(command, &workspace)?;
    let report = ReferenceCheckReport {
        scope: workspace.scope.report_scope().to_string(),
        book_id: workspace.scope.report_book_id(),
        references_root: relative_repo_path(&workspace.repo_root, &workspace.references_root),
        entries_root: relative_repo_path(&workspace.repo_root, &workspace.entries_root),
        entry_count: scan.entry_count,
        issues: scan.issues.clone(),
    };
    let report_path = reference_check_report_path(&workspace.repo_root, &workspace.scope);
    write_reference_check_report(&report_path, &report)?;
    let has_errors = scan
        .issues
        .iter()
        .any(|issue| issue.severity == Severity::Error);

    Ok(ReferenceCheckResult {
        summary: format!(
            "reference check completed for {} with {} entry(s), issues: {}, report: {}",
            scope_summary_label(&workspace.scope),
            report.entry_count,
            scan.issues.len(),
            report_path.display()
        ),
        report_path,
        issue_count: scan.issues.len(),
        issues: scan.issues,
        has_errors,
    })
}

pub fn reference_drift(
    command: &CommandContext,
    _options: ReferenceDriftOptions,
) -> Result<ReferenceDriftResult, ReferenceDriftError> {
    let workspace = discover_book_reference_workspace_for_drift(command)?;
    let mut issues = Vec::new();
    let shared_entries = collect_reference_drift_scope(
        &workspace.repo_root,
        &workspace.shared_entries_root,
        ReferenceEntryScope::Shared,
        &mut issues,
    )?;
    let book_entries = collect_reference_drift_scope(
        &workspace.repo_root,
        &workspace.book_entries_root,
        ReferenceEntryScope::Book,
        &mut issues,
    )?;

    let mut drifts = Vec::new();
    let mut gaps = Vec::new();
    for (id, shared) in &shared_entries {
        let Some(book) = book_entries.get(id) else {
            issues.push(
                ValidationIssue::warning(
                    "reference",
                    format!("shared-only reference gap for `{id}`"),
                    format!(
                        "`{}` にだけ存在します。book 側でも使うなら `reference sync --book {} --from shared --id {}` を検討してください。",
                        relative_display(&workspace.repo_root, &shared.path),
                        workspace.book_id,
                        id
                    ),
                )
                .at(shared.path.clone()),
            );
            gaps.push(ReferenceGapEntry {
                id: id.clone(),
                status: ReferenceGapStatus::SharedOnly,
                path: relative_repo_path(&workspace.repo_root, &shared.path),
            });
            continue;
        };

        let status = if shared.contents == book.contents {
            issues.push(
                ValidationIssue::warning(
                    "reference",
                    format!("redundant shared/book reference copy for `{id}`"),
                    format!(
                        "`{}` と `{}` は同じ内容です。shared か book のどちらを source of truth にするか決めて整理してください。",
                        relative_display(&workspace.repo_root, &shared.path),
                        relative_display(&workspace.repo_root, &book.path)
                    ),
                )
                .at(book.path.clone()),
            );
            ReferenceDriftStatus::RedundantCopy
        } else {
            issues.push(
                ValidationIssue::error(
                    "reference",
                    format!("shared reference drift for `{id}`"),
                    format!(
                        "`{}` と `{}` の内容が分岐しています。shared を正とするか、book 側の差分を明示的に整理してください。",
                        relative_display(&workspace.repo_root, &shared.path),
                        relative_display(&workspace.repo_root, &book.path)
                    ),
                )
                .at(book.path.clone()),
            );
            ReferenceDriftStatus::Drift
        };

        drifts.push(ReferenceDriftEntry {
            id: id.clone(),
            status,
            shared_path: relative_repo_path(&workspace.repo_root, &shared.path),
            book_path: relative_repo_path(&workspace.repo_root, &book.path),
        });
    }
    for (id, book) in &book_entries {
        if shared_entries.contains_key(id) {
            continue;
        }
        issues.push(
            ValidationIssue::warning(
                "reference",
                format!("book-only reference gap for `{id}`"),
                format!(
                    "`{}` にだけ存在します。shared へ寄せたいなら `reference sync --book {} --to shared --id {}` を検討してください。",
                    relative_display(&workspace.repo_root, &book.path),
                    workspace.book_id,
                    id
                ),
            )
            .at(book.path.clone()),
        );
        gaps.push(ReferenceGapEntry {
            id: id.clone(),
            status: ReferenceGapStatus::BookOnly,
            path: relative_repo_path(&workspace.repo_root, &book.path),
        });
    }
    drifts.sort_by(|left, right| left.id.cmp(&right.id));
    gaps.sort_by(|left, right| left.id.cmp(&right.id));

    let report = ReferenceDriftReport {
        book_id: workspace.book_id.clone(),
        shared_references_root: relative_repo_path(
            &workspace.repo_root,
            &workspace.shared_references_root,
        ),
        book_references_root: relative_repo_path(
            &workspace.repo_root,
            &workspace.book_references_root,
        ),
        shared_entry_count: shared_entries.len(),
        book_entry_count: book_entries.len(),
        drifts,
        gaps,
        issues: issues.clone(),
    };
    let report_path = reference_drift_report_path(&workspace.repo_root, &workspace.book_id);
    write_reference_drift_report(&report_path, &report)?;
    let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);

    Ok(ReferenceDriftResult {
        summary: format!(
            "reference drift completed for {} with issues: {}, report: {}",
            workspace.book_id,
            issues.len(),
            report_path.display()
        ),
        report_path,
        issue_count: issues.len(),
        has_errors,
    })
}

pub fn reference_sync(
    command: &CommandContext,
    options: ReferenceSyncOptions,
) -> Result<ReferenceSyncResult, ReferenceSyncError> {
    let workspace = discover_book_reference_workspace_for_sync(command)?;
    let direction = reference_sync_direction(&options)?;
    match reference_sync_target(&options)? {
        ReferenceSyncTarget::Single { id } => {
            let endpoints = match direction {
                ReferenceSyncDirection::FromShared => ReferenceSyncEndpoints {
                    source_entries_root: &workspace.shared_entries_root,
                    source_scope: ReferenceEntryScope::Shared,
                    destination_entries_root: &workspace.book_entries_root,
                    destination_scope: ReferenceEntryScope::Book,
                },
                ReferenceSyncDirection::ToShared => ReferenceSyncEndpoints {
                    source_entries_root: &workspace.book_entries_root,
                    source_scope: ReferenceEntryScope::Book,
                    destination_entries_root: &workspace.shared_entries_root,
                    destination_scope: ReferenceEntryScope::Shared,
                },
            };
            sync_reference_entry(&workspace, &id, options.force, endpoints)
        }
        ReferenceSyncTarget::Report { path } => sync_reference_report(&workspace, direction, &path),
    }
}

impl ReferenceScope {
    fn label(&self) -> String {
        match self {
            Self::SingleBook => "single-book reference workspace".to_string(),
            Self::SeriesBook { book_id } => format!("reference workspace for {book_id}"),
            Self::SharedSeries => "shared series reference workspace".to_string(),
        }
    }

    fn report_scope(&self) -> &'static str {
        match self {
            Self::SingleBook => "single-book",
            Self::SeriesBook { .. } => "series-book",
            Self::SharedSeries => "shared-series",
        }
    }

    fn report_book_id(&self) -> Option<String> {
        match self {
            Self::SeriesBook { book_id } => Some(book_id.clone()),
            _ => None,
        }
    }
}

fn discover_reference_workspace(
    start_path: &Path,
    selected_book: Option<&str>,
    shared: bool,
) -> Result<ReferenceWorkspace, ReferenceScopeFailure> {
    let context = repo::discover(start_path, selected_book).map_err(ReferenceScopeFailure::Repo)?;
    let repo_root = context.repo_root.clone();

    if shared && selected_book.is_some() {
        return Err(ReferenceScopeFailure::ConflictingScope);
    }

    let (references_root, scope) = if shared {
        if context.mode != RepoMode::Series {
            return Err(ReferenceScopeFailure::SharedRequiresSeries);
        }
        (
            repo_root.join("shared/metadata/references"),
            ReferenceScope::SharedSeries,
        )
    } else {
        match context.mode {
            RepoMode::SingleBook => (repo_root.join("references"), ReferenceScope::SingleBook),
            RepoMode::Series => {
                let context =
                    repo::require_book_context(context).map_err(ReferenceScopeFailure::Repo)?;
                let book = context.book.expect("series book must be resolved");
                (
                    book.root.join("references"),
                    ReferenceScope::SeriesBook { book_id: book.id },
                )
            }
        }
    };
    let entries_root = references_root.join("entries");

    Ok(ReferenceWorkspace {
        repo_root,
        references_root,
        entries_root,
        scope,
    })
}

fn discover_book_reference_workspace_for_drift(
    command: &CommandContext,
) -> Result<BookReferenceWorkspace, ReferenceDriftError> {
    let context = repo::discover(&command.start_path, command.book_id.as_deref())
        .map_err(ReferenceDriftError::Repo)?;
    if context.mode != RepoMode::Series {
        return Err(ReferenceDriftError::SeriesOnly);
    }
    let context = repo::require_book_context(context).map_err(ReferenceDriftError::Repo)?;
    let book = context.book.expect("series book must be resolved");

    Ok(BookReferenceWorkspace {
        repo_root: context.repo_root.clone(),
        book_id: book.id,
        book_references_root: book.root.join("references"),
        book_entries_root: book.root.join("references/entries"),
        shared_references_root: context.repo_root.join("shared/metadata/references"),
        shared_entries_root: context.repo_root.join("shared/metadata/references/entries"),
    })
}

fn discover_book_reference_workspace_for_sync(
    command: &CommandContext,
) -> Result<BookReferenceWorkspace, ReferenceSyncError> {
    let context = repo::discover(&command.start_path, command.book_id.as_deref())
        .map_err(ReferenceSyncError::Repo)?;
    if context.mode != RepoMode::Series {
        return Err(ReferenceSyncError::SeriesOnly);
    }
    let context = repo::require_book_context(context).map_err(ReferenceSyncError::Repo)?;
    let book = context.book.expect("series book must be resolved");

    Ok(BookReferenceWorkspace {
        repo_root: context.repo_root.clone(),
        book_id: book.id,
        book_references_root: book.root.join("references"),
        book_entries_root: book.root.join("references/entries"),
        shared_references_root: context.repo_root.join("shared/metadata/references"),
        shared_entries_root: context.repo_root.join("shared/metadata/references/entries"),
    })
}

fn scaffold_reference_workspace(
    repo_root: &Path,
    references_root: &Path,
    scope: &ReferenceScope,
    force: bool,
    created: &mut Vec<String>,
    kept: &mut Vec<String>,
) -> Result<(), ReferenceScaffoldError> {
    ensure_dir_for_scaffold(references_root)?;
    write_scaffold_file(
        repo_root,
        &references_root.join("README.md"),
        &reference_root_readme(scope),
        force,
        created,
        kept,
    )?;
    write_scaffold_file(
        repo_root,
        &references_root.join("entries/README.md"),
        entries_readme(),
        force,
        created,
        kept,
    )?;
    Ok(())
}

fn collect_reference_entries(
    workspace: &ReferenceWorkspace,
) -> Result<Vec<ReferenceMapEntry>, ReferenceMapError> {
    reference_entry_paths(&workspace.entries_root)
        .map_err(|source| ReferenceMapError::ScanEntriesDir {
            path: workspace.entries_root.clone(),
            source,
        })?
        .into_iter()
        .map(|path| load_reference_entry(workspace, &path))
        .collect()
}

fn collect_reference_check_issues(
    command: &CommandContext,
    workspace: &ReferenceWorkspace,
) -> Result<ReferenceCheckScan, ReferenceCheckError> {
    let paths = reference_entry_paths(&workspace.entries_root).map_err(|source| {
        ReferenceCheckError::ScanEntriesDir {
            path: workspace.entries_root.clone(),
            source,
        }
    })?;
    let mut issues = Vec::new();
    let mut ids = HashMap::<String, PathBuf>::new();
    let mut available_reference_ids = HashSet::new();

    for path in &paths {
        let contents =
            fs::read_to_string(path).map_err(|source| ReferenceCheckError::ReadEntry {
                path: path.clone(),
                source,
            })?;
        let Some(frontmatter) = parse_reference_frontmatter_for_check(path, &contents, &mut issues)
        else {
            continue;
        };

        let Some(id) = reference_entry_id(path, frontmatter.id.as_deref()) else {
            issues.push(
                ValidationIssue::error(
                    "reference",
                    "reference entry must have a non-empty `id` or filename stem",
                    "`id` を non-empty string にするか、entry filename を見直してください。",
                )
                .at_line(path.clone(), 1),
            );
            continue;
        };

        available_reference_ids.insert(id.clone());

        if let Some(previous) = ids.insert(id.clone(), path.clone()) {
            issues.push(
                ValidationIssue::error(
                    "reference",
                    format!("duplicate reference id `{id}`"),
                    format!(
                        "`{}` と `{}` で同じ `id` を使わないでください。",
                        relative_display(&workspace.repo_root, &previous),
                        relative_display(&workspace.repo_root, path)
                    ),
                )
                .at(path.clone()),
            );
        }

        validate_reference_targets(
            &workspace.repo_root,
            path,
            &frontmatter.links,
            ReferenceTargetValidation {
                field_name: "links",
                label: "reference link",
                invalid_remedy: "`links` には repo-relative path か URL を入れてください。",
                missing_remedy: "対象 file を作成するか、`links` の path を修正してください。",
            },
            &mut issues,
        );
        validate_reference_targets(
            &workspace.repo_root,
            path,
            &frontmatter.related_sections,
            ReferenceTargetValidation {
                field_name: "related_sections",
                label: "related section",
                invalid_remedy: "`related_sections` には repo-relative かつ `/` 区切りの path を入れてください。",
                missing_remedy: "対象 section file を作成するか、`related_sections` の path を修正してください。",
            },
            &mut issues,
        );
    }
    issues.extend(collect_claim_source_reference_issues(
        command,
        workspace,
        &available_reference_ids,
    ));

    Ok(ReferenceCheckScan {
        entry_count: paths.len(),
        issues,
    })
}

fn collect_claim_source_reference_issues(
    command: &CommandContext,
    workspace: &ReferenceWorkspace,
    current_reference_ids: &HashSet<String>,
) -> Vec<ValidationIssue> {
    if matches!(workspace.scope, ReferenceScope::SharedSeries) {
        return Vec::new();
    }

    let context = match repo::discover(&command.start_path, command.book_id.as_deref())
        .and_then(repo::require_book_context)
    {
        Ok(context) => context,
        Err(_) => return Vec::new(),
    };
    let resolved = match config::resolve_book_config(&context) {
        Ok(resolved) => resolved,
        Err(_) => return Vec::new(),
    };
    if !resolved.effective.project.project_type.is_prose() {
        return Vec::new();
    }
    let claims = match editorial::load_claims(&resolved) {
        Ok(claims) => claims,
        Err(source) => return vec![claim_ledger_load_issue(source)],
    };
    let Some(claims) = claims else {
        return Vec::new();
    };

    let claims_contents = fs::read_to_string(&claims.path).ok();
    let mut available_reference_ids = current_reference_ids.clone();
    if matches!(workspace.scope, ReferenceScope::SeriesBook { .. }) {
        available_reference_ids.extend(collect_reference_ids_if_present(
            &workspace
                .repo_root
                .join("shared/metadata/references/entries"),
        ));
    }

    let mut issues = Vec::new();
    for claim in &claims.data.claims {
        for source in &claim.sources {
            let trimmed = source.trim();
            let Some(reference_id) = trimmed.strip_prefix(CLAIM_SOURCE_REFERENCE_PREFIX) else {
                continue;
            };
            let location = yaml_sequence_item_location(
                &claims.path,
                claims_contents.as_deref(),
                "sources",
                trimmed,
            );
            let reference_id = reference_id.trim();
            if reference_id.is_empty() {
                issues.push(
                    ValidationIssue::error(
                        "reference",
                        format!("claim `{}` has empty reference source `ref:`", claim.id),
                        "`claims.yml` の source は `ref:<id>` の形で指定してください。",
                    )
                    .at_location(location),
                );
                continue;
            }
            if !available_reference_ids.contains(reference_id) {
                issues.push(
                    ValidationIssue::error(
                        "reference",
                        format!(
                            "claim `{}` references missing source `ref:{}`",
                            claim.id, reference_id
                        ),
                        "対応する reference entry を追加するか、`claims.yml` の source id を修正してください。",
                    )
                    .at_location(location),
                );
            }
        }
    }

    issues
}

fn collect_reference_drift_scope(
    repo_root: &Path,
    entries_root: &Path,
    scope: ReferenceEntryScope,
    issues: &mut Vec<ValidationIssue>,
) -> Result<HashMap<String, ReferenceDriftCandidate>, ReferenceDriftError> {
    let paths = reference_entry_paths_if_present(entries_root).map_err(|source| {
        ReferenceDriftError::ScanEntriesDir {
            path: entries_root.to_path_buf(),
            source,
        }
    })?;
    let mut entries = HashMap::new();

    for path in paths {
        let contents =
            fs::read_to_string(&path).map_err(|source| ReferenceDriftError::ReadEntry {
                path: path.clone(),
                source,
            })?;
        let Some(frontmatter) = parse_reference_frontmatter_for_check(&path, &contents, issues)
        else {
            continue;
        };
        let Some(id) = reference_entry_id(&path, frontmatter.id.as_deref()) else {
            issues.push(
                ValidationIssue::error(
                    "reference",
                    "reference entry must have a non-empty `id` or filename stem",
                    "`id` を non-empty string にするか、entry filename を見直してください。",
                )
                .at_line(path.clone(), 1),
            );
            continue;
        };

        let candidate = ReferenceDriftCandidate {
            path: path.clone(),
            scope,
            contents,
        };
        if let Some(previous) = entries.insert(id.clone(), candidate) {
            issues.push(reference_same_scope_duplicate_issue(
                repo_root,
                &id,
                &previous.path,
                &path,
                scope,
            ));
            entries.insert(
                id,
                ReferenceDriftCandidate {
                    path: previous.path,
                    scope: previous.scope,
                    contents: previous.contents,
                },
            );
        }
    }

    Ok(entries)
}

fn reference_sync_direction(
    options: &ReferenceSyncOptions,
) -> Result<ReferenceSyncDirection, ReferenceSyncError> {
    match (options.source.as_deref(), options.destination.as_deref()) {
        (Some("shared"), None) => Ok(ReferenceSyncDirection::FromShared),
        (None, Some("shared")) => Ok(ReferenceSyncDirection::ToShared),
        (Some(value), None) => Err(ReferenceSyncError::UnsupportedSource {
            value: value.to_string(),
        }),
        (None, Some(value)) => Err(ReferenceSyncError::UnsupportedDestination {
            value: value.to_string(),
        }),
        _ => Err(ReferenceSyncError::InvalidDirection),
    }
}

fn reference_sync_target(
    options: &ReferenceSyncOptions,
) -> Result<ReferenceSyncTarget, ReferenceSyncError> {
    match (&options.report, options.id.as_deref()) {
        (Some(_), None) => {
            if !options.force {
                return Err(ReferenceSyncError::ReportSyncRequiresForce);
            }
            Ok(ReferenceSyncTarget::Report {
                path: options.report.clone().expect("checked above"),
            })
        }
        (Some(_), Some(_)) => Err(ReferenceSyncError::InvalidSelection),
        (None, Some(id)) => Ok(ReferenceSyncTarget::Single { id: id.to_string() }),
        (None, None) => Err(ReferenceSyncError::InvalidSelection),
    }
}

fn sync_reference_entry(
    workspace: &BookReferenceWorkspace,
    id: &str,
    force: bool,
    endpoints: ReferenceSyncEndpoints<'_>,
) -> Result<ReferenceSyncResult, ReferenceSyncError> {
    let source_entry =
        find_reference_entry_by_id(endpoints.source_entries_root, endpoints.source_scope, id)?
            .ok_or_else(|| missing_reference_entry_error(endpoints.source_scope, id))?;
    let destination_entry = find_reference_entry_by_id(
        endpoints.destination_entries_root,
        endpoints.destination_scope,
        id,
    )?;
    let target_path = if let Some(entry) = &destination_entry {
        entry.path.clone()
    } else {
        endpoints.destination_entries_root.join(
            source_entry
                .path
                .file_name()
                .expect("reference entry file name must exist"),
        )
    };

    if let Some(entry) = &destination_entry {
        if entry.contents == source_entry.contents {
            return Ok(ReferenceSyncResult {
                summary: format!(
                    "reference sync: `{}` already matches {} at {}",
                    id,
                    endpoints.source_scope.label(),
                    relative_display(&workspace.repo_root, &entry.path)
                ),
                target_path: Some(entry.path.clone()),
                changed: false,
                changed_count: 0,
                skipped_count: 0,
                requested_count: 1,
            });
        }
        if !force {
            return Err(conflicting_reference_entry_error(
                endpoints.destination_scope,
                id,
            ));
        }
    } else if target_path.exists() {
        return Err(ReferenceSyncError::TargetPathConflict { path: target_path });
    }

    fs::create_dir_all(endpoints.destination_entries_root).map_err(|source| {
        ReferenceSyncError::CreateDir {
            path: endpoints.destination_entries_root.to_path_buf(),
            source,
        }
    })?;
    fs::write(&target_path, &source_entry.contents).map_err(|source| {
        ReferenceSyncError::WriteFile {
            path: target_path.clone(),
            source,
        }
    })?;

    Ok(ReferenceSyncResult {
        summary: format!(
            "reference sync: copied {} reference `{}` to {}",
            endpoints.source_scope.label(),
            id,
            relative_display(&workspace.repo_root, &target_path)
        ),
        target_path: Some(target_path),
        changed: true,
        changed_count: 1,
        skipped_count: 0,
        requested_count: 1,
    })
}

fn sync_reference_report(
    workspace: &BookReferenceWorkspace,
    direction: ReferenceSyncDirection,
    report_path: &Path,
) -> Result<ReferenceSyncResult, ReferenceSyncError> {
    let report = load_reference_sync_report(report_path)?;
    if report.book_id != workspace.book_id {
        return Err(ReferenceSyncError::ReportBookMismatch {
            expected: workspace.book_id.clone(),
            actual: report.book_id,
        });
    }

    let mut seen = HashSet::new();
    let mut plans = Vec::new();
    let mut skipped_count = 0;
    for drift in report.drifts {
        if !seen.insert(drift.id.clone()) {
            return Err(ReferenceSyncError::DuplicateReportEntry { id: drift.id });
        }
        plans.push(prepare_reference_sync_report_plan(
            workspace, direction, drift,
        )?);
    }
    for gap in report.gaps {
        if !seen.insert(gap.id.clone()) {
            return Err(ReferenceSyncError::DuplicateReportEntry { id: gap.id });
        }
        match prepare_reference_sync_gap_plan(workspace, direction, gap)? {
            Some(plan) => plans.push(plan),
            None => skipped_count += 1,
        }
    }

    let changed_count = plans
        .iter()
        .filter(|plan| plan.contents_to_write.is_some())
        .count();
    for plan in &plans {
        if let Some(contents) = &plan.contents_to_write {
            if let Some(parent) = plan.target_path.parent() {
                fs::create_dir_all(parent).map_err(|source| ReferenceSyncError::CreateDir {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::write(&plan.target_path, contents).map_err(|source| {
                ReferenceSyncError::WriteFile {
                    path: plan.target_path.clone(),
                    source,
                }
            })?;
        }
    }

    Ok(ReferenceSyncResult {
        summary: format!(
            "reference sync: applied {} applicable report entries from {} (changed: {}, unchanged: {}, skipped: {})",
            plans.len(),
            report_path.display(),
            changed_count,
            plans.len().saturating_sub(changed_count),
            skipped_count,
        ),
        target_path: Some(report_path.to_path_buf()),
        changed: changed_count > 0,
        changed_count,
        skipped_count,
        requested_count: plans.len() + skipped_count,
    })
}

fn load_reference_sync_report(
    report_path: &Path,
) -> Result<ReferenceSyncReportInput, ReferenceSyncError> {
    let contents =
        fs::read_to_string(report_path).map_err(|source| ReferenceSyncError::ReadReport {
            path: report_path.to_path_buf(),
            source,
        })?;
    serde_json::from_str(&contents).map_err(|source| ReferenceSyncError::ParseReport {
        path: report_path.to_path_buf(),
        source,
    })
}

fn prepare_reference_sync_report_plan(
    workspace: &BookReferenceWorkspace,
    direction: ReferenceSyncDirection,
    drift: ReferenceDriftEntry,
) -> Result<ReferenceSyncPlan, ReferenceSyncError> {
    let (source_value, destination_value) = match direction {
        ReferenceSyncDirection::FromShared => (drift.shared_path, drift.book_path),
        ReferenceSyncDirection::ToShared => (drift.book_path, drift.shared_path),
    };
    let source_repo_path = RepoPath::parse(source_value.clone()).map_err(|source| {
        ReferenceSyncError::InvalidReportPath {
            value: source_value.clone(),
            source,
        }
    })?;
    let destination_repo_path = RepoPath::parse(destination_value.clone()).map_err(|source| {
        ReferenceSyncError::InvalidReportPath {
            value: destination_value.clone(),
            source,
        }
    })?;

    let source_path = join_repo_path(&workspace.repo_root, &source_repo_path);
    let target_path = join_repo_path(&workspace.repo_root, &destination_repo_path);
    let source_contents =
        fs::read_to_string(&source_path).map_err(|source| ReferenceSyncError::ReadFile {
            path: source_path.clone(),
            source,
        })?;
    let contents_to_write = match fs::read_to_string(&target_path) {
        Ok(existing) if existing == source_contents => None,
        Ok(_) => Some(source_contents),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Some(source_contents),
        Err(source) => {
            return Err(ReferenceSyncError::ReadFile {
                path: target_path.clone(),
                source,
            });
        }
    };

    Ok(ReferenceSyncPlan {
        target_path,
        contents_to_write,
    })
}

fn prepare_reference_sync_gap_plan(
    workspace: &BookReferenceWorkspace,
    direction: ReferenceSyncDirection,
    gap: ReferenceGapEntry,
) -> Result<Option<ReferenceSyncPlan>, ReferenceSyncError> {
    if !gap.status.applies_to(direction) {
        return Ok(None);
    }

    let source_repo_path = RepoPath::parse(gap.path.clone()).map_err(|source| {
        ReferenceSyncError::InvalidReportPath {
            value: gap.path.clone(),
            source,
        }
    })?;
    let source_path = join_repo_path(&workspace.repo_root, &source_repo_path);
    let target_root = match direction {
        ReferenceSyncDirection::FromShared => &workspace.book_entries_root,
        ReferenceSyncDirection::ToShared => &workspace.shared_entries_root,
    };
    let file_name = source_path
        .file_name()
        .expect("reference entry file name must exist");
    let target_path = target_root.join(file_name);
    let source_contents =
        fs::read_to_string(&source_path).map_err(|source| ReferenceSyncError::ReadFile {
            path: source_path.clone(),
            source,
        })?;
    let contents_to_write = match fs::read_to_string(&target_path) {
        Ok(existing) if existing == source_contents => None,
        Ok(_) => {
            return Err(ReferenceSyncError::TargetPathConflict { path: target_path });
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Some(source_contents),
        Err(source) => {
            return Err(ReferenceSyncError::ReadFile {
                path: target_path.clone(),
                source,
            });
        }
    };

    Ok(Some(ReferenceSyncPlan {
        target_path,
        contents_to_write,
    }))
}

fn find_reference_entry_by_id(
    entries_root: &Path,
    scope: ReferenceEntryScope,
    id: &str,
) -> Result<Option<ReferenceDriftCandidate>, ReferenceSyncError> {
    let paths = reference_entry_paths_if_present(entries_root).map_err(|source| {
        ReferenceSyncError::ScanDir {
            path: entries_root.to_path_buf(),
            source,
        }
    })?;
    let mut matched = None;

    for path in paths {
        let contents =
            fs::read_to_string(&path).map_err(|source| ReferenceSyncError::ReadFile {
                path: path.clone(),
                source,
            })?;
        let frontmatter =
            parse_frontmatter(&contents).map_err(|source| ReferenceSyncError::ParseEntry {
                path: path.clone(),
                detail: source.to_string(),
            })?;
        let entry_id =
            reference_entry_id(&path, reference_id_from_frontmatter(frontmatter.as_ref()))
                .ok_or_else(|| ReferenceSyncError::InvalidEntryId { path: path.clone() })?;
        if entry_id != id {
            continue;
        }
        if matched.is_some() {
            return Err(ReferenceSyncError::DuplicateEntry {
                scope: scope.label().to_string(),
                id: id.to_string(),
            });
        }
        matched = Some(ReferenceDriftCandidate {
            path,
            scope,
            contents,
        });
    }

    Ok(matched)
}

fn reference_entry_paths(entries_root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(entries_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.eq_ignore_ascii_case("README.md") {
            continue;
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("md"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn reference_entry_paths_if_present(entries_root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if !entries_root.is_dir() {
        return Ok(Vec::new());
    }
    reference_entry_paths(entries_root)
}

fn load_reference_entry(
    workspace: &ReferenceWorkspace,
    path: &Path,
) -> Result<ReferenceMapEntry, ReferenceMapError> {
    let contents = fs::read_to_string(path).map_err(|source| ReferenceMapError::ReadEntry {
        path: path.to_path_buf(),
        source,
    })?;
    let frontmatter = parse_frontmatter(&contents).map_err(|source| {
        ReferenceMapError::InvalidEntryFrontmatter {
            path: path.to_path_buf(),
            detail: source.to_string(),
        }
    })?;
    let frontmatter: ReferenceEntryFrontmatter = if let Some(mapping) = frontmatter {
        serde_yaml::from_value(Value::Mapping(mapping)).map_err(|source| {
            ReferenceMapError::ParseEntryFrontmatter {
                path: path.to_path_buf(),
                source,
            }
        })?
    } else {
        ReferenceEntryFrontmatter::default()
    };

    let file = relative_repo_path(&workspace.repo_root, path);
    let id = reference_entry_id(path, frontmatter.id.as_deref()).ok_or_else(|| {
        ReferenceMapError::InvalidEntryId {
            path: path.to_path_buf(),
        }
    })?;
    let title = non_empty_string(frontmatter.title);
    let status = non_empty_string(frontmatter.status);
    let links = frontmatter
        .links
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let tags = frontmatter
        .tags
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let related_sections = frontmatter
        .related_sections
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();

    Ok(ReferenceMapEntry {
        file,
        id,
        title,
        link_count: links.len(),
        links,
        tags,
        related_sections,
        status,
    })
}

fn parse_reference_frontmatter_for_check(
    path: &Path,
    contents: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<ReferenceEntryFrontmatter> {
    let frontmatter = match parse_frontmatter(contents) {
        Ok(frontmatter) => frontmatter,
        Err(source) => {
            issues.push(
                ValidationIssue::error(
                    "reference",
                    format!("invalid reference entry frontmatter: {source}"),
                    "frontmatter は YAML mapping として閉じてください。",
                )
                .at_line(path.to_path_buf(), 1),
            );
            return None;
        }
    };

    match frontmatter {
        Some(mapping) => {
            match serde_yaml::from_value::<ReferenceEntryFrontmatter>(Value::Mapping(mapping)) {
                Ok(frontmatter) => Some(frontmatter),
                Err(source) => {
                    issues.push(
                    ValidationIssue::error(
                        "reference",
                        format!("invalid reference entry frontmatter shape: {source}"),
                        "`id`, `title`, `status` は string、`links`, `tags`, `related_sections` は string 配列にしてください。",
                    )
                    .at_line(path.to_path_buf(), 1),
                );
                    None
                }
            }
        }
        None => Some(ReferenceEntryFrontmatter::default()),
    }
}

fn reference_id_from_frontmatter(frontmatter: Option<&serde_yaml::Mapping>) -> Option<&str> {
    let frontmatter = frontmatter?;
    match frontmatter.get(Value::String("id".to_string())) {
        Some(Value::String(id)) => Some(id.as_str()),
        _ => None,
    }
}

fn validate_reference_targets(
    repo_root: &Path,
    entry_path: &Path,
    values: &[String],
    validation: ReferenceTargetValidation<'_>,
    issues: &mut Vec<ValidationIssue>,
) {
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || is_external_reference(trimmed) || trimmed.starts_with('#') {
            continue;
        }

        let reference_value = trimmed.split('#').next().unwrap_or(trimmed).trim();
        if reference_value.is_empty() {
            continue;
        }

        match RepoPath::parse(reference_value.to_string()) {
            Ok(repo_path) => {
                if !join_repo_path(repo_root, &repo_path).exists() {
                    issues.push(
                        ValidationIssue::warning(
                            "reference",
                            format!(
                                "{} target not found: {}",
                                validation.label,
                                repo_path.as_str()
                            ),
                            validation.missing_remedy,
                        )
                        .at(entry_path.to_path_buf()),
                    );
                }
            }
            Err(source) => issues.push(reference_path_issue(
                entry_path,
                validation.field_name,
                trimmed,
                validation.label,
                source,
                validation.invalid_remedy,
            )),
        }
    }
}

fn reference_path_issue(
    path: &Path,
    field_name: &str,
    value: &str,
    label: &str,
    source: RepoPathError,
    remedy: &str,
) -> ValidationIssue {
    ValidationIssue::error(
        "reference",
        format!("invalid {label} in `{field_name}`: `{value}` ({source})"),
        remedy,
    )
    .at(path.to_path_buf())
}

fn missing_reference_entry_error(scope: ReferenceEntryScope, id: &str) -> ReferenceSyncError {
    match scope {
        ReferenceEntryScope::Shared => {
            ReferenceSyncError::MissingSharedEntry { id: id.to_string() }
        }
        ReferenceEntryScope::Book => ReferenceSyncError::MissingBookEntry { id: id.to_string() },
    }
}

fn conflicting_reference_entry_error(scope: ReferenceEntryScope, id: &str) -> ReferenceSyncError {
    match scope {
        ReferenceEntryScope::Shared => {
            ReferenceSyncError::SharedEntryConflict { id: id.to_string() }
        }
        ReferenceEntryScope::Book => ReferenceSyncError::BookEntryConflict { id: id.to_string() },
    }
}

fn reference_same_scope_duplicate_issue(
    repo_root: &Path,
    id: &str,
    previous_path: &Path,
    current_path: &Path,
    scope: ReferenceEntryScope,
) -> ValidationIssue {
    ValidationIssue::error(
        "reference",
        format!("duplicate {} reference id `{id}`", scope.label()),
        format!(
            "`{}` と `{}` で同じ `id` を使わないでください。",
            relative_display(repo_root, previous_path),
            relative_display(repo_root, current_path)
        ),
    )
    .at(current_path.to_path_buf())
}

fn is_external_reference(value: &str) -> bool {
    value.contains("://") || value.starts_with("mailto:") || value.starts_with("tel:")
}

fn reference_entry_id(path: &Path, id: Option<&str>) -> Option<String> {
    if let Some(id) = id {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_string());
    }

    path.file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn collect_reference_ids_if_present(entries_root: &Path) -> HashSet<String> {
    let Ok(paths) = reference_entry_paths_if_present(entries_root) else {
        return HashSet::new();
    };

    paths
        .into_iter()
        .filter_map(|path| {
            let contents = fs::read_to_string(&path).ok()?;
            let frontmatter = parse_frontmatter(&contents).ok()?;
            reference_entry_id(&path, reference_id_from_frontmatter(frontmatter.as_ref()))
        })
        .collect()
}

fn claim_ledger_load_issue(source: editorial::EditorialError) -> ValidationIssue {
    match source {
        editorial::EditorialError::Read { path, source } => ValidationIssue::error(
            "reference",
            format!("failed to read claim ledger {path}: {source}"),
            "`editorial.claims` の path を確認するか、claims.yml を追加してください。",
        )
        .at(PathBuf::from(path)),
        editorial::EditorialError::Parse { path, source } => ValidationIssue::error(
            "reference",
            format!("failed to parse claim ledger {path}: {source}"),
            "claims.yml を有効な YAML として修正し、`ref:<id>` は string として書いてください。",
        )
        .at(PathBuf::from(path)),
    }
}

fn yaml_sequence_item_location(
    path: &Path,
    contents: Option<&str>,
    field: &str,
    value: &str,
) -> IssueLocation {
    let patterns = [
        format!("- {value}"),
        format!("- \"{value}\""),
        format!("- '{value}'"),
        format!("{field}: {value}"),
        format!("{field}: \"{value}\""),
        format!("{field}: '{value}'"),
    ];
    location_for_patterns(path, contents, &patterns)
}

fn location_for_patterns(
    path: &Path,
    contents: Option<&str>,
    patterns: &[String],
) -> IssueLocation {
    if let Some(contents) = contents {
        for pattern in patterns {
            if let Some(line) = line_number_of_substring(contents, pattern) {
                return IssueLocation::with_line(path.to_path_buf(), line);
            }
        }
    }
    path.to_path_buf().into()
}

fn line_number_of_substring(contents: &str, needle: &str) -> Option<usize> {
    contents
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
}

fn reference_entry_summary(entry: &ReferenceMapEntry) -> String {
    let mut parts = vec![entry.id.clone()];
    if let Some(title) = &entry.title {
        parts.push(format!("\"{title}\""));
    }
    parts.push(format!("file: {}", entry.file));
    parts.push(format!("links: {}", entry.link_count));
    if let Some(status) = &entry.status {
        parts.push(format!("status: {status}"));
    }
    parts.join(" | ")
}

fn reference_map_report_path(repo_root: &Path, scope: &ReferenceScope) -> PathBuf {
    match scope {
        ReferenceScope::SingleBook => repo_root.join("dist/reports/default-reference-map.json"),
        ReferenceScope::SeriesBook { book_id } => repo_root
            .join("dist/reports")
            .join(format!("{book_id}-reference-map.json")),
        ReferenceScope::SharedSeries => repo_root.join("dist/reports/shared-reference-map.json"),
    }
}

fn reference_check_report_path(repo_root: &Path, scope: &ReferenceScope) -> PathBuf {
    match scope {
        ReferenceScope::SingleBook => repo_root.join("dist/reports/default-reference-check.json"),
        ReferenceScope::SeriesBook { book_id } => repo_root
            .join("dist/reports")
            .join(format!("{book_id}-reference-check.json")),
        ReferenceScope::SharedSeries => repo_root.join("dist/reports/shared-reference-check.json"),
    }
}

fn reference_drift_report_path(repo_root: &Path, book_id: &str) -> PathBuf {
    repo_root
        .join("dist/reports")
        .join(format!("{book_id}-reference-drift.json"))
}

fn write_reference_map_report(
    path: &Path,
    report: &ReferenceMapReport,
) -> Result<(), ReferenceMapError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ReferenceMapError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(report).map_err(|source| {
        ReferenceMapError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, json).map_err(|source| ReferenceMapError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn write_reference_check_report(
    path: &Path,
    report: &ReferenceCheckReport,
) -> Result<(), ReferenceCheckError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ReferenceCheckError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(report).map_err(|source| {
        ReferenceCheckError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, json).map_err(|source| ReferenceCheckError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn write_reference_drift_report(
    path: &Path,
    report: &ReferenceDriftReport,
) -> Result<(), ReferenceDriftError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ReferenceDriftError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(report).map_err(|source| {
        ReferenceDriftError::SerializeReport {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, json).map_err(|source| ReferenceDriftError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn reference_root_readme(scope: &ReferenceScope) -> String {
    match scope {
        ReferenceScope::SingleBook => "# Reference Workspace\n\nThis workspace stores reference links and working notes for this single-book repo.\n\n- `entries/`: one Markdown file per reference or note\n\nGuidelines:\n- Keep one topic or source per file.\n- Use YAML frontmatter for stable fields such as `id`, `title`, `links`, `tags`, `related_sections`, and `status`.\n- Keep repo file paths repo-relative and `/`-separated when you copy them into notes.\n- This scaffold is manual-first. Keep only the references you actually need.\n".to_string(),
        ReferenceScope::SeriesBook { book_id } => format!(
            "# Reference Workspace\n\nThis workspace stores book-scoped reference links and working notes for `{book_id}`.\n\n- `entries/`: one Markdown file per reference or note for this book\n\nGuidelines:\n- Keep one topic or source per file.\n- Use YAML frontmatter for stable fields such as `id`, `title`, `links`, `tags`, `related_sections`, and `status`.\n- In `series`, shared references that multiple books reuse belong under `shared/metadata/references/`.\n- Keep repo file paths repo-relative and `/`-separated when you copy them into notes.\n- This scaffold is manual-first. Keep only the references you actually need.\n"
        ),
        ReferenceScope::SharedSeries => "# Reference Workspace\n\nThis workspace stores shared reference links and reusable notes for a series repo.\n\n- `entries/`: one Markdown file per shared reference or note\n\nGuidelines:\n- Keep one topic or source per file.\n- Use this scope only for references that multiple books may reuse.\n- Keep repo file paths repo-relative and `/`-separated when you copy them into notes.\n- This scaffold is manual-first. Keep only the references you actually need.\n".to_string(),
    }
}

fn entries_readme() -> &'static str {
    "# Reference Entries\n\nUse one Markdown file per reference item.\nSuggested frontmatter: `id`, `title`, `links`, `tags`, `related_sections`, `status`.\nKeep the body for free-form notes, extracted points, and follow-up decisions.\n"
}

fn ensure_dir_for_scaffold(path: &Path) -> Result<(), ReferenceScaffoldError> {
    fs::create_dir_all(path).map_err(|source| ReferenceScaffoldError::CreateDir {
        path: path.display().to_string(),
        source,
    })
}

fn write_scaffold_file(
    repo_root: &Path,
    path: &Path,
    contents: &str,
    force: bool,
    created: &mut Vec<String>,
    kept: &mut Vec<String>,
) -> Result<(), ReferenceScaffoldError> {
    if let Some(parent) = path.parent() {
        ensure_dir_for_scaffold(parent)?;
    }

    let display_path = relative_display(repo_root, path);
    if path.exists() && !force {
        kept.push(display_path);
        return Ok(());
    }

    fs::write(path, contents).map_err(|source| ReferenceScaffoldError::WriteFile {
        path: path.display().to_string(),
        source,
    })?;
    created.push(display_path);
    Ok(())
}

fn map_reference_scope_error_for_scaffold(error: ReferenceScopeFailure) -> ReferenceScaffoldError {
    match error {
        ReferenceScopeFailure::Repo(source) => ReferenceScaffoldError::Repo(source),
        ReferenceScopeFailure::SharedRequiresSeries => ReferenceScaffoldError::SharedRequiresSeries,
        ReferenceScopeFailure::ConflictingScope => ReferenceScaffoldError::ConflictingScope,
    }
}

fn map_reference_scope_error_for_map(error: ReferenceScopeFailure) -> ReferenceMapError {
    match error {
        ReferenceScopeFailure::Repo(source) => ReferenceMapError::Repo(source),
        ReferenceScopeFailure::SharedRequiresSeries => ReferenceMapError::SharedRequiresSeries,
        ReferenceScopeFailure::ConflictingScope => ReferenceMapError::ConflictingScope,
    }
}

fn map_reference_scope_error_for_check(error: ReferenceScopeFailure) -> ReferenceCheckError {
    match error {
        ReferenceScopeFailure::Repo(source) => ReferenceCheckError::Repo(source),
        ReferenceScopeFailure::SharedRequiresSeries => ReferenceCheckError::SharedRequiresSeries,
        ReferenceScopeFailure::ConflictingScope => ReferenceCheckError::ConflictingScope,
    }
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn scope_summary_label(scope: &ReferenceScope) -> String {
    match scope {
        ReferenceScope::SingleBook => "default".to_string(),
        ReferenceScope::SeriesBook { book_id } => book_id.clone(),
        ReferenceScope::SharedSeries => "shared".to_string(),
    }
}

impl ReferenceEntryScope {
    fn label(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Book => "book",
        }
    }
}

impl ReferenceGapStatus {
    fn applies_to(self, direction: ReferenceSyncDirection) -> bool {
        matches!(
            (self, direction),
            (Self::SharedOnly, ReferenceSyncDirection::FromShared)
                | (Self::BookOnly, ReferenceSyncDirection::ToShared)
        )
    }
}

fn relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn relative_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
