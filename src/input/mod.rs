mod image;
mod pdf;
mod types;
mod word;

pub use types::{ExtractedImage, ImageLabel, InputKind};

use crate::Result;
use std::path::Path;

pub fn extract_images(path: &Path) -> Result<Vec<ExtractedImage>> {
    match kind_from_path(path) {
        Some(InputKind::Docx) => word::extract(path),
        Some(InputKind::Pdf) => pdf::extract(path),
        Some(InputKind::Image) | None => image::extract(path),
    }
}

pub fn is_multi_image_input(path: &Path) -> bool {
    matches!(kind_from_path(path), Some(InputKind::Pdf | InputKind::Docx))
}

pub fn kind_from_path(path: &Path) -> Option<InputKind> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("pdf") {
        Some(InputKind::Pdf)
    } else if ext.eq_ignore_ascii_case("docx") {
        Some(InputKind::Docx)
    } else {
        Some(InputKind::Image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_is_multi_image_input() {
        let path = Path::new("report.docx");

        assert_eq!(kind_from_path(path), Some(InputKind::Docx));
        assert!(is_multi_image_input(path));
    }
}
