use crate::pdf::renderer::PdfRenderer;
use crate::{Error, Result};
use image::DynamicImage;
use std::path::Path;

pub(super) struct MuPdfRenderer;

impl PdfRenderer for MuPdfRenderer {
    fn render_pages(&self, path: &Path) -> Result<Vec<DynamicImage>> {
        let doc = mupdf::Document::open(
            path.to_str()
                .ok_or_else(|| Error::Pdf("path contains non-UTF-8 characters".into()))?,
        )
        .map_err(|e| Error::Pdf(format!("failed to open: {}", e)))?;

        let page_count = doc
            .page_count()
            .map_err(|e| Error::Pdf(format!("无法获取页数: {}", e)))?;
        if page_count == 0 {
            return Err(Error::Pdf("PDF has no pages".into()));
        }

        let scale = 2.0;
        let matrix = mupdf::Matrix::new_scale(scale, scale);

        let mut result = Vec::with_capacity(page_count as usize);
        let mut page_errors: Vec<String> = Vec::new();

        for i in 0..page_count {
            let page = match doc.load_page(i) {
                Ok(p) => p,
                Err(e) => {
                    page_errors.push(format!("page {} load failed: {}", i + 1, e));
                    continue;
                }
            };

            let pixmap =
                match page.to_pixmap(&matrix, &mupdf::Colorspace::device_rgb(), false, false) {
                    Ok(p) => p,
                    Err(e) => {
                        page_errors.push(format!("page {} render failed: {}", i + 1, e));
                        continue;
                    }
                };

            match pixmap_to_image(&pixmap) {
                Some(image) => result.push(image),
                None => {
                    page_errors.push(format!("page {} pixel data size mismatch", i + 1));
                }
            }
        }

        if result.is_empty() {
            let msg = if page_errors.is_empty() {
                "PDF 中没有可提取的页面".to_string()
            } else {
                page_errors.join("; ")
            };
            return Err(Error::Pdf(msg));
        }

        if !page_errors.is_empty() {
            log::warn!("部分页面失败: {}", page_errors.join("; "));
        }

        Ok(result)
    }
}

fn pixmap_to_image(pixmap: &mupdf::Pixmap) -> Option<DynamicImage> {
    let w = pixmap.width();
    let h = pixmap.height();
    let n = pixmap.n() as usize;
    let stride = pixmap.stride() as usize;
    let samples = pixmap.samples();
    let row_bytes = (w as usize) * n;
    let total = row_bytes * (h as usize);

    let buf = if stride == row_bytes {
        samples[..total].to_vec()
    } else {
        let mut buf = Vec::with_capacity(total);
        for y in 0..h as usize {
            let start = y * stride;
            buf.extend_from_slice(&samples[start..start + row_bytes]);
        }
        buf
    };

    if n >= 4 {
        image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba8)
    } else {
        image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb8)
    }
}
