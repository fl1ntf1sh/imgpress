use super::delete_confirm::DeletePrompt;
use super::run_log::RunLog;
use super::slint_app::MainWindow;
use crate::pipeline::CompressReport;
use crate::progress::ProgressReporter;
use slint::ComponentHandle;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub(super) type Runtime = Arc<Mutex<Option<Arc<AtomicBool>>>>;

struct SlintReporter {
    ui: slint::Weak<MainWindow>,
    cancel: Arc<AtomicBool>,
    total: AtomicUsize,
    processed: AtomicUsize,
    log: RunLog,
    delete_prompt: DeletePrompt,
}

impl SlintReporter {
    fn new(
        ui: slint::Weak<MainWindow>,
        cancel: Arc<AtomicBool>,
        delete_prompt: DeletePrompt,
    ) -> Self {
        Self {
            ui,
            cancel,
            total: AtomicUsize::new(0),
            processed: AtomicUsize::new(0),
            log: RunLog::new(12),
            delete_prompt,
        }
    }

    fn update_ui(&self, f: impl FnOnce(&MainWindow) + Send + 'static) {
        update_ui(self.ui.clone(), f);
    }

    fn append_log(&self, line: impl Into<String>) {
        let text = self.log.append(line);
        self.update_ui(move |ui| {
            ui.set_run_log(text.into());
        });
    }
}

impl ProgressReporter for SlintReporter {
    fn on_start(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.processed.store(0, Ordering::Relaxed);
        self.update_ui(move |ui| {
            ui.set_running(true);
            ui.set_progress(0.0);
            ui.set_run_log("开始扫描并处理文件...".into());
            ui.set_summary(format!("共 {} 个文件", total).into());
        });
    }

    fn on_file_start(&self, name: &Path) {
        self.append_log(format!("正在处理：{}", name.display()));
    }

    fn on_file_progress(&self, _name: &Path, done: usize, total: usize) {
        if total > 1 {
            self.append_log(format!("  当前文件进度：{}/{}", done, total));
        }
    }

    fn on_file_done(&self, name: &Path, ok: bool, msg: Option<&str>) {
        let done = self.processed.fetch_add(1, Ordering::Relaxed) + 1;
        let total = self.total.load(Ordering::Relaxed);
        let progress = if total == 0 {
            0.0
        } else {
            done as f32 / total as f32
        };
        self.update_ui(move |ui| {
            ui.set_progress(progress);
            ui.set_summary(format!("已处理 {}/{} 个文件", done, total).into());
        });
        if ok {
            self.append_log(format!("完成：{}", name.display()));
        } else {
            self.append_log(format!(
                "失败：{} - {}",
                name.display(),
                msg.unwrap_or("未知错误")
            ));
        }
    }

    fn on_finish(&self, report: &CompressReport) {
        let report = report.clone();
        let cancelled = self.cancel.load(Ordering::Relaxed);
        self.update_ui(move |ui| {
            ui.set_running(false);
            ui.set_progress(1.0);
            let summary = if cancelled {
                format!(
                    "已取消：{} 个成功，{} 个失败",
                    report.success,
                    report.failed.len()
                )
            } else {
                format!(
                    "完成：{} 个成功，{} 个失败，{:.2} MB -> {:.2} MB",
                    report.success,
                    report.failed.len(),
                    report.bytes_in as f64 / 1_048_576.0,
                    report.bytes_out as f64 / 1_048_576.0
                )
            };
            ui.set_summary(summary.into());
        });
        self.append_log("任务结束。");
    }

    fn confirm_delete_source(&self, input: &Path) -> bool {
        self.append_log(format!("等待确认是否删除源文件：{}", input.display()));
        let should_delete = super::delete_confirm::confirm_with_timeout(
            self.ui.clone(),
            &self.delete_prompt,
            input,
        );
        if should_delete {
            self.append_log("已确认删除源文件。");
        } else {
            self.append_log("已取消删除源文件。");
        }
        should_delete
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

pub(super) fn start_compression(app: &MainWindow, runtime: &Runtime, delete_prompt: &DeletePrompt) {
    if app.get_running() {
        return;
    }

    let opts = match crate::gui::ui_options::options_from_ui(app) {
        Ok(opts) => opts,
        Err(msg) => {
            app.set_summary(msg.into());
            return;
        }
    };
    crate::gui::settings_binding::save_settings_from_ui(app, &opts);

    let cancel = Arc::new(AtomicBool::new(false));
    *runtime.lock().unwrap() = Some(cancel.clone());
    app.set_running(true);
    app.set_progress(0.0);
    app.set_run_log("正在启动任务...".into());
    app.set_summary("正在启动".into());

    let ui = app.as_weak();
    let write_log = app.get_write_log();
    let delete_prompt = delete_prompt.clone();
    std::thread::spawn(move || {
        let reporter = SlintReporter::new(ui.clone(), cancel, delete_prompt);
        let result =
            crate::pipeline::compress_directory(&opts.input, &opts.output, &opts, &reporter);
        match result {
            Ok(report) => {
                if write_log {
                    if let Some(path) = crate::log::log_file_path() {
                        if let Err(e) = crate::log::write_log_file(&report, &opts, &path) {
                            update_status(
                                ui,
                                format!("写入日志失败: {}", e),
                                Some("写入日志失败。".into()),
                                false,
                            );
                        }
                    }
                }
            }
            Err(e) => update_status(
                ui,
                format!("处理失败: {}", e),
                Some("任务失败。".into()),
                false,
            ),
        }
    });
}

fn update_status(ui: slint::Weak<MainWindow>, message: String, log: Option<String>, running: bool) {
    update_ui(ui, move |ui| {
        ui.set_running(running);
        ui.set_summary(message.into());
        if let Some(log) = log {
            ui.set_run_log(log.into());
        }
    });
}

fn update_ui(ui: slint::Weak<MainWindow>, f: impl FnOnce(&MainWindow) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            f(&ui);
        }
    });
}
