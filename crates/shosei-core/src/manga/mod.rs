use std::{
    ffi::OsStr,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use image::{DynamicImage, ImageFormat};
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, RawImage, XObjectTransform};
use thiserror::Error;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::config::{MangaPageSide, ReadingDirection, SpreadPolicyForKindle};

const PAGE_DIR: &str = "manga/pages";
const IMAGE_DPI: f32 = 300.0;

#[derive(Debug, Clone)]
pub struct MangaPageAsset {
    pub file_name: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
    pub is_color: bool,
}

impl MangaPageAsset {
    pub fn is_wide_spread_candidate(&self) -> bool {
        self.width_px > self.height_px
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FixedLayoutOptions {
    pub reading_direction: ReadingDirection,
    pub default_page_side: MangaPageSide,
    pub spread_policy_for_kindle: SpreadPolicyForKindle,
}

#[derive(Debug, Error)]
pub enum MangaRenderError {
    #[error("manga page directory not found: {path}")]
    MissingPageDirectory { path: PathBuf },
    #[error("no supported page images were found under {path}")]
    NoPageImages { path: PathBuf },
    #[error("failed to read manga page {path}: {source}")]
    ReadPage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to decode manga page {path}")]
    DecodePage { path: PathBuf },
    #[error("failed to encode manga page {path}: {source}")]
    EncodePage {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error("failed to write manga artifact {path}: {source}")]
    WriteArtifact {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to package manga EPUB {path}: {source}")]
    PackageEpub {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("kindle spread policy removed every manga page for {path}")]
    EmptyKindlePageSet { path: PathBuf },
}

pub fn discover_page_files(book_root: &Path) -> Result<Vec<PathBuf>, MangaRenderError> {
    let page_dir = book_root.join(PAGE_DIR);
    if !page_dir.is_dir() {
        return Err(MangaRenderError::MissingPageDirectory { path: page_dir });
    }

    let mut pages = fs::read_dir(&page_dir)
        .map_err(|_| MangaRenderError::MissingPageDirectory {
            path: page_dir.clone(),
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && media_type_for_path(path).is_some())
        .collect::<Vec<_>>();
    pages.sort();

    if pages.is_empty() {
        return Err(MangaRenderError::NoPageImages { path: page_dir });
    }

    Ok(pages)
}

pub fn write_fixed_layout_epub(
    book_id: &str,
    title: &str,
    language: &str,
    page_paths: &[PathBuf],
    output: &Path,
    options: FixedLayoutOptions,
) -> Result<(), MangaRenderError> {
    let pages = resolve_kindle_page_assets(page_paths, options)?;
    let file = fs::File::create(output).map_err(|source| MangaRenderError::WriteArtifact {
        path: output.to_path_buf(),
        source,
    })?;
    let mut zip = ZipWriter::new(file);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file("mimetype", stored)
        .map_err(|source| MangaRenderError::PackageEpub {
            path: output.to_path_buf(),
            source,
        })?;
    zip.write_all(b"application/epub+zip")
        .map_err(|source| MangaRenderError::WriteArtifact {
            path: output.to_path_buf(),
            source,
        })?;

    write_zip_entry(
        &mut zip,
        "META-INF/container.xml",
        container_xml().as_bytes(),
        output,
    )?;
    write_zip_entry(
        &mut zip,
        "OEBPS/styles/fxl.css",
        fxl_css().as_bytes(),
        output,
    )?;
    write_zip_entry(
        &mut zip,
        "OEBPS/nav.xhtml",
        nav_document(&pages).as_bytes(),
        output,
    )?;
    write_zip_entry(
        &mut zip,
        "OEBPS/package.opf",
        package_document(book_id, title, language, &pages, options).as_bytes(),
        output,
    )?;

    for (index, page) in pages.iter().enumerate() {
        write_zip_entry(
            &mut zip,
            &format!("OEBPS/pages/page-{:04}.xhtml", index + 1),
            page_document(page).as_bytes(),
            output,
        )?;
        write_zip_entry(
            &mut zip,
            &format!("OEBPS/images/{}", page.file_name),
            &page.bytes,
            output,
        )?;
    }

    zip.finish()
        .map_err(|source| MangaRenderError::PackageEpub {
            path: output.to_path_buf(),
            source,
        })?;
    Ok(())
}

pub fn write_image_pdf(
    title: &str,
    page_paths: &[PathBuf],
    output: &Path,
) -> Result<(), MangaRenderError> {
    let mut doc = PdfDocument::new(title);
    let mut pdf_pages = Vec::new();

    for path in page_paths {
        let bytes = fs::read(path).map_err(|source| MangaRenderError::ReadPage {
            path: path.clone(),
            source,
        })?;
        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&bytes, &mut warnings)
            .map_err(|_| MangaRenderError::DecodePage { path: path.clone() })?;
        let width_mm = Mm((image.width as f32) * 25.4 / IMAGE_DPI);
        let height_mm = Mm((image.height as f32) * 25.4 / IMAGE_DPI);
        let image_id = doc.add_image(&image);
        let page = PdfPage::new(
            width_mm,
            height_mm,
            vec![Op::UseXobject {
                id: image_id,
                transform: XObjectTransform {
                    dpi: Some(IMAGE_DPI),
                    ..Default::default()
                },
            }],
        );
        pdf_pages.push(page);
    }

    let mut warnings = Vec::new();
    let pdf_bytes = doc
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    fs::write(output, pdf_bytes).map_err(|source| MangaRenderError::WriteArtifact {
        path: output.to_path_buf(),
        source,
    })
}

pub fn inspect_page_assets(
    page_paths: &[PathBuf],
) -> Result<Vec<MangaPageAsset>, MangaRenderError> {
    load_page_assets(page_paths)
}

fn resolve_kindle_page_assets(
    page_paths: &[PathBuf],
    options: FixedLayoutOptions,
) -> Result<Vec<MangaPageAsset>, MangaRenderError> {
    let mut resolved = Vec::new();

    for page in load_page_assets(page_paths)? {
        if !page.is_wide_spread_candidate() {
            resolved.push(page);
            continue;
        }

        match options.spread_policy_for_kindle {
            SpreadPolicyForKindle::Split => {
                resolved.extend(split_page_asset(&page, options.reading_direction)?);
            }
            SpreadPolicyForKindle::SinglePage => resolved.push(page),
            SpreadPolicyForKindle::Skip => {}
        }
    }

    if resolved.is_empty() {
        let path = page_paths
            .first()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(PAGE_DIR));
        return Err(MangaRenderError::EmptyKindlePageSet { path });
    }

    Ok(resolved)
}

fn load_page_assets(page_paths: &[PathBuf]) -> Result<Vec<MangaPageAsset>, MangaRenderError> {
    page_paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|source| MangaRenderError::ReadPage {
                path: path.clone(),
                source,
            })?;
            let image = image::load_from_memory(&bytes)
                .map_err(|_| MangaRenderError::DecodePage { path: path.clone() })?;
            Ok(MangaPageAsset {
                file_name: path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or("page.bin")
                    .to_string(),
                media_type: media_type_for_path(path).unwrap_or("application/octet-stream"),
                bytes,
                width_px: image.width(),
                height_px: image.height(),
                is_color: image_is_color(&image),
            })
        })
        .collect()
}

fn split_page_asset(
    page: &MangaPageAsset,
    reading_direction: ReadingDirection,
) -> Result<Vec<MangaPageAsset>, MangaRenderError> {
    let image = image::load_from_memory(&page.bytes).map_err(|_| MangaRenderError::DecodePage {
        path: PathBuf::from(&page.file_name),
    })?;
    let split_at = image.width() / 2;
    let left = image.crop_imm(0, 0, split_at, image.height());
    let right = image.crop_imm(split_at, 0, image.width() - split_at, image.height());
    let stem = Path::new(&page.file_name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("page");

    let ordered = match reading_direction {
        ReadingDirection::Ltr => [(left, "left"), (right, "right")],
        ReadingDirection::Rtl => [(right, "right"), (left, "left")],
    };

    ordered
        .into_iter()
        .map(|(image, side)| {
            let file_name = format!("{stem}-{side}.png");
            let bytes = encode_png(&image, &file_name)?;
            Ok(MangaPageAsset {
                file_name,
                media_type: "image/png",
                bytes,
                width_px: image.width(),
                height_px: image.height(),
                is_color: page.is_color,
            })
        })
        .collect()
}

fn encode_png(image: &DynamicImage, file_name: &str) -> Result<Vec<u8>, MangaRenderError> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|source| MangaRenderError::EncodePage {
            path: PathBuf::from(file_name),
            source,
        })?;
    Ok(bytes)
}

fn image_is_color(image: &DynamicImage) -> bool {
    const COLOR_THRESHOLD: u8 = 3;

    image.to_rgba8().pixels().any(|pixel| {
        let [r, g, b, _] = pixel.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        max.saturating_sub(min) >= COLOR_THRESHOLD
    })
}

fn media_type_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(OsStr::to_str) {
        Some("jpg" | "jpeg" | "JPG" | "JPEG") => Some("image/jpeg"),
        Some("png" | "PNG") => Some("image/png"),
        _ => None,
    }
}

fn write_zip_entry(
    zip: &mut ZipWriter<fs::File>,
    name: &str,
    contents: &[u8],
    output: &Path,
) -> Result<(), MangaRenderError> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(name, options)
        .map_err(|source| MangaRenderError::PackageEpub {
            path: output.to_path_buf(),
            source,
        })?;
    zip.write_all(contents)
        .map_err(|source| MangaRenderError::WriteArtifact {
            path: output.to_path_buf(),
            source,
        })
}

fn container_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#
    .to_string()
}

fn fxl_css() -> String {
    "html, body { margin: 0; padding: 0; width: 100%; height: 100%; }\nimg { display: block; width: 100%; height: 100%; object-fit: contain; }\n".to_string()
}

fn nav_document(pages: &[MangaPageAsset]) -> String {
    let items = pages
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "      <li><a href=\"pages/page-{number:04}.xhtml\">Page {number}</a></li>",
                number = index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n  <head>\n    <title>Navigation</title>\n  </head>\n  <body>\n    <nav epub:type=\"toc\" id=\"toc\">\n      <ol>\n{items}\n      </ol>\n    </nav>\n  </body>\n</html>\n"
    )
}

fn package_document(
    book_id: &str,
    title: &str,
    language: &str,
    pages: &[MangaPageAsset],
    options: FixedLayoutOptions,
) -> String {
    let page_manifest = pages
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let side = page_side_for_index(index, options.default_page_side);
            format!(
                "    <item id=\"page-{number:04}\" href=\"pages/page-{number:04}.xhtml\" media-type=\"application/xhtml+xml\" properties=\"{side}\"/>",
                number = index + 1,
                side = page_side_property(side),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let image_manifest = pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            format!(
                "    <item id=\"img-{number:04}\" href=\"images/{file_name}\" media-type=\"{media_type}\"/>",
                number = index + 1,
                file_name = xml_escape(&page.file_name),
                media_type = page.media_type
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let spine = pages
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "    <itemref idref=\"page-{number:04}\"/>",
                number = index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package version=\"3.0\" unique-identifier=\"bookid\" xmlns=\"http://www.idpf.org/2007/opf\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n    <dc:identifier id=\"bookid\">shosei:{book_id}</dc:identifier>\n    <dc:title>{title}</dc:title>\n    <dc:language>{language}</dc:language>\n    <meta property=\"rendition:layout\">pre-paginated</meta>\n    <meta property=\"rendition:orientation\">auto</meta>\n    <meta property=\"rendition:spread\">auto</meta>\n  </metadata>\n  <manifest>\n    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n    <item id=\"fxl-css\" href=\"styles/fxl.css\" media-type=\"text/css\"/>\n{page_manifest}\n{image_manifest}\n  </manifest>\n  <spine page-progression-direction=\"{page_progression_direction}\">\n{spine}\n  </spine>\n</package>\n",
        book_id = xml_escape(book_id),
        title = xml_escape(title),
        language = xml_escape(language),
        page_progression_direction = options.reading_direction.as_str(),
    )
}

fn page_side_for_index(index: usize, default_page_side: MangaPageSide) -> MangaPageSide {
    if index.is_multiple_of(2) {
        default_page_side
    } else {
        match default_page_side {
            MangaPageSide::Left => MangaPageSide::Right,
            MangaPageSide::Right => MangaPageSide::Left,
        }
    }
}

fn page_side_property(side: MangaPageSide) -> &'static str {
    match side {
        MangaPageSide::Left => "page-spread-left",
        MangaPageSide::Right => "page-spread-right",
    }
}

fn page_document(page: &MangaPageAsset) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n  <head>\n    <title>{title}</title>\n    <meta name=\"viewport\" content=\"width={width},height={height}\"/>\n    <link rel=\"stylesheet\" type=\"text/css\" href=\"../styles/fxl.css\"/>\n  </head>\n  <body>\n    <img src=\"../images/{file_name}\" alt=\"{title}\"/>\n  </body>\n</html>\n",
        title = xml_escape(&page.file_name),
        width = page.width_px,
        height = page.height_px,
        file_name = xml_escape(&page.file_name),
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
