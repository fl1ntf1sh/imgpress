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
        let n = pixmap.n();
        let stride = pixmap.stride();
        let samples = pixmap.samples();

        let img = if n >= 4 {
            let mut rgba = image::RgbaImage::new(w, h);
            let pixels = rgba.as_mut();
            let stride = stride as usize;
            for y in 0..h as usize {
                let src = y * stride;
                let dst = y * (w as usize) * 4;
                for x in 0..w as usize {
                    pixels[dst + x * 4] = samples[src + x * 4];
                    pixels[dst + x * 4 + 1] = samples[src + x * 4 + 1];
                    pixels[dst + x * 4 + 2] = samples[src + x * 4 + 2];
                    pixels[dst + x * 4 + 3] = samples[src + x * 4 + 3];
                }
            }
            DynamicImage::ImageRgba8(rgba)
        } else {
            let mut rgb = image::RgbImage::new(w, h);
            let pixels = rgb.as_mut();
            let stride = stride as usize;
            for y in 0..h as usize {
                let src = y * stride;
                let dst = y * (w as usize) * 3;
                for x in 0..w as usize {
                    pixels[dst + x * 3] = samples[src + x * 3];
                    pixels[dst + x * 3 + 1] = samples[src + x * 3 + 1];
                    pixels[dst + x * 3 + 2] = samples[src + x * 3 + 2];
                }
            }
            DynamicImage::ImageRgb8(rgb)
        };

        result.push(img);
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