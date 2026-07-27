mod mupdf;
mod renderer;

use crate::Result;
use image::DynamicImage;
use renderer::PdfRenderer;

pub fn render_pdf_pages(path: &std::path::Path) -> Result<Vec<DynamicImage>> {
    mupdf::MuPdfRenderer.render_pages(path)
}

pub(crate) fn render_pdf_bytes(bytes: &[u8]) -> Result<Vec<DynamicImage>> {
    mupdf::MuPdfRenderer.render_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_pdf_from_converted_bytes() {
        let bytes = include_bytes!("../../patch/mupdf/tests/files/dummy.pdf");

        let pages = render_pdf_bytes(bytes).unwrap();

        assert!(!pages.is_empty());
    }
}
