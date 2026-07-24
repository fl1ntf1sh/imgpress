mod image;
mod pdf;
mod types;

pub use types::{ExtractedImage, ImageLabel, InputKind};

use crate::Result;
use std::path::Path;

pub fn extract_images(path: &Path) -> Result<Vec<ExtractedImage>> {
    match kind_from_path(path) {
        Some(InputKind::Pdf) => pdf::extract(path),
        Some(InputKind::Image) | None => image::extract(path),
    }
}

pub fn is_multi_image_input(path: &Path) -> bool {
    matches!(kind_from_path(path), Some(InputKind::Pdf))
}

pub fn kind_from_path(path: &Path) -> Option<InputKind> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("pdf") {
        Some(InputKind::Pdf)
    } else {
        Some(InputKind::Image)
    }
}
