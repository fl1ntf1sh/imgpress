use crate::{Error, Result};
use std::path::Path;

use super::ExtractedImage;

pub(super) fn extract(path: &Path) -> Result<Vec<ExtractedImage>> {
    let converted = office2pdf::convert(path)
        .map_err(|e| Error::Office(format!("Office 转 PDF 失败: {}", e)))?;
    super::pdf::extract_bytes(&converted.pdf)
}
