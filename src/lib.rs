pub mod error;
pub mod config;
pub mod codec;
pub mod compressor;
pub mod discovery;
pub mod pipeline;
pub mod progress;
pub mod settings;
pub mod source;
pub mod log;
pub mod gui;
pub mod cli;
pub mod pdf;

pub use error::{Error, Result};
pub use config::{CompressOptions, Format, SizeLimit};
pub use pipeline::{compress_directory, CompressReport};
pub use discovery::{FileTask, is_supported, collect_files};

pub fn app_data_dir() -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "imgpress")?;
    let data = dirs.data_dir();
    let path = match data.file_name() {
        Some(name) if name == "data" || name == "config" || name == "cache" => data.parent()?.to_path_buf(),
        _ => data.to_path_buf(),
    };
    Some(path)
}