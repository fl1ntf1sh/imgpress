mod image;
mod office;
mod pdf;
mod types;

pub use types::{ExtractedImage, ImageLabel, InputKind};

use crate::Result;
use std::path::Path;

pub fn extract_images(path: &Path) -> Result<Vec<ExtractedImage>> {
    match kind_from_path(path) {
        Some(InputKind::Office) => office::extract(path),
        Some(InputKind::Pdf) => pdf::extract(path),
        Some(InputKind::Image) | None => image::extract(path),
    }
}

pub fn is_multi_image_input(path: &Path) -> bool {
    matches!(
        kind_from_path(path),
        Some(InputKind::Pdf | InputKind::Office)
    )
}

pub fn kind_from_path(path: &Path) -> Option<InputKind> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("pdf") {
        Some(InputKind::Pdf)
    } else if ["docx", "xlsx", "pptx"]
        .iter()
        .any(|office_ext| office_ext.eq_ignore_ascii_case(ext))
    {
        Some(InputKind::Office)
    } else {
        Some(InputKind::Image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_files_use_multi_image_pipeline() {
        for path in ["report.docx", "book.XLSX", "slides.Pptx"] {
            let path = Path::new(path);

            assert_eq!(kind_from_path(path), Some(InputKind::Office));
            assert!(is_multi_image_input(path));
        }
    }
}
