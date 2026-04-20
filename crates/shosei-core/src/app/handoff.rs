use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config, editorial,
    fs::join_repo_path,
    repo::{self, RepoError},
    toolchain,
};

use super::{
    BuildBookError, ValidateBookError, build_book::build_book_with_toolchain,
    validate_book::validate_book_with_toolchain,
};

#[derive(Debug, Clone)]
pub struct HandoffResult {
    pub summary: String,
    pub package_dir: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum HandoffError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Editorial(#[from] editorial::EditorialError),
    #[error(transparent)]
    Build(#[from] BuildBookError),
    #[error(transparent)]
    Validate(#[from] ValidateBookError),
    #[error("unsupported handoff destination `{destination}`")]
    UnsupportedDestination { destination: String },
    #[error("handoff `{destination}` has no matching built artifact for {book_id}")]
    NoArtifactsForDestination {
        destination: String,
        book_id: String,
    },
    #[error("failed to prepare handoff package at {path}: {source}")]
    PreparePackage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to copy handoff file from {from} to {to}: {source}")]
    CopyFile {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write handoff manifest to {path}: {source}")]
    WriteManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize handoff manifest for {path}: {source}")]
    SerializeManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write review notes to {path}: {source}")]
    WriteReviewNotes {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write review packet to {path}: {source}")]
    WriteReviewPacket {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize review packet for {path}: {source}")]
    SerializeReviewPacket {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Serialize)]
struct HandoffManifest {
    book_id: String,
    destination: String,
    created_at_unix_seconds: u64,
    build_summary: String,
    build_stages: Vec<String>,
    build_inputs: Vec<String>,
    validation_summary: String,
    validation_issue_count: usize,
    validation_has_errors: bool,
    selected_artifacts: Vec<String>,
    selected_artifact_details: Vec<HandoffArtifactDetail>,
    validation_report: String,
    cover_ebook_image: Option<String>,
    editorial_files: Vec<String>,
    review_notes: Option<String>,
    review_packet: Option<String>,
    editorial_summary: Option<EditorialSummary>,
    git_commit: Option<String>,
    git_dirty: Option<bool>,
    dirty_worktree_warning: bool,
}

#[derive(Debug, Clone, Serialize)]
struct HandoffArtifactDetail {
    channel: String,
    target: String,
    path: String,
    primary_tool: String,
    target_profile: String,
    artifact_metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
struct EditorialSummary {
    style_rule_count: usize,
    claim_count: usize,
    figure_count: usize,
    freshness_item_count: usize,
    reviewer_note_count: usize,
    overdue_freshness_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPacket {
    book_id: String,
    issue_summary: ReviewIssueSummary,
    issues: Vec<crate::diagnostics::ValidationIssue>,
    reviewer_notes: Vec<String>,
    editorial_summary: Option<EditorialSummary>,
    claims: Vec<ReviewPacketClaim>,
    figures: Vec<ReviewPacketFigure>,
    freshness: Vec<ReviewPacketFreshness>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewIssueSummary {
    total: usize,
    warnings: usize,
    errors: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPacketClaim {
    id: String,
    summary: String,
    section: String,
    sources: Vec<String>,
    reviewer_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPacketFigure {
    id: String,
    path: String,
    caption: String,
    source: Option<String>,
    rights: Option<String>,
    reviewer_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPacketFreshness {
    kind: String,
    id: String,
    last_verified: String,
    review_due_on: String,
    overdue: bool,
    note: Option<String>,
}

pub fn handoff(command: &CommandContext, destination: &str) -> Result<HandoffResult, HandoffError> {
    let toolchain = toolchain::inspect_default_toolchain();
    handoff_with_toolchain(command, destination, &toolchain)
}

fn handoff_with_toolchain(
    command: &CommandContext,
    destination: &str,
    toolchain: &toolchain::ToolchainReport,
) -> Result<HandoffResult, HandoffError> {
    if !matches!(destination, "kindle" | "print" | "proof") {
        return Err(HandoffError::UnsupportedDestination {
            destination: destination.to_string(),
        });
    }

    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    let editorial = if resolved.effective.project.project_type.is_prose() {
        Some(editorial::load_bundle(&resolved)?)
    } else {
        None
    };
    let book = context
        .book
        .as_ref()
        .expect("selected book must exist for handoff");

    let build_result = build_book_with_toolchain(command, toolchain)?;
    let validate_result = validate_book_with_toolchain(command, toolchain)?;

    let selected_outputs = build_result
        .plan
        .outputs
        .iter()
        .filter(|output| match destination {
            "kindle" => output.channel == "kindle",
            "print" => output.channel == "print",
            "proof" => true,
            _ => false,
        })
        .collect::<Vec<_>>();
    if selected_outputs.is_empty() {
        return Err(HandoffError::NoArtifactsForDestination {
            destination: destination.to_string(),
            book_id: book.id.clone(),
        });
    }

    let package_dir = handoff_dir(&resolved.repo.repo_root, &book.id, destination);
    prepare_package_dir(&package_dir)?;

    let artifacts_dir = package_dir.join("artifacts");
    fs::create_dir_all(&artifacts_dir).map_err(|source| HandoffError::PreparePackage {
        path: artifacts_dir.clone(),
        source,
    })?;
    let copied_artifacts = selected_outputs
        .iter()
        .map(|output| copy_into_dir(&output.artifact_path, &artifacts_dir))
        .collect::<Result<Vec<_>, _>>()?;

    let reports_dir = package_dir.join("reports");
    fs::create_dir_all(&reports_dir).map_err(|source| HandoffError::PreparePackage {
        path: reports_dir.clone(),
        source,
    })?;
    let copied_report = copy_with_name(
        &validate_result.report_path,
        &reports_dir.join("validate.json"),
    )?;

    let copied_cover = resolved
        .effective
        .cover
        .ebook_image
        .as_ref()
        .map(|cover_path| {
            copy_with_name(
                &join_repo_path(&resolved.repo.repo_root, cover_path),
                &package_dir.join("assets").join("cover").join(
                    Path::new(cover_path.as_str())
                        .file_name()
                        .unwrap_or_default(),
                ),
            )
        })
        .transpose()?;
    let copied_editorial_files = if destination == "proof" {
        editorial::configured_files(&resolved)
            .into_iter()
            .map(|(repo_path, fs_path)| {
                copy_with_name(&fs_path, &package_dir.join(repo_path.as_str()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let review_notes_path = if destination == "proof" {
        Some(write_review_notes(
            &package_dir.join("review-notes.md"),
            &book.id,
            &validate_result.issues,
            editorial.as_ref(),
        )?)
    } else {
        None
    };
    let review_packet_path = if destination == "proof" {
        Some(write_review_packet(
            &reports_dir.join("review-packet.json"),
            &book.id,
            &validate_result.issues,
            editorial.as_ref(),
        )?)
    } else {
        None
    };
    let editorial_summary = editorial.as_ref().map(build_editorial_summary);

    let git_commit = git_head(&resolved.repo.repo_root);
    let git_dirty = git_is_dirty(&resolved.repo.repo_root);
    let dirty_worktree_warning =
        resolved.effective.git.require_clean_worktree_for_handoff && git_dirty == Some(true);

    let manifest = HandoffManifest {
        book_id: book.id.clone(),
        destination: destination.to_string(),
        created_at_unix_seconds: now_unix_seconds(),
        build_summary: build_result.summary.clone(),
        build_stages: build_result
            .plan
            .stages
            .iter()
            .map(|stage| (*stage).to_string())
            .collect(),
        build_inputs: build_result
            .plan
            .manuscript_files
            .iter()
            .map(|path| relative_to(&resolved.repo.repo_root, path))
            .collect(),
        validation_summary: validate_result.summary.clone(),
        validation_issue_count: validate_result.issue_count,
        validation_has_errors: validate_result.has_errors,
        selected_artifacts: copied_artifacts
            .iter()
            .map(|path| relative_to(&package_dir, path))
            .collect(),
        selected_artifact_details: selected_outputs
            .iter()
            .zip(copied_artifacts.iter())
            .map(|(output, copied_path)| HandoffArtifactDetail {
                channel: output.channel.to_string(),
                target: output.target.clone(),
                path: relative_to(&package_dir, copied_path),
                primary_tool: output.primary_tool.to_string(),
                target_profile: resolved.effective.book.profile.clone(),
                artifact_metadata: build_result
                    .artifact_metadata(output.channel, &output.target)
                    .cloned()
                    .unwrap_or(Value::Null),
            })
            .collect(),
        validation_report: relative_to(&package_dir, &copied_report),
        cover_ebook_image: copied_cover
            .as_ref()
            .map(|path| relative_to(&package_dir, path)),
        editorial_files: copied_editorial_files
            .iter()
            .map(|path| relative_to(&package_dir, path))
            .collect(),
        review_notes: review_notes_path
            .as_ref()
            .map(|path| relative_to(&package_dir, path)),
        review_packet: review_packet_path
            .as_ref()
            .map(|path| relative_to(&package_dir, path)),
        editorial_summary,
        git_commit,
        git_dirty,
        dirty_worktree_warning,
    };
    let manifest_path = package_dir.join("manifest.json");
    write_manifest(&manifest_path, &manifest)?;

    let mut summary = format!(
        "handoff packaged for {} ({}) at {}, artifacts: {}, validation issues: {}, manifest: {}",
        book.id,
        destination,
        package_dir.display(),
        manifest.selected_artifacts.join(", "),
        validate_result.issue_count,
        manifest_path.display()
    );
    if let Some(commit) = &manifest.git_commit {
        summary.push_str(&format!(", commit: {commit}"));
    } else {
        summary.push_str(", commit: unknown");
    }
    if dirty_worktree_warning {
        summary.push_str(", warning: git worktree is dirty");
    }

    Ok(HandoffResult {
        summary,
        package_dir,
        manifest_path,
    })
}

fn write_review_notes(
    path: &Path,
    book_id: &str,
    issues: &[crate::diagnostics::ValidationIssue],
    editorial: Option<&editorial::EditorialBundle>,
) -> Result<PathBuf, HandoffError> {
    let mut lines = vec![
        format!("# Review Notes: {book_id}"),
        String::new(),
        format!("- open issues: {}", issues.len()),
    ];

    if let Some(editorial) = editorial {
        lines.push(format!("- style rules: {}", editorial.style_rule_count()));
        lines.push(format!("- claims: {}", editorial.claim_count()));
        lines.push(format!("- figures: {}", editorial.figure_count()));
        lines.push(format!(
            "- freshness items: {}",
            editorial.freshness_count()
        ));
    }

    lines.push(String::new());
    lines.push("## Validation Issues".to_string());
    if issues.is_empty() {
        lines.push("- none".to_string());
    } else {
        for issue in issues {
            let location = issue
                .location
                .as_ref()
                .map(|location| format!(" ({location})"))
                .unwrap_or_default();
            lines.push(format!(
                "- [{}] {}{}",
                match issue.severity {
                    crate::diagnostics::Severity::Warning => "warn",
                    crate::diagnostics::Severity::Error => "error",
                },
                issue.cause,
                location
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Reviewer Notes".to_string());
    let reviewer_notes = editorial
        .map(|bundle| bundle.reviewer_notes())
        .unwrap_or_default();
    if reviewer_notes.is_empty() {
        lines.push("- none".to_string());
    } else {
        for note in reviewer_notes {
            lines.push(format!("- {note}"));
        }
    }

    if let Some(editorial) = editorial {
        lines.push(String::new());
        lines.push("## Claims".to_string());
        if let Some(claims) = &editorial.claims {
            if claims.data.claims.is_empty() {
                lines.push("- none".to_string());
            } else {
                for claim in &claims.data.claims {
                    lines.push(format!("- {}: {}", claim.id, claim.summary));
                }
            }
        } else {
            lines.push("- none".to_string());
        }

        lines.push(String::new());
        lines.push("## Figures".to_string());
        if let Some(figures) = &editorial.figures {
            if figures.data.figures.is_empty() {
                lines.push("- none".to_string());
            } else {
                for figure in &figures.data.figures {
                    let rights = figure.rights.as_deref().unwrap_or("unspecified");
                    lines.push(format!("- {}: {} [{}]", figure.id, figure.caption, rights));
                }
            }
        } else {
            lines.push("- none".to_string());
        }
    }

    fs::write(path, lines.join("\n")).map_err(|source| HandoffError::WriteReviewNotes {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(path.to_path_buf())
}

fn write_review_packet(
    path: &Path,
    book_id: &str,
    issues: &[crate::diagnostics::ValidationIssue],
    editorial: Option<&editorial::EditorialBundle>,
) -> Result<PathBuf, HandoffError> {
    let packet = build_review_packet(book_id, issues, editorial);
    let contents = serde_json::to_string_pretty(&packet).map_err(|source| {
        HandoffError::SerializeReviewPacket {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, contents).map_err(|source| HandoffError::WriteReviewPacket {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(path.to_path_buf())
}

fn build_review_packet(
    book_id: &str,
    issues: &[crate::diagnostics::ValidationIssue],
    editorial: Option<&editorial::EditorialBundle>,
) -> ReviewPacket {
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == crate::diagnostics::Severity::Warning)
        .count();
    let errors = issues.len().saturating_sub(warnings);

    let reviewer_notes = editorial
        .map(|bundle| bundle.reviewer_notes())
        .unwrap_or_default();
    let claims = editorial
        .and_then(|bundle| bundle.claims.as_ref())
        .map(|claims| {
            claims
                .data
                .claims
                .iter()
                .map(|claim| ReviewPacketClaim {
                    id: claim.id.clone(),
                    summary: claim.summary.clone(),
                    section: claim.section.clone(),
                    sources: claim.sources.clone(),
                    reviewer_note: claim.reviewer_note.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let figures = editorial
        .and_then(|bundle| bundle.figures.as_ref())
        .map(|figures| {
            figures
                .data
                .figures
                .iter()
                .map(|figure| ReviewPacketFigure {
                    id: figure.id.clone(),
                    path: figure.path.clone(),
                    caption: figure.caption.clone(),
                    source: figure.source.clone(),
                    rights: figure.rights.clone(),
                    reviewer_note: figure.reviewer_note.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let freshness = editorial
        .and_then(|bundle| bundle.freshness.as_ref())
        .map(|freshness| {
            let today = editorial::today_local();
            freshness
                .data
                .tracked
                .iter()
                .map(|item| ReviewPacketFreshness {
                    kind: item.kind.as_str().to_string(),
                    id: item.id.clone(),
                    last_verified: item.last_verified.clone(),
                    review_due_on: item.review_due_on.clone(),
                    overdue: editorial::parse_iso_date(&item.review_due_on)
                        .map(|date| date < today)
                        .unwrap_or(false),
                    note: item.note.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    ReviewPacket {
        book_id: book_id.to_string(),
        issue_summary: ReviewIssueSummary {
            total: issues.len(),
            warnings,
            errors,
        },
        issues: issues.to_vec(),
        reviewer_notes,
        editorial_summary: editorial.map(build_editorial_summary),
        claims,
        figures,
        freshness,
    }
}

fn build_editorial_summary(editorial: &editorial::EditorialBundle) -> EditorialSummary {
    let overdue_freshness_count = editorial
        .freshness
        .as_ref()
        .map(|freshness| {
            let today = editorial::today_local();
            freshness
                .data
                .tracked
                .iter()
                .filter(|item| {
                    editorial::parse_iso_date(&item.review_due_on)
                        .map(|date| date < today)
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);

    EditorialSummary {
        style_rule_count: editorial.style_rule_count(),
        claim_count: editorial.claim_count(),
        figure_count: editorial.figure_count(),
        freshness_item_count: editorial.freshness_count(),
        reviewer_note_count: editorial.reviewer_notes().len(),
        overdue_freshness_count,
    }
}

fn handoff_dir(repo_root: &Path, book_id: &str, destination: &str) -> PathBuf {
    repo_root
        .join("dist")
        .join("handoff")
        .join(format!("{book_id}-{destination}"))
}

fn prepare_package_dir(path: &Path) -> Result<(), HandoffError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|source| HandoffError::PreparePackage {
            path: path.to_path_buf(),
            source,
        })?;
    }
    fs::create_dir_all(path).map_err(|source| HandoffError::PreparePackage {
        path: path.to_path_buf(),
        source,
    })
}

fn copy_into_dir(from: &Path, dir: &Path) -> Result<PathBuf, HandoffError> {
    let target = dir.join(from.file_name().unwrap_or_default());
    copy_with_name(from, &target)
}

fn copy_with_name(from: &Path, to: &Path) -> Result<PathBuf, HandoffError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|source| HandoffError::PreparePackage {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::copy(from, to).map_err(|source| HandoffError::CopyFile {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })?;
    Ok(to.to_path_buf())
}

fn write_manifest(path: &Path, manifest: &HandoffManifest) -> Result<(), HandoffError> {
    let contents = serde_json::to_string_pretty(manifest).map_err(|source| {
        HandoffError::SerializeManifest {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, contents).map_err(|source| HandoffError::WriteManifest {
        path: path.to_path_buf(),
        source,
    })
}

fn relative_to(base: &Path, path: &Path) -> String {
    path.strip_prefix(base).unwrap_or(path).components().fold(
        String::new(),
        |mut relative, component| {
            match component {
                Component::Prefix(prefix) => {
                    relative.push_str(&prefix.as_os_str().to_string_lossy());
                }
                Component::RootDir => relative.push('/'),
                Component::CurDir => relative.push('.'),
                Component::ParentDir => {
                    if !relative.is_empty() && !relative.ends_with('/') {
                        relative.push('/');
                    }
                    relative.push_str("..");
                }
                Component::Normal(part) => {
                    if !relative.is_empty() && !relative.ends_with('/') {
                        relative.push('/');
                    }
                    relative.push_str(&part.to_string_lossy());
                }
            }
            relative
        },
    )
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn git_head(repo_root: &Path) -> Option<String> {
    git_output(repo_root, &["rev-parse", "HEAD"])
}

fn git_is_dirty(repo_root: &Path) -> Option<bool> {
    git_output(repo_root, &["status", "--porcelain"]).map(|output| !output.is_empty())
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(stdout)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use crate::{
        cli_api::CommandContext,
        diagnostics::{IssueLocation, Severity, ValidationIssue},
        editorial::{
            ClaimLedger, ClaimRecord, EditorialBundle, FigureLedger, FigureRecord, FreshnessKind,
            FreshnessLedger, FreshnessRecord, LoadedClaimLedger, LoadedFigureLedger,
            LoadedFreshnessLedger,
        },
        toolchain::{ToolRecord, ToolStatus, ToolchainReport},
    };

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-handoff-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tiny_png() -> &'static [u8] {
        &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ]
    }

    fn write_manga_book(root: &Path, output_block: &str, with_cover: bool) {
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(root.join("manga/pages/001.png"), tiny_png()).unwrap();
        if with_cover {
            fs::create_dir_all(root.join("assets/cover")).unwrap();
            fs::write(root.join("assets/cover/front.png"), tiny_png()).unwrap();
        }
        let cover_block = if with_cover {
            "cover:\n  ebook_image: assets/cover/front.png\n"
        } else {
            ""
        };
        fs::write(
            root.join("book.yml"),
            format!(
                "project:\n  type: manga\n  vcs: git\nbook:\n  title: \"Sample Manga\"\n  authors:\n    - \"Author\"\n  reading_direction: rtl\nlayout:\n  binding: right\n{cover_block}{output_block}validation:\n  strict: true\n  missing_image: error\ngit:\n  lfs: true\nmanga:\n  reading_direction: rtl\n  default_page_side: right\n  spread_policy_for_kindle: split\n  front_color_pages: 0\n  body_mode: monochrome\n"
            ),
        )
        .unwrap();
    }

    fn fake_toolchain(pandoc_path: Option<PathBuf>) -> ToolchainReport {
        ToolchainReport {
            tools: vec![
                ToolRecord {
                    key: "pandoc",
                    display_name: "pandoc",
                    status: if pandoc_path.is_some() {
                        ToolStatus::Available
                    } else {
                        ToolStatus::Missing
                    },
                    detected_as: Some("pandoc".to_string()),
                    resolved_path: pandoc_path,
                    version: None,
                    install_hint: "Install pandoc and ensure it is available on PATH.".to_string(),
                },
                ToolRecord {
                    key: "epubcheck",
                    display_name: "epubcheck",
                    status: ToolStatus::Missing,
                    detected_as: Some("epubcheck".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install epubcheck and ensure the launcher is available on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "weasyprint",
                    display_name: "weasyprint",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint: "Install weasyprint and ensure the launcher is on PATH."
                        .to_string(),
                },
                ToolRecord {
                    key: "chromium",
                    display_name: "Chromium PDF",
                    status: ToolStatus::Available,
                    detected_as: Some("chromium".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install a Chromium-based browser and ensure its executable is available."
                            .to_string(),
                },
                ToolRecord {
                    key: "pdf-engine",
                    display_name: "PDF engine",
                    status: ToolStatus::Available,
                    detected_as: Some("weasyprint".to_string()),
                    resolved_path: None,
                    version: None,
                    install_hint:
                        "Install one supported PDF engine such as weasyprint, Chromium, typst, or lualatex."
                            .to_string(),
                },
            ],
        }
    }

    fn write_prose_book_with_editorial(root: &Path) {
        fs::create_dir_all(root.join("manuscript")).unwrap();
        fs::create_dir_all(root.join("editorial")).unwrap();
        fs::create_dir_all(root.join("assets/images")).unwrap();
        fs::write(
            root.join("manuscript/01.md"),
            "# Chapter 1\nUse git in the workflow.\n![Figure](../assets/images/example.png)\n",
        )
        .unwrap();
        fs::write(root.join("assets/images/example.png"), tiny_png()).unwrap();
        fs::write(
            root.join("editorial/style.yml"),
            r#"
preferred_terms:
  - preferred: "Git"
    aliases:
      - "git"
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/claims.yml"),
            r#"
claims:
  - id: claim-1
    summary: "Summary"
    section: manuscript/01.md
    sources:
      - https://example.com/source
    reviewer_note: "double-check the source"
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/figures.yml"),
            r#"
figures:
  - id: fig-1
    path: assets/images/example.png
    caption: "Example"
    source: "Internal"
    rights: "owned"
    reviewer_note: "replace logo"
"#,
        )
        .unwrap();
        fs::write(
            root.join("editorial/freshness.yml"),
            r#"
tracked:
  - kind: claim
    id: claim-1
    last_verified: 1999-01-01
    review_due_on: 2000-01-01
    note: "refresh before launch"
"#,
        )
        .unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: business
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
  reading_direction: ltr
layout:
  binding: left
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
  print:
    enabled: true
    target: print-jp-pdfx1a
validation:
  strict: true
  epubcheck: false
  accessibility: warn
git:
  lfs: true
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
"#,
        )
        .unwrap();
    }

    #[test]
    fn handoff_packages_kindle_artifact_and_manifest() {
        let root = temp_dir("kindle");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            true,
        );

        let result = handoff(&CommandContext::new(&root, None, None), "kindle").unwrap();

        assert!(result.package_dir.is_dir());
        assert!(result.manifest_path.is_file());
        assert!(
            result
                .package_dir
                .join("artifacts/default-kindle-comic.epub")
                .is_file()
        );
        assert!(result.package_dir.join("reports/validate.json").is_file());
        assert!(result.package_dir.join("assets/cover/front.png").is_file());

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(result.manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["destination"], "kindle");
        assert_eq!(manifest["cover_ebook_image"], "assets/cover/front.png");
        assert_eq!(manifest["build_inputs"][0], "manga/pages/001.png");
        assert_eq!(manifest["build_stages"][0], "resolve-config");
        assert!(
            manifest["selected_artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "artifacts/default-kindle-comic.epub")
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["channel"],
            "kindle"
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["primary_tool"],
            "shosei-fxl-epub"
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["target_profile"],
            "manga"
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["artifact_metadata"]["kindle"]["fixed_layout"],
            true
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["artifact_metadata"]["manga"]["source_page_count"],
            1
        );
    }

    #[test]
    fn handoff_packages_proof_with_all_artifacts() {
        let root = temp_dir("proof");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n  print:\n    enabled: true\n    target: print-manga\n",
            false,
        );

        let result = handoff(&CommandContext::new(&root, None, None), "proof").unwrap();
        assert!(result.package_dir.join("review-notes.md").is_file());
        assert!(
            result
                .package_dir
                .join("reports/review-packet.json")
                .is_file()
        );
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(result.manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["destination"], "proof");
        assert_eq!(manifest["selected_artifacts"].as_array().unwrap().len(), 2);
        assert_eq!(
            manifest["selected_artifact_details"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(manifest["review_notes"], "review-notes.md");
        assert_eq!(manifest["review_packet"], "reports/review-packet.json");
        assert!(manifest["editorial_summary"].is_null());
    }

    #[test]
    fn handoff_rejects_unknown_destination() {
        let root = temp_dir("unknown-destination");
        write_manga_book(
            &root,
            "outputs:\n  kindle:\n    enabled: true\n    target: kindle-comic\n",
            false,
        );

        let error = handoff(&CommandContext::new(&root, None, None), "web").unwrap_err();
        assert!(matches!(
            error,
            HandoffError::UnsupportedDestination { destination } if destination == "web"
        ));
    }

    #[test]
    fn review_notes_include_editorial_claims_figures_and_issues() {
        let root = temp_dir("review-notes");
        let path = root.join("review-notes.md");
        let issues = vec![ValidationIssue {
            severity: Severity::Warning,
            target: "common".to_string(),
            location: Some(IssueLocation::with_line(
                PathBuf::from("manuscript/01.md"),
                2,
            )),
            cause: "preferred term `Git` should replace `git`".to_string(),
            remedy: "fix it".to_string(),
        }];
        let editorial = EditorialBundle {
            style: None,
            claims: Some(LoadedClaimLedger {
                path: root.join("editorial/claims.yml"),
                data: ClaimLedger {
                    claims: vec![ClaimRecord {
                        id: "claim-1".to_string(),
                        summary: "Summary".to_string(),
                        section: "manuscript/01.md".to_string(),
                        sources: vec!["https://example.com".to_string()],
                        reviewer_note: Some("double-check the source".to_string()),
                    }],
                },
            }),
            figures: Some(LoadedFigureLedger {
                path: root.join("editorial/figures.yml"),
                data: FigureLedger {
                    figures: vec![FigureRecord {
                        id: "fig-1".to_string(),
                        path: "assets/images/example.png".to_string(),
                        caption: "Architecture".to_string(),
                        source: Some("Internal".to_string()),
                        rights: Some("owned".to_string()),
                        reviewer_note: Some("replace logo".to_string()),
                    }],
                },
            }),
            freshness: Some(LoadedFreshnessLedger {
                path: root.join("editorial/freshness.yml"),
                data: FreshnessLedger {
                    tracked: vec![FreshnessRecord {
                        kind: FreshnessKind::Claim,
                        id: "claim-1".to_string(),
                        last_verified: "2026-04-13".to_string(),
                        review_due_on: "2026-05-13".to_string(),
                        note: Some("refresh before launch".to_string()),
                    }],
                },
            }),
        };

        write_review_notes(&path, "default", &issues, Some(&editorial)).unwrap();

        let contents = fs::read_to_string(path).unwrap();
        assert!(
            contents.contains("preferred term `Git` should replace `git` (manuscript/01.md:2)")
        );
        assert!(contents.contains("claim-1: Summary"));
        assert!(contents.contains("fig-1: Architecture [owned]"));
        assert!(contents.contains("double-check the source"));
        assert!(contents.contains("refresh before launch"));
    }

    #[test]
    fn review_packet_includes_structured_editorial_summary() {
        let root = temp_dir("review-packet");
        let path = root.join("reports/review-packet.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let issues = vec![ValidationIssue {
            severity: Severity::Warning,
            target: "common".to_string(),
            location: Some(IssueLocation::with_line(
                PathBuf::from("manuscript/01.md"),
                2,
            )),
            cause: "preferred term `Git` should replace `git`".to_string(),
            remedy: "fix it".to_string(),
        }];
        let editorial = EditorialBundle {
            style: None,
            claims: Some(LoadedClaimLedger {
                path: root.join("editorial/claims.yml"),
                data: ClaimLedger {
                    claims: vec![ClaimRecord {
                        id: "claim-1".to_string(),
                        summary: "Summary".to_string(),
                        section: "manuscript/01.md".to_string(),
                        sources: vec!["https://example.com".to_string()],
                        reviewer_note: Some("double-check the source".to_string()),
                    }],
                },
            }),
            figures: Some(LoadedFigureLedger {
                path: root.join("editorial/figures.yml"),
                data: FigureLedger {
                    figures: vec![FigureRecord {
                        id: "fig-1".to_string(),
                        path: "assets/images/example.png".to_string(),
                        caption: "Architecture".to_string(),
                        source: Some("Internal".to_string()),
                        rights: Some("owned".to_string()),
                        reviewer_note: Some("replace logo".to_string()),
                    }],
                },
            }),
            freshness: Some(LoadedFreshnessLedger {
                path: root.join("editorial/freshness.yml"),
                data: FreshnessLedger {
                    tracked: vec![FreshnessRecord {
                        kind: FreshnessKind::Claim,
                        id: "claim-1".to_string(),
                        last_verified: "1999-01-01".to_string(),
                        review_due_on: "2000-01-01".to_string(),
                        note: Some("refresh before launch".to_string()),
                    }],
                },
            }),
        };

        write_review_packet(&path, "default", &issues, Some(&editorial)).unwrap();

        let packet: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(packet["book_id"], "default");
        assert_eq!(packet["issue_summary"]["total"], 1);
        assert_eq!(packet["issue_summary"]["warnings"], 1);
        assert_eq!(packet["issue_summary"]["errors"], 0);
        assert_eq!(packet["editorial_summary"]["claim_count"], 1);
        assert_eq!(packet["editorial_summary"]["figure_count"], 1);
        assert_eq!(packet["editorial_summary"]["freshness_item_count"], 1);
        assert_eq!(packet["editorial_summary"]["reviewer_note_count"], 3);
        assert_eq!(packet["editorial_summary"]["overdue_freshness_count"], 1);
        assert_eq!(packet["claims"][0]["section"], "manuscript/01.md");
        assert_eq!(packet["claims"][0]["sources"][0], "https://example.com");
        assert_eq!(packet["figures"][0]["rights"], "owned");
        assert_eq!(packet["freshness"][0]["kind"], "claim");
        assert_eq!(packet["freshness"][0]["overdue"], true);
        assert!(
            packet["reviewer_notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "claim claim-1: double-check the source")
        );
    }

    #[test]
    fn handoff_proof_packages_editorial_review_packet_for_prose_books() {
        if !cfg!(unix) {
            return;
        }

        let root = temp_dir("proof-prose");
        write_prose_book_with_editorial(&root);
        let pandoc = root.join("pandoc");
        fs::write(
            &pandoc,
            r#"#!/bin/sh
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
mkdir -p "$(dirname "$out")"
printf 'fake output' > "$out"
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = handoff_with_toolchain(
            &CommandContext::new(&root, None, None),
            "proof",
            &fake_toolchain(Some(pandoc)),
        )
        .unwrap();

        assert!(result.package_dir.join("review-notes.md").is_file());
        assert!(
            result
                .package_dir
                .join("reports/review-packet.json")
                .is_file()
        );
        assert!(result.package_dir.join("editorial/style.yml").is_file());
        assert!(result.package_dir.join("editorial/claims.yml").is_file());
        assert!(result.package_dir.join("editorial/figures.yml").is_file());
        assert!(result.package_dir.join("editorial/freshness.yml").is_file());

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(result.manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["review_notes"], "review-notes.md");
        assert_eq!(manifest["review_packet"], "reports/review-packet.json");
        assert_eq!(manifest["editorial_files"].as_array().unwrap().len(), 4);
        assert_eq!(manifest["build_inputs"][0], "manuscript/01.md");
        assert_eq!(
            manifest["selected_artifact_details"][0]["path"],
            "artifacts/default-kindle-ja.epub"
        );
        assert_eq!(
            manifest["selected_artifact_details"][0]["artifact_metadata"]["kindle"]["fixed_layout"],
            false
        );
        assert_eq!(
            manifest["selected_artifact_details"][1]["target_profile"],
            "business"
        );
        assert_eq!(
            manifest["selected_artifact_details"][1]["artifact_metadata"]["print"]["pdf_engine"],
            "weasyprint"
        );
        assert_eq!(manifest["editorial_summary"]["claim_count"], 1);
        assert_eq!(manifest["editorial_summary"]["figure_count"], 1);
        assert_eq!(manifest["editorial_summary"]["freshness_item_count"], 1);
        assert_eq!(manifest["editorial_summary"]["reviewer_note_count"], 3);

        let review_notes = fs::read_to_string(result.package_dir.join("review-notes.md")).unwrap();
        assert!(review_notes.contains("double-check the source"));
        assert!(review_notes.contains("fig-1: Example [owned]"));
        assert!(review_notes.contains("refresh before launch"));
        assert!(review_notes.contains("manuscript/01.md:2"));

        let packet: Value = serde_json::from_str(
            &fs::read_to_string(result.package_dir.join("reports/review-packet.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(packet["book_id"], "default");
        assert_eq!(packet["claims"][0]["id"], "claim-1");
        assert_eq!(packet["figures"][0]["id"], "fig-1");
        assert_eq!(packet["freshness"][0]["overdue"], true);
    }
}
