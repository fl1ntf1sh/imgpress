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
        Self {
            bytes: kb as u64 * 1024,
        }
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
    pub delete_source: bool,
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
            delete_source: false,
        }
    }
}

pub fn validate_options(opts: &CompressOptions) -> std::result::Result<(), String> {
    if opts.input.as_os_str().is_empty() || opts.output.as_os_str().is_empty() {
        return Err("请选择源目录和输出目录".into());
    }
    let max_size_kb = opts.max_size.bytes / 1024;
    if !(1..=102400).contains(&max_size_kb) {
        return Err("目标大小必须在 1 到 102400 KB 之间".into());
    }
    if !(1..=100).contains(&opts.min_quality) || !(1..=100).contains(&opts.max_quality) {
        return Err("质量范围必须在 1 到 100 之间".into());
    }
    if opts.min_quality > opts.max_quality {
        return Err("质量范围无效: 最小值不能大于最大值".into());
    }
    if !(0.10..=0.99).contains(&opts.scale_step) {
        return Err("缩放步长必须在 10% 到 99% 之间".into());
    }
    if opts.max_scales > 32 {
        return Err("缩放轮数必须在 0 到 32 之间".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_options() -> CompressOptions {
        CompressOptions {
            input: PathBuf::from("input"),
            output: PathBuf::from("output"),
            ..CompressOptions::default()
        }
    }

    #[test]
    fn validate_accepts_default_ranges() {
        assert!(validate_options(&valid_options()).is_ok());
    }

    #[test]
    fn validate_rejects_invalid_quality_order() {
        let mut opts = valid_options();
        opts.min_quality = 90;
        opts.max_quality = 80;

        assert!(validate_options(&opts).is_err());
    }

    #[test]
    fn validate_rejects_invalid_scale_step() {
        let mut opts = valid_options();
        opts.scale_step = 1.0;

        assert!(validate_options(&opts).is_err());
    }
}
