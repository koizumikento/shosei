use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    cli_api::CommandContext,
    config,
    diagnostics::{Severity, ValidationIssue},
    domain::ProjectType,
    manga,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct PageCheckResult {
    pub summary: String,
    pub report_path: PathBuf,
    pub issues: Vec<ValidationIssue>,
    pub issue_count: usize,
    pub has_errors: bool,
}

#[derive(Debug, Error)]
pub enum PageCheckError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("page check is only supported for manga projects, got {project_type}")]
    UnsupportedProjectType { project_type: ProjectType },
    #[error("failed to write page check report to {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize page check report for {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn page_check(command: &CommandContext) -> Result<PageCheckResult, PageCheckError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let resolved = config::resolve_book_config(&context)?;
    if resolved.effective.project.project_type != ProjectType::Manga {
        return Err(PageCheckError::UnsupportedProjectType {
            project_type: resolved.effective.project.project_type,
        });
    }

    let book = context.book.expect("selected book must exist");
    let report_path = report_path(&resolved);

    let mut issues = Vec::new();
    let mut page_order = Vec::new();
    let mut spread_candidates = Vec::new();

    let page_files = match manga::discover_page_files(&book.root) {
        Ok(files) => files,
        Err(manga::MangaRenderError::MissingPageDirectory { path }) => {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!("manga page directory not found: {}", path.display()),
                    "manga/pages/ を作成してページ画像を追加してください。",
                )
                .at(path),
            );
            Vec::new()
        }
        Err(manga::MangaRenderError::NoPageImages { path }) => {
            issues.push(
                ValidationIssue::error(
                    "common",
                    format!(
                        "no supported page images were found under {}",
                        path.display()
                    ),
                    "manga/pages/ 直下に PNG または JPEG のページ画像を追加してください。",
                )
                .at(path),
            );
            Vec::new()
        }
        Err(other) => {
            issues.push(ValidationIssue::error(
                "common",
                other.to_string(),
                "manga/pages/ 配下の画像を確認してください。",
            ));
            Vec::new()
        }
    };

    if !page_files.is_empty() {
        page_order = page_files
            .iter()
            .map(|path| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        issues.extend(page_order_issues(&book.root, &page_files));

        match manga::inspect_page_assets(&page_files) {
            Ok(page_assets) => {
                spread_candidates = page_assets
                    .iter()
                    .filter(|page| page.is_wide_spread_candidate())
                    .map(|page| page.file_name.clone())
                    .collect();
                issues.extend(page_size_issues(&book.root, &page_assets));
                issues.extend(kindle_spread_policy_issues(&resolved, &page_assets));
                issues.extend(manga_color_policy_issues(&resolved, &page_assets));
            }
            Err(manga::MangaRenderError::DecodePage { path }) => {
                issues.push(
                    ValidationIssue::error(
                        "common",
                        format!("failed to decode manga page: {}", path.display()),
                        "壊れている画像を置き換えるか、ファイル形式を見直してください。",
                    )
                    .at(path),
                );
            }
            Err(other) => issues.push(ValidationIssue::error(
                "common",
                other.to_string(),
                "manga/pages/ 配下の画像を確認してください。",
            )),
        }
    }

    let report = PageCheckReport {
        book_id: book.id.clone(),
        page_count: page_order.len(),
        page_order,
        spread_candidates,
        issues: issues.clone(),
    };
    write_report(&report_path, &report)?;
    let has_errors = issues.iter().any(|issue| issue.severity == Severity::Error);
    let issue_count = issues.len();

    Ok(PageCheckResult {
        summary: format!(
            "page check completed for {} with {} page(s), issues: {}, report: {}\npage order: {}\nspread candidates: {}",
            book.id,
            report.page_count,
            issue_count,
            report_path.display(),
            summarize_file_list(&report.page_order),
            summarize_file_list(&report.spread_candidates),
        ),
        report_path,
        issues,
        issue_count,
        has_errors,
    })
}

#[derive(Debug, Clone, Serialize)]
struct PageCheckReport {
    book_id: String,
    page_count: usize,
    page_order: Vec<String>,
    spread_candidates: Vec<String>,
    issues: Vec<ValidationIssue>,
}

fn report_path(resolved: &config::ResolvedBookConfig) -> PathBuf {
    let book_id = resolved
        .repo
        .book
        .as_ref()
        .map(|book| book.id.as_str())
        .unwrap_or("default");
    resolved
        .repo
        .repo_root
        .join("dist")
        .join("reports")
        .join(format!("{book_id}-page-check.json"))
}

fn write_report(path: &Path, report: &PageCheckReport) -> Result<(), PageCheckError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| PageCheckError::WriteReport {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents =
        serde_json::to_string_pretty(report).map_err(|source| PageCheckError::SerializeReport {
            path: path.to_path_buf(),
            source,
        })?;
    fs::write(path, contents).map_err(|source| PageCheckError::WriteReport {
        path: path.to_path_buf(),
        source,
    })
}

fn summarize_file_list(items: &[String]) -> String {
    const LIMIT: usize = 10;

    if items.is_empty() {
        return "none".to_string();
    }
    if items.len() <= LIMIT {
        return items.join(", ");
    }

    format!(
        "{}, ... ({} total)",
        items
            .iter()
            .take(LIMIT)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        items.len()
    )
}

fn page_order_issues(book_root: &Path, page_files: &[PathBuf]) -> Vec<ValidationIssue> {
    let numbered = page_files
        .iter()
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            let index = stem.parse::<u64>().ok()?;
            Some((path.clone(), index))
        })
        .collect::<Vec<_>>();
    if numbered.len() != page_files.len() {
        return Vec::new();
    }

    let mut numeric_sorted = numbered.clone();
    numeric_sorted.sort_by(|(left_path, left_index), (right_path, right_index)| {
        left_index
            .cmp(right_index)
            .then_with(|| left_path.cmp(right_path))
    });
    let lexicographic = numbered.iter().map(|(path, _)| path).collect::<Vec<_>>();
    let numeric = numeric_sorted
        .iter()
        .map(|(path, _)| path)
        .collect::<Vec<_>>();

    if lexicographic == numeric {
        return Vec::new();
    }

    vec![
        ValidationIssue::warning(
            "common",
            "lexicographic page order differs from numeric order".to_string(),
            "ページ順はファイル名の辞書順で決まります。ゼロ埋めした連番へ揃えてください。",
        )
        .at(book_root.join("manga/pages")),
    ]
}

fn page_size_issues(
    book_root: &Path,
    page_assets: &[manga::MangaPageAsset],
) -> Vec<ValidationIssue> {
    let Some(first_page) = page_assets.first() else {
        return Vec::new();
    };
    let expected = (first_page.width_px, first_page.height_px);
    page_assets
        .iter()
        .enumerate()
        .filter(|(_, page)| (page.width_px, page.height_px) != expected)
        .map(|(index, page)| {
            ValidationIssue::error(
                "common",
                format!(
                    "manga page size mismatch on page {}: expected {}x{}, got {}x{}",
                    index + 1,
                    expected.0,
                    expected.1,
                    page.width_px,
                    page.height_px
                ),
                "manga/pages/ の画像を修正し、すべてのページを同じ仕上がりサイズに揃えてください。",
            )
            .at(book_root.join("manga/pages").join(&page.file_name))
        })
        .collect()
}

fn kindle_spread_policy_issues(
    resolved: &config::ResolvedBookConfig,
    page_assets: &[manga::MangaPageAsset],
) -> Vec<ValidationIssue> {
    let Some(manga_settings) = resolved.effective.manga.as_ref() else {
        return Vec::new();
    };
    let wide_pages = page_assets
        .iter()
        .filter(|page| page.is_wide_spread_candidate())
        .collect::<Vec<_>>();
    if wide_pages.is_empty() {
        return Vec::new();
    }

    let page_path = |file_name: &str| {
        resolved
            .repo
            .book
            .as_ref()
            .expect("book context must exist")
            .root
            .join("manga/pages")
            .join(file_name)
    };

    match manga_settings.spread_policy_for_kindle {
        config::SpreadPolicyForKindle::Split => Vec::new(),
        config::SpreadPolicyForKindle::SinglePage => wide_pages
            .into_iter()
            .map(|page| {
                ValidationIssue::warning(
                    "kindle",
                    format!(
                        "wide manga page will be emitted as a single Kindle page: {}",
                        page.file_name
                    ),
                    "manga.spread_policy_for_kindle を split にすると、見開き候補を 2 ページへ分割します。",
                )
                .at(page_path(&page.file_name))
            })
            .collect(),
        config::SpreadPolicyForKindle::Skip => {
            if wide_pages.len() == page_assets.len() {
                vec![ValidationIssue::error(
                    "kindle",
                    "kindle spread policy would skip every manga page".to_string(),
                    "manga.spread_policy_for_kindle を split または single-page に変更してください。",
                )]
            } else {
                wide_pages
                    .into_iter()
                    .map(|page| {
                        ValidationIssue::warning(
                            "kindle",
                            format!(
                                "wide manga page will be omitted from Kindle output: {}",
                                page.file_name
                            ),
                            "manga.spread_policy_for_kindle を split または single-page に変更すると、このページも Kindle 出力へ含められます。",
                        )
                        .at(page_path(&page.file_name))
                    })
                    .collect()
            }
        }
    }
}

fn manga_color_policy_issues(
    resolved: &config::ResolvedBookConfig,
    page_assets: &[manga::MangaPageAsset],
) -> Vec<ValidationIssue> {
    let Some(manga_settings) = resolved.effective.manga.as_ref() else {
        return Vec::new();
    };

    let page_path = |file_name: &str| {
        resolved
            .repo
            .book
            .as_ref()
            .expect("book context must exist")
            .root
            .join("manga/pages")
            .join(file_name)
    };

    let front_color_pages = manga_settings.front_color_pages as usize;
    if front_color_pages > page_assets.len() {
        return vec![ValidationIssue::error(
            "common",
            format!(
                "manga.front_color_pages exceeds resolved page count: {} > {}",
                front_color_pages,
                page_assets.len()
            ),
            "manga.front_color_pages を実際のページ数以下に修正してください。",
        )];
    }

    let mut issues = page_assets
        .iter()
        .take(front_color_pages)
        .filter(|page| !page.is_color)
        .map(|page| {
            ValidationIssue::warning(
                "common",
                format!(
                    "front color page is not detected as color: {}",
                    page.file_name
                ),
                "巻頭カラーページ数を減らすか、該当ページの画像内容を確認してください。",
            )
            .at(page_path(&page.file_name))
        })
        .collect::<Vec<_>>();

    let body_pages = &page_assets[front_color_pages..];
    match manga_settings.body_mode {
        config::MangaBodyMode::Monochrome => {
            issues.extend(body_pages.iter().filter(|page| page.is_color).map(|page| {
                ValidationIssue::error(
                    "common",
                    format!(
                        "body page is detected as color while manga.body_mode is monochrome: {}",
                        page.file_name
                    ),
                    "manga.body_mode を mixed または color に変更するか、本文ページをモノクロ画像へ揃えてください。",
                )
                .at(page_path(&page.file_name))
            }));
        }
        config::MangaBodyMode::Color => {
            issues.extend(body_pages.iter().filter(|page| !page.is_color).map(|page| {
                ValidationIssue::warning(
                    "common",
                    format!(
                        "body page is not detected as color while manga.body_mode is color: {}",
                        page.file_name
                    ),
                    "manga.body_mode を mixed または monochrome に変更するか、本文ページの画像内容を確認してください。",
                )
                .at(page_path(&page.file_name))
            }));
        }
        config::MangaBodyMode::Mixed => {}
    }

    issues
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use serde_json::Value;

    use crate::cli_api::CommandContext;

    use super::page_check;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-page-check-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tiny_png(color: Rgba<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, color));
        image
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn write_manga_book(root: &Path, front_color_pages: usize, body_mode: &str) {
        fs::create_dir_all(root.join("manga/pages")).unwrap();
        fs::write(
            root.join("book.yml"),
            format!(
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
validation:
  strict: true
git:
  lfs: true
manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: skip
  front_color_pages: {front_color_pages}
  body_mode: {body_mode}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn page_check_reports_lexicographic_order_warning() {
        let root = temp_dir("order-warning");
        write_manga_book(&root, 0, "mixed");
        fs::write(
            root.join("manga/pages/1.png"),
            tiny_png(Rgba([0, 0, 0, 255])),
        )
        .unwrap();
        fs::write(
            root.join("manga/pages/2.png"),
            tiny_png(Rgba([0, 0, 0, 255])),
        )
        .unwrap();
        fs::write(
            root.join("manga/pages/10.png"),
            tiny_png(Rgba([0, 0, 0, 255])),
        )
        .unwrap();

        let result = page_check(&CommandContext::new(&root, None, None)).unwrap();
        let report = fs::read_to_string(result.report_path).unwrap();
        let json: Value = serde_json::from_str(&report).unwrap();

        assert!(!result.has_errors);
        assert!(report.contains("lexicographic page order differs from numeric order"));
        assert!(result.summary.contains("page order: 1.png, 10.png, 2.png"));
        assert!(result.summary.contains("spread candidates: none"));
        assert_eq!(
            json["page_order"],
            serde_json::json!(["1.png", "10.png", "2.png"])
        );
        assert_eq!(json["spread_candidates"].as_array().unwrap().len(), 0);
        assert_eq!(
            json["issues"][0]["location"]["path"],
            root.join("manga").join("pages").display().to_string()
        );
        assert!(json["issues"][0]["location"]["line"].is_null());
    }

    #[test]
    fn page_check_reports_color_policy_errors() {
        let root = temp_dir("color-policy");
        write_manga_book(&root, 0, "monochrome");
        fs::write(
            root.join("manga/pages/001.png"),
            tiny_png(Rgba([255, 0, 0, 255])),
        )
        .unwrap();

        let result = page_check(&CommandContext::new(&root, None, None)).unwrap();
        let report = fs::read_to_string(result.report_path).unwrap();
        let json: Value = serde_json::from_str(&report).unwrap();

        assert!(result.has_errors);
        assert!(
            report.contains("body page is detected as color while manga.body_mode is monochrome")
        );
        assert!(result.summary.contains("page order: 001.png"));
        assert!(result.summary.contains("spread candidates: none"));
        assert_eq!(
            json["issues"][0]["location"]["path"],
            root.join("manga")
                .join("pages")
                .join("001.png")
                .display()
                .to_string()
        );
        assert!(json["issues"][0]["location"]["line"].is_null());
    }

    #[test]
    fn page_check_reports_spread_candidates_in_summary_and_json() {
        let root = temp_dir("spread-candidates");
        write_manga_book(&root, 0, "mixed");
        fs::write(
            root.join("manga/pages/001.png"),
            tiny_png(Rgba([0, 0, 0, 255])),
        )
        .unwrap();
        fs::write(root.join("manga/pages/002.png"), wide_png_like()).unwrap();

        let result = page_check(&CommandContext::new(&root, None, None)).unwrap();
        let report = fs::read_to_string(result.report_path).unwrap();
        let json: Value = serde_json::from_str(&report).unwrap();

        assert!(result.summary.contains("page order: 001.png, 002.png"));
        assert!(result.summary.contains("spread candidates: 002.png"));
        assert_eq!(json["spread_candidates"], serde_json::json!(["002.png"]));
    }

    #[test]
    fn summarize_file_list_truncates_after_ten_items() {
        let items = (1..=11)
            .map(|index| format!("{index:03}.png"))
            .collect::<Vec<_>>();

        assert_eq!(
            super::summarize_file_list(&items),
            "001.png, 002.png, 003.png, 004.png, 005.png, 006.png, 007.png, 008.png, 009.png, 010.png, ... (11 total)"
        );
    }

    fn wide_png_like() -> Vec<u8> {
        let mut bytes = Vec::new();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 1, Rgba([0, 0, 0, 255])));
        image
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }
}
