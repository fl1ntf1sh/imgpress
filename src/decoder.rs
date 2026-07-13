use crate::Result;
use image::DynamicImage;

pub fn load_image(path: &std::path::Path) -> Result<DynamicImage> {
    let img = image::open(path)?;
    Ok(img)
}

