use crate::Result;
use image::DynamicImage;
use std::path::Path;

pub(super) trait PdfRenderer {
    fn render_pages(&self, path: &Path) -> Result<Vec<DynamicImage>>;
}
