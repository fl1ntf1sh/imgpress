pub mod error;
pub mod config;
pub mod codec;
pub mod decoder;
pub mod compressor;
pub mod pipeline;
pub mod progress;
pub mod settings;
pub mod gui;
pub mod cli;
pub mod pdf;

pub use error::{Error, Result};
pub use config::{CompressOptions, Format, SizeLimit};
pub use pipeline::{compress_directory, CompressReport};

pub fn lib_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}