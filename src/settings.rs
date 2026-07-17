use crate::config::Format;
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