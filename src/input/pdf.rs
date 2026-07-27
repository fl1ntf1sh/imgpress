use crate::{Error, Result};
use std::path::Path;

use super::{ExtractedImage, ImageLabel};

pub(super) fn extract(path: &Path) -> Result<Vec<ExtractedImage>> {
    let pages =
        crate::pdf::render_pdf_pages(path).map_err(|e| Error::Pdf(format!("PDF 解析: {}", e)))?;
    extracted_pages(pages)
}

pub(super) fn extract_bytes(bytes: &[u8]) -> Result<Vec<ExtractedImage>> {
    let pages = crate::pdf::render_pdf_bytes(bytes)
        .map_err(|e| Error::Pdf(format!("转换后 PDF 解析: {}", e)))?;
    extracted_pages(pages)
}

fn extracted_pages(pages: Vec<image::DynamicImage>) -> Result<Vec<ExtractedImage>> {
    if pages.is_empty() {
        return Err(Error::Pdf("PDF 没有可提取的页面".into()));
    }

    Ok(pages
        .into_iter()
        .enumerate()
        .map(|(idx, image)| ExtractedImage {
            image,
            label: ImageLabel::Page { index: idx + 1 },
        })
        .collect())
}
