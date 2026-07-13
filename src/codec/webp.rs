use crate::codec::Codec;
use crate::Result;
use image::DynamicImage;

pub struct WebPCodec;

impl WebPCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Codec for WebPCodec {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);
        let memory = encoder.encode(quality as f32);
        Ok(memory.to_vec())
    }
}