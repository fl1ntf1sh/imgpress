mod mupdf;
mod renderer;

use crate::Result;
use image::DynamicImage;
use renderer::PdfRenderer;

pub fn render_pdf_pages(path: &std::path::Path) -> Result<Vec<DynamicImage>> {
    mupdf::MuPdfRenderer.render_pages(path)
}
