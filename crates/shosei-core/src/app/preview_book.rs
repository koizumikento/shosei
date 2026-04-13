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

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use crate::cli_api::CommandContext;

    use super::preview_book;

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
}
