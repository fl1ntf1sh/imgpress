pub mod jpeg;
pub mod webp;

use crate::Result;
use image::DynamicImage;

pub trait Codec: Send + Sync {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>>;
}
