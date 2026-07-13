use crate::codec::Codec;
use crate::{Error, Result};
use image::{DynamicImage, ImageEncoder};

pub struct JpegCodec;

impl JpegCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Codec for JpegCodec {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let rgb = flatten_alpha_to_rgb(img);
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        encoder
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| Error::Encode(format!("jpeg encode: {}", e)))?;
        Ok(buf)
    }
}

fn flatten_alpha_to_rgb(img: &DynamicImage) -> image::RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    image::ImageBuffer::from_fn(w, h, |x, y| {
        let p = rgba.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        if a >= 1.0 {
            image::Rgb([p[0], p[1], p[2]])
        } else {
            image::Rgb([
                (p[0] as f32 * a + 255.0 * (1.0 - a)) as u8,
                (p[1] as f32 * a + 255.0 * (1.0 - a)) as u8,
                (p[2] as f32 * a + 255.0 * (1.0 - a)) as u8,
            ])
        }
    })
}