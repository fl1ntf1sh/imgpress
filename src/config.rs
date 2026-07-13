use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Format {
    #[default]
    Jpeg,
    WebP,
}

impl Format {
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::WebP => "webp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeLimit {
    pub bytes: u64,
}

impl SizeLimit {
    pub fn from_kb(kb: u32) -> Self {
        Self { bytes: kb as u64 * 1024 }
    }

    pub fn from_bytes(b: u64) -> Self {
        Self { bytes: b }
    }
}

#[derive(Debug, Clone)]
pub struct CompressOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub max_size: SizeLimit,
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub max_scales: u32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub recursive: bool,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            max_size: SizeLimit::from_kb(500),
            format: Format::Jpeg,
            min_quality: 20,
            max_quality: 95,
            scale_step: 0.85,
            max_scales: 8,
            preserve_structure: true,
            skip_if_smaller: true,
            recursive: true,
        }
    }
}