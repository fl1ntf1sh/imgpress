use crate::progress::ProgressReporter;
use crate::{Error, Result};

pub struct Compressor {
    codec: Box<dyn crate::codec::Codec>,
}

impl Compressor {
    pub fn new(codec: Box<dyn crate::codec::Codec>) -> Self {
        Self { codec }
    }

    pub fn compress_to_size(
        &self,
        img: &image::DynamicImage,
        target: u64,
        min_q: u8,
        max_q: u8,
        scale_step: f32,
        max_scales: u32,
        progress: &dyn ProgressReporter,
    ) -> Result<Vec<u8>> {
        let mut current = img.clone();
        let mut best: Option<Vec<u8>> = None;

        for scale_round in 0..=max_scales {
            let (mut lo, mut hi) = (min_q.max(1), max_q.min(100));
            let mut last_valid: Option<Vec<u8>> = None;

            while lo <= hi {
                if progress.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let mid = ((lo as u16 + hi as u16) / 2) as u8;
                let bytes = self.codec.encode(&current, mid)?;
                let size = bytes.len() as u64;

                if size <= target {
                    last_valid = Some(bytes);
                    lo = mid.saturating_add(1);
                } else {
                    if hi == 0 {
                        break;
                    }
                    hi = mid.saturating_sub(1);
                }
            }

            if let Some(valid) = last_valid {
                if valid.len() as u64 <= target {
                    best = Some(valid);
                    break;
                }
            }

            if scale_round == max_scales {
                break;
            }

            let new_w = ((current.width() as f32) * scale_step).max(1.0) as u32;
            let new_h = ((current.height() as f32) * scale_step).max(1.0) as u32;
            current = current.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
        }

        best.ok_or_else(|| {
            Error::Encode(format!(
                "unable to compress to target size even at min quality and min scale"
            ))
        })
    }
}