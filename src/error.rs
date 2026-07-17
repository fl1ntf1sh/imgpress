use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    Image(String),

    #[error("image encode error: {0}")]
    Encode(String),

    #[error("pdf error: {0}")]
    Pdf(String),

    #[error("cancelled by user")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::Image(e.to_string())
    }
}

pub fn format_panic(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        format!("panic: {}", s)
    } else if let Some(s) = panic.downcast_ref::<String>() {
        format!("panic: {}", s)
    } else {
        "unknown panic".to_string()
    }
}

pub type Result<T> = std::result::Result<T, Error>;