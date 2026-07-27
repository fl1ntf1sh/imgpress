use crate::Result;
use image::DynamicImage;
use std::path::Path;

pub(super) trait PdfRenderer {
    fn render_pages(&self, path: &Path) -> Result<Vec<DynamicImage>>;
    fn render_bytes(&self, bytes: &[u8]) -> Result<Vec<DynamicImage>>;
}
