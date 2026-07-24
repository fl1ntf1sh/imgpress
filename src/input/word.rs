use crate::{Error, Result};
use std::io::Read;
use std::path::Path;

use super::{ExtractedImage, ImageLabel};

pub(super) fn extract(path: &Path) -> Result<Vec<ExtractedImage>> {
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Other(format!("DOCX 解析失败: {}", e)))?;
    let mut images = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| Error::Other(format!("DOCX 条目读取失败: {}", e)))?;
        if !is_media_file(entry.name()) {
            continue;
        }

        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let image = image::load_from_memory(&bytes)
            .map_err(|e| Error::Image(format!("DOCX 图片解码失败: {}", e)))?;
        images.push(ExtractedImage {
            image,
            label: ImageLabel::Embedded {
                index: images.len() + 1,
            },
        });
    }

    if images.is_empty() {
        return Err(Error::Other("DOCX 中没有可提取的图片".into()));
    }

    Ok(images)
}

fn is_media_file(name: &str) -> bool {
    let name = name.replace('\\', "/");
    name.starts_with("word/media/") && !name.ends_with('/')
}
