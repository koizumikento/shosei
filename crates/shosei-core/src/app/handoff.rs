use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config,
    fs::join_repo_path,
    repo::{self, RepoError},
};

use super::{BuildBookError, ValidateBookError, build_book, validate_book};

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
}

#[derive(Debug, Clone, Serialize)]
struct HandoffManifest {
    book_id: String,
    destination: String,
    created_at_unix_seconds: u64,
    build_summary: String,
    validation_summary: String,
    validation_issue_count: usize,
    validation_has_errors: bool,
    selected_artifacts: Vec<String>,
    validation_report: String,
    cover_ebook_image: Option<String>,
    git_commit: Option<String>,
    git_dirty: Option<bool>,
    dirty_worktree_warning: bool,
}

pub fn handoff(command: &CommandContext, destination: &str) -> Result<HandoffResult, HandoffError> {
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
    let book = context
        .book
        .as_ref()
        .expect("selected book must exist for handoff");

    let build_result = build_book(command)?;
    let validate_result = validate_book(command)?;

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

    let git_commit = git_head(&resolved.repo.repo_root);
    let git_dirty = git_is_dirty(&resolved.repo.repo_root);
    let dirty_worktree_warning =
        resolved.effective.git.require_clean_worktree_for_handoff && git_dirty == Some(true);

    let manifest = HandoffManifest {
        book_id: book.id.clone(),
        destination: destination.to_string(),
        created_at_unix_seconds: now_unix_seconds(),
        build_summary: build_result.summary.clone(),
        validation_summary: validate_result.summary.clone(),
        validation_issue_count: validate_result.issue_count,
        validation_has_errors: validate_result.has_errors,
        selected_artifacts: copied_artifacts
            .iter()
            .map(|path| relative_to(&package_dir, path))
            .collect(),
        validation_report: relative_to(&package_dir, &copied_report),
        cover_ebook_image: copied_cover
            .as_ref()
            .map(|path| relative_to(&package_dir, path)),
        git_commit,
        git_dirty,
        dirty_worktree_warning,
    };
    let manifest_path = package_dir.join("manifest.json");
    write_manifest(&manifest_path, &manifest)?;

    let mut summary = format!(
        "handoff packaged for {} ({}) at {}, artifacts: {}, validation issues: {}",
        book.id,
        destination,
        package_dir.display(),
        manifest.selected_artifacts.join(", "),
        validate_result.issue_count
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
    path.strip_prefix(base)
        .unwrap_or(path)
        .display()
        .to_string()
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
    use std::fs;

    use serde_json::Value;

    use crate::cli_api::CommandContext;

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
        assert!(
            manifest["selected_artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "artifacts/default-kindle-comic.epub")
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
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(result.manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["destination"], "proof");
        assert_eq!(manifest["selected_artifacts"].as_array().unwrap().len(), 2);
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
}
