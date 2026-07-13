use crate::pipeline::CompressReport;

pub trait ProgressReporter: Send + Sync {
    fn on_start(&self, total: usize);
    fn on_file_start(&self, name: &std::path::Path);
    fn on_file_done(&self, name: &std::path::Path, ok: bool, msg: Option<&str>);
    fn on_finish(&self, report: &CompressReport);
    fn is_cancelled(&self) -> bool;
}

pub struct NullProgress;

impl ProgressReporter for NullProgress {
    fn on_start(&self, _total: usize) {}
    fn on_file_start(&self, _name: &std::path::Path) {}
    fn on_file_done(&self, _name: &std::path::Path, _ok: bool, _msg: Option<&str>) {}
    fn on_finish(&self, _report: &CompressReport) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}