mod image;
mod pdf;
mod types;

pub use types::{ExtractedImage, ImageLabel};

use crate::Result;
use std::path::Path;

pub fn extract_images(path: &Path) -> Result<Vec<ExtractedImage>> {
    if is_pdf(path) {
        return pdf::extract(path);
    }
    image::extract(path)
}

pub fn is_multi_image_input(path: &Path) -> bool {
    is_pdf(path)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
}
