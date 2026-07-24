use crate::pipeline::CompressReport;
use std::path::Path;

pub trait ProgressReporter: Send + Sync {
    fn on_start(&self, total: usize);
    fn on_file_start(&self, name: &Path);
    fn on_file_progress(&self, _name: &Path, _done: usize, _total: usize) {}
    fn on_file_done(&self, name: &Path, ok: bool, msg: Option<&str>);
    fn on_finish(&self, report: &CompressReport);
    fn confirm_delete_source(&self, _input: &Path) -> bool {
        true
    }
    fn is_cancelled(&self) -> bool;
}

pub struct CliProgress;

impl ProgressReporter for CliProgress {
    fn on_start(&self, total: usize) {
        if total > 0 {
            eprintln!("处理 {} 个文件 ...", total);
        }
    }
    fn on_file_start(&self, _name: &std::path::Path) {}
    fn on_file_done(&self, name: &std::path::Path, ok: bool, msg: Option<&str>) {
        if !ok {
            let m = msg.unwrap_or("未知错误");
            eprintln!("  失败: {} - {}", name.display(), m);
        }
    }
    fn on_finish(&self, _report: &CompressReport) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}
