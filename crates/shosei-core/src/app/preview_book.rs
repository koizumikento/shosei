use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    app::{BuildBookError, build_book},
    cli_api::CommandContext,
    repo::{self, RepoError},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PreviewBookResult {
    pub summary: String,
    pub artifacts: Vec<std::path::PathBuf>,
}

#[derive(Debug, Error)]
pub enum PreviewBookError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Build(#[from] BuildBookError),
    #[error("failed to inspect preview watch paths under {path}: {source}")]
    WatchSnapshot {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn preview_book(command: &CommandContext) -> Result<PreviewBookResult, PreviewBookError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    let book = context.book.expect("selected book must exist");
    let build = build_book(command)?;
    let preview_target = build
        .plan
        .outputs
        .iter()
        .find(|output| output.channel == "print")
        .or_else(|| build.plan.outputs.first())
        .map(|output| output.target.as_str())
        .unwrap_or("none");
    let artifacts = build.artifacts.clone();
    Ok(PreviewBookResult {
        summary: format!(
            "preview ready for {} using target {}: {}",
            book.id,
            preview_target,
            artifacts
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        artifacts,
    })
}

pub fn watch_preview(
    command: &CommandContext,
    mut emit: impl FnMut(&str),
) -> Result<(), PreviewBookError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let book = context.book.expect("selected book must exist");
    let watch_roots = watch_roots(&context.repo_root, &book.root);
    let mut snapshot = build_watch_snapshot(&watch_roots)?;

    let initial = preview_book(command)?;
    emit(&initial.summary);
    emit(&format!(
        "watching preview for {} under {} path(s); press Ctrl-C to stop",
        book.id,
        watch_roots.len()
    ));

    loop {
        thread::sleep(Duration::from_secs(1));
        let next = build_watch_snapshot(&watch_roots)?;
        if next == snapshot {
            continue;
        }
        snapshot = next;
        match preview_book(command) {
            Ok(result) => emit(&format!("rebuild: {}", result.summary)),
            Err(error) => emit(&format!("rebuild failed: {error}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    is_dir: bool,
    len: u64,
    modified_millis: u128,
}

fn watch_roots(repo_root: &Path, book_root: &Path) -> Vec<PathBuf> {
    let candidates = [
        repo_root.join("book.yml"),
        repo_root.join("series.yml"),
        repo_root.join("shared"),
        book_root.join("book.yml"),
        book_root.join("manuscript"),
        book_root.join("manga"),
        book_root.join("assets"),
        book_root.join("styles"),
    ];
    let mut roots = Vec::new();
    for candidate in candidates {
        if !roots.iter().any(|existing| existing == &candidate) && candidate.exists() {
            roots.push(candidate);
        }
    }
    roots
}

fn build_watch_snapshot(
    roots: &[PathBuf],
) -> Result<BTreeMap<PathBuf, FileStamp>, PreviewBookError> {
    let mut snapshot = BTreeMap::new();
    for root in roots {
        collect_watch_snapshot(root, &mut snapshot)?;
    }
    Ok(snapshot)
}

fn collect_watch_snapshot(
    path: &Path,
    snapshot: &mut BTreeMap<PathBuf, FileStamp>,
) -> Result<(), PreviewBookError> {
    let metadata = fs::metadata(path).map_err(|source| PreviewBookError::WatchSnapshot {
        path: path.to_path_buf(),
        source,
    })?;
    snapshot.insert(path.to_path_buf(), file_stamp(&metadata));

    if metadata.is_dir() {
        let mut children = fs::read_dir(path)
            .map_err(|source| PreviewBookError::WatchSnapshot {
                path: path.to_path_buf(),
                source,
            })?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect::<Vec<_>>();
        children.sort();
        for child in children {
            collect_watch_snapshot(&child, snapshot)?;
        }
    }

    Ok(())
}

fn file_stamp(metadata: &fs::Metadata) -> FileStamp {
    let modified_millis = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    FileStamp {
        is_dir: metadata.is_dir(),
        len: metadata.len(),
        modified_millis,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, thread, time::Duration};

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use crate::cli_api::CommandContext;

    use super::{build_watch_snapshot, preview_book, watch_roots};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-preview-book-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tiny_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])));
        image
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn preview_builds_manga_artifact_for_selected_target() {
        let root = temp_dir("manga-preview");
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(
            root.join("book.yml"),
            r#"
project:
  type: manga
  vcs: git
book:
  title: "Sample Manga"
  authors:
    - "Author"
  reading_direction: rtl
layout:
  binding: right
outputs:
  kindle:
    enabled: true
    target: kindle-comic
  print:
    enabled: true
    target: print-manga
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 0
  body_mode: mixed
"#,
        )
        .unwrap();
        fs::write(root.join("manga/pages/001.png"), tiny_png()).unwrap();

        let result =
            preview_book(&CommandContext::new(&root, None, Some("print".to_string()))).unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert!(result.artifacts[0].is_file());
        assert!(result.summary.contains("print-manga"));
    }

    #[test]
    fn watch_roots_include_series_shared_paths() {
        let root = temp_dir("watch-roots");
        fs::create_dir_all(root.join("shared/styles")).unwrap();
        fs::create_dir_all(root.join("books/vol-01/manga/pages")).unwrap();
        fs::write(root.join("series.yml"), "series:\n  id: sample\n").unwrap();
        fs::write(
            root.join("books/vol-01/book.yml"),
            "project:\n  type: manga\n",
        )
        .unwrap();

        let roots = watch_roots(&root, &root.join("books/vol-01"));

        assert!(roots.contains(&root.join("series.yml")));
        assert!(roots.contains(&root.join("shared")));
        assert!(roots.contains(&root.join("books/vol-01/manga")));
    }

    #[test]
    fn watch_snapshot_changes_when_file_is_modified() {
        let root = temp_dir("watch-snapshot");
        fs::create_dir_all(root.join("manuscript")).unwrap();
        let path = root.join("manuscript/01.md");
        fs::write(&path, "# Chapter 1\n").unwrap();

        let before = build_watch_snapshot(&[root.join("manuscript")]).unwrap();
        thread::sleep(Duration::from_millis(20));
        fs::write(&path, "# Chapter 1\n\nupdated\n").unwrap();
        let after = build_watch_snapshot(&[root.join("manuscript")]).unwrap();

        assert_ne!(before, after);
    }
}
