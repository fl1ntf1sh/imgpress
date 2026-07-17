use crate::config::{CompressOptions, Format, SizeLimit};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub last_input: Option<PathBuf>,
    pub last_output: Option<PathBuf>,
    pub max_size_kb: u32,
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub recursive: bool,
    pub delete_source: bool,
    pub write_log: bool,
    pub max_scales: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_input: None,
            last_output: None,
            max_size_kb: 500,
            format: Format::Jpeg,
            min_quality: 20,
            max_quality: 95,
            scale_step: 0.85,
            preserve_structure: true,
            skip_if_smaller: true,
            recursive: true,
            delete_source: false,
            write_log: false,
            max_scales: 8,
        }
    }
}

impl From<AppSettings> for CompressOptions {
    fn from(s: AppSettings) -> Self {
        CompressOptions {
            input: s.last_input.unwrap_or_default(),
            output: s.last_output.unwrap_or_default(),
            max_size: SizeLimit::from_kb(s.max_size_kb),
            format: s.format,
            min_quality: s.min_quality,
            max_quality: s.max_quality,
            scale_step: s.scale_step,
            max_scales: s.max_scales,
            preserve_structure: s.preserve_structure,
            skip_if_smaller: s.skip_if_smaller,
            recursive: s.recursive,
            delete_source: s.delete_source,
        }
    }
}

impl AppSettings {
    pub fn config_path() -> Option<PathBuf> {
        let dir = crate::app_data_dir()?;
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let Ok(data) = std::fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&data).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}