use crate::source::SourceAction;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CompressReport {
    pub total: usize,
    pub success: usize,
    pub failed: Vec<(PathBuf, String)>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub source_action: SourceAction,
}
