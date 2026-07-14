use crate::{Error, Result};
use image::DynamicImage;

pub fn render_pdf_pages(path: &std::path::Path) -> Result<Vec<DynamicImage>> {
    let doc = mupdf::Document::open(path.to_str().ok_or_else(|| {
        Error::Pdf("路径包含非 UTF-8 字符".into())
    })?)
    .map_err(|e| Error::Pdf(format!("无法打开: {}", e)))?;

    let page_count = doc
        .page_count()
        .map_err(|e| Error::Pdf(format!("无法获取页数: {}", e)))?;
    if page_count == 0 {
        return Err(Error::Pdf("PDF 没有页面".into()));
    }

    let scale = 2.0;
    let matrix = mupdf::Matrix::new_scale(scale, scale);

    let mut result = Vec::with_capacity(page_count as usize);
    let mut page_errors: Vec<String> = Vec::new();

    for i in 0..page_count {
        let page = match doc.load_page(i) {
            Ok(p) => p,
            Err(e) => {
                page_errors.push(format!("第 {} 页加载失败: {}", i + 1, e));
                continue;
            }
        };

        let pixmap = match page.to_pixmap(
            &matrix,
            &mupdf::Colorspace::device_rgb(),
            false,
            false,
        ) {
            Ok(p) => p,
            Err(e) => {
                page_errors.push(format!("第 {} 页渲染失败: {}", i + 1, e));
                continue;
            }
        };

        let w = pixmap.width();
        let h = pixmap.height();
        let n = pixmap.n() as usize;
        let stride = pixmap.stride() as usize;
        let samples = pixmap.samples();
        let row_bytes = (w as usize) * n;
        let total = row_bytes * (h as usize);

        let img = if stride == row_bytes {
            let buf = samples[..total].to_vec();
            if n >= 4 {
                image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba8)
            } else {
                image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb8)
            }
        } else {
            let mut buf = Vec::with_capacity(total);
            for y in 0..h as usize {
                let start = y * stride;
                buf.extend_from_slice(&samples[start..start + row_bytes]);
            }
            if n >= 4 {
                image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgba8)
            } else {
                image::ImageBuffer::from_raw(w, h, buf).map(DynamicImage::ImageRgb8)
            }
        };

        match img {
            Some(image) => result.push(image),
            None => {
                page_errors.push(format!("第 {} 页像素数据尺寸不匹配", i + 1));
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