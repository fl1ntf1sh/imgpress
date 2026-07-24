use crate::config::{CompressOptions, Format, SizeLimit};
use crate::pipeline::CompressReport;
use crate::progress::ProgressReporter;
use crate::settings::AppSettings;
use slint::ComponentHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

slint::include_modules!();

struct SlintReporter {
    ui: slint::Weak<MainWindow>,
    cancel: Arc<AtomicBool>,
    total: AtomicUsize,
    processed: AtomicUsize,
}

impl SlintReporter {
    fn new(ui: slint::Weak<MainWindow>, cancel: Arc<AtomicBool>) -> Self {
        Self {
            ui,
            cancel,
            total: AtomicUsize::new(0),
            processed: AtomicUsize::new(0),
        }
    }

    fn update_ui(&self, f: impl FnOnce(&MainWindow) + Send + 'static) {
        let ui = self.ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                f(&ui);
            }
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
            ui.set_summary(format!("Processing {} files", total).into());
        });
    }

    fn on_file_start(&self, name: &Path) {
        let current = name.display().to_string();
        self.update_ui(move |ui| {
            ui.set_current_file(current.into());
        });
    }

    fn on_file_done(&self, _name: &Path, _ok: bool, _msg: Option<&str>) {
        let done = self.processed.fetch_add(1, Ordering::Relaxed) + 1;
        let total = self.total.load(Ordering::Relaxed);
        let progress = if total == 0 {
            0.0
        } else {
            done as f32 / total as f32
        };
        self.update_ui(move |ui| {
            ui.set_progress(progress);
            ui.set_summary(format!("Processed {}/{} files", done, total).into());
        });
    }

    fn on_finish(&self, report: &CompressReport) {
        let report = report.clone();
        let cancelled = self.cancel.load(Ordering::Relaxed);
        self.update_ui(move |ui| {
            ui.set_running(false);
            ui.set_current_file("".into());
            ui.set_progress(1.0);
            let summary = if cancelled {
                format!(
                    "Cancelled: {} success, {} failed",
                    report.success,
                    report.failed.len()
                )
            } else {
                format!(
                    "Done: {} success, {} failed, {:.2} MB -> {:.2} MB",
                    report.success,
                    report.failed.len(),
                    report.bytes_in as f64 / 1_048_576.0,
                    report.bytes_out as f64 / 1_048_576.0
                )
            };
            ui.set_summary(summary.into());
        });
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

pub fn run() -> crate::Result<()> {
    let app =
        MainWindow::new().map_err(|e| crate::Error::Other(format!("slint init error: {}", e)))?;
    let runtime = Arc::new(Mutex::new(None::<Arc<AtomicBool>>));

    load_settings(&app);
    bind_callbacks(&app, runtime);

    app.run()
        .map_err(|e| crate::Error::Other(format!("slint runtime error: {}", e)))
}

fn load_settings(app: &MainWindow) {
    let settings = AppSettings::load();
    if let Some(path) = settings.last_input {
        app.set_input_path(path.display().to_string().into());
    }
    if let Some(path) = settings.last_output {
        app.set_output_path(path.display().to_string().into());
    }
    app.set_max_size_kb_text(settings.max_size_kb.to_string().into());
    app.set_output_format_index(if settings.format == Format::WebP {
        1
    } else {
        0
    });
    app.set_min_quality_text(settings.min_quality.to_string().into());
    app.set_max_quality_text(settings.max_quality.to_string().into());
    app.set_scale_step_percent_text(
        ((settings.scale_step * 100.0).round() as u32)
            .to_string()
            .into(),
    );
    app.set_max_scales_text(settings.max_scales.to_string().into());
    app.set_preserve_structure(settings.preserve_structure);
    app.set_skip_if_smaller(settings.skip_if_smaller);
    app.set_recursive(settings.recursive);
    app.set_delete_source(settings.delete_source);
    app.set_write_log(settings.write_log);
    app.set_summary("Ready".into());
}

fn bind_callbacks(app: &MainWindow, runtime: Arc<Mutex<Option<Arc<AtomicBool>>>>) {
    let weak = app.as_weak();
    app.on_browse_input(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(app) = weak.upgrade() {
                app.set_input_path(path.display().to_string().into());
            }
        }
    });

    let weak = app.as_weak();
    app.on_browse_output(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(app) = weak.upgrade() {
                app.set_output_path(path.display().to_string().into());
            }
        }
    });

    let weak = app.as_weak();
    let runtime_for_start = runtime.clone();
    app.on_start(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        if app.get_running() {
            return;
        }

        let opts = match options_from_ui(&app) {
            Ok(opts) => opts,
            Err(msg) => {
                app.set_summary(msg.into());
                return;
            }
        };
        save_settings_from_ui(&app, &opts);

        let cancel = Arc::new(AtomicBool::new(false));
        *runtime_for_start.lock().unwrap() = Some(cancel.clone());
        app.set_running(true);
        app.set_progress(0.0);
        app.set_summary("Starting".into());

        let ui = app.as_weak();
        let write_log = app.get_write_log();
        std::thread::spawn(move || {
            let reporter = SlintReporter::new(ui.clone(), cancel.clone());
            let result =
                crate::pipeline::compress_directory(&opts.input, &opts.output, &opts, &reporter);
            match result {
                Ok(report) => {
                    if write_log {
                        if let Some(path) = crate::log::log_file_path() {
                            if let Err(e) = crate::log::write_log_file(&report, &opts, &path) {
                                update_summary(ui, format!("写入日志失败: {}", e), false);
                            }
                        }
                    }
                }
                Err(e) => update_summary(ui, format!("处理失败: {}", e), false),
            }
        });
    });

    let weak = app.as_weak();
    let runtime_for_cancel = runtime.clone();
    app.on_cancel(move || {
        if let Some(cancel) = runtime_for_cancel.lock().unwrap().as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(app) = weak.upgrade() {
            app.set_summary("Cancelling...".into());
        }
    });

    let weak = app.as_weak();
    app.on_open_output(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(app.get_output_path().to_string());
        if path.as_os_str().is_empty() {
            return;
        }
        std::fs::create_dir_all(&path).ok();
        let _ = open::that(path);
    });

    app.on_open_log_dir(move || {
        let Some(path) =
            crate::log::log_file_path().and_then(|path| path.parent().map(Path::to_path_buf))
        else {
            return;
        };
        std::fs::create_dir_all(&path).ok();
        let _ = open::that(path);
    });
}

fn options_from_ui(app: &MainWindow) -> std::result::Result<CompressOptions, String> {
    let input = PathBuf::from(app.get_input_path().trim().to_string());
    let output = PathBuf::from(app.get_output_path().trim().to_string());
    if input.as_os_str().is_empty() || output.as_os_str().is_empty() {
        return Err("请选择源目录和输出目录".into());
    }

    let max_size_kb = parse_u32(app.get_max_size_kb_text().as_str(), "目标大小")?;
    let min_quality = parse_u8(app.get_min_quality_text().as_str(), "最低质量")?;
    let max_quality = parse_u8(app.get_max_quality_text().as_str(), "最高质量")?;
    let scale_step_percent = parse_u32(app.get_scale_step_percent_text().as_str(), "缩放步长")?;
    let max_scales = parse_u32(app.get_max_scales_text().as_str(), "缩放轮数")?;

    if !(1..=102400).contains(&max_size_kb) {
        return Err("目标大小必须在 1 到 102400 KB 之间".into());
    }
    if !(1..=100).contains(&min_quality) || !(1..=100).contains(&max_quality) {
        return Err("质量范围必须在 1 到 100 之间".into());
    }
    if min_quality > max_quality {
        return Err("质量范围无效: 最小值不能大于最大值".into());
    }
    if !(10..=99).contains(&scale_step_percent) {
        return Err("缩放步长必须在 10 到 99 之间".into());
    }
    if max_scales > 32 {
        return Err("缩放轮数必须在 0 到 32 之间".into());
    }

    Ok(CompressOptions {
        input,
        output,
        max_size: SizeLimit::from_kb(max_size_kb),
        format: if app.get_output_format_index() == 1 {
            Format::WebP
        } else {
            Format::Jpeg
        },
        min_quality,
        max_quality,
        scale_step: scale_step_percent as f32 / 100.0,
        max_scales,
        preserve_structure: app.get_preserve_structure(),
        skip_if_smaller: app.get_skip_if_smaller(),
        recursive: app.get_recursive(),
        delete_source: app.get_delete_source(),
    })
}

fn save_settings_from_ui(app: &MainWindow, opts: &CompressOptions) {
    let settings = AppSettings {
        last_input: Some(opts.input.clone()),
        last_output: Some(opts.output.clone()),
        max_size_kb: opts.max_size.bytes.saturating_div(1024) as u32,
        format: opts.format,
        min_quality: opts.min_quality,
        max_quality: opts.max_quality,
        scale_step: opts.scale_step,
        preserve_structure: opts.preserve_structure,
        skip_if_smaller: opts.skip_if_smaller,
        recursive: opts.recursive,
        delete_source: opts.delete_source,
        write_log: app.get_write_log(),
        max_scales: opts.max_scales,
    };
    let _ = settings.save();
}

fn parse_u32(value: &str, label: &str) -> std::result::Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("{}必须是数字", label))
}

fn parse_u8(value: &str, label: &str) -> std::result::Result<u8, String> {
    value
        .trim()
        .parse::<u8>()
        .map_err(|_| format!("{}必须是 1 到 100 之间的数字", label))
}

fn update_summary(ui: slint::Weak<MainWindow>, message: String, running: bool) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            ui.set_running(running);
            ui.set_summary(message.into());
        }
    });
}
