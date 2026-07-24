use crate::{Error, Result};
use std::path::Path;

use super::{ExtractedImage, ImageLabel};

pub fn extract(path: &Path) -> Result<Vec<ExtractedImage>> {
    let image = image::open(path).map_err(|e| Error::Image(format!("解码失败: {}", e)))?;
    Ok(vec![ExtractedImage {
        image,
        label: ImageLabel::Single,
    }])
}
