use crate::config::{CompressOptions, SizeLimit};
use crate::settings::AppSettings;
use crossbeam_channel::{Receiver, Sender};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct WorkerReporter(Sender<WorkerEvent>, Arc<AtomicBool>);

impl crate::progress::ProgressReporter for WorkerReporter {
    fn on_start(&self, total: usize) {
        let _ = self.0.send(WorkerEvent::Started { total });
    }
    fn on_file_start(&self, name: &std::path::Path) {
        let _ = self.0.send(WorkerEvent::FileStart { name: name.to_path_buf() });
    }
    fn on_file_done(&self, name: &std::path::Path, ok: bool, msg: Option<&str>) {
        let _ = self.0.send(WorkerEvent::FileDone {
            name: name.to_path_buf(),
            ok,
            msg: msg.map(String::from),
        });
    }
    fn on_finish(&self, report: &crate::pipeline::CompressReport) {
        let _ = self.0.send(WorkerEvent::Finished {
            report: report.clone(),
            cancelled: self.1.load(Ordering::Relaxed),
        });
    }
    fn is_cancelled(&self) -> bool {
        self.1.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub enum ScanResult {
    Done {
        count: usize,
        total_bytes: u64,
        cancelled: bool,
    },
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    Started { total: usize },
    FileStart { name: PathBuf },
    FileDone { name: PathBuf, ok: bool, msg: Option<String> },
    Finished { report: crate::pipeline::CompressReport, cancelled: bool },
}

pub struct AppState {
    pub settings: AppSettings,
    pub input_path: String,
    pub output_path: String,
    pub max_size_kb: u32,
    pub format: crate::config::Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub recursive: bool,
    pub delete_source: bool,
    pub write_log: bool,
    pub max_scales: u32,

    pub total: usize,
    pub processed: usize,
    pub failed_count: usize,
    pub current_file: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub scan_count: usize,
    pub scan_bytes: u64,
    pub scanning: bool,
    pub failed_files: Vec<(PathBuf, String)>,
    pub info_messages: Vec<String>,
    pub running: bool,
    pub started_at: Option<std::time::Instant>,
    pub finished_elapsed: Option<f64>,
    pub cancel_flag: Option<Arc<AtomicBool>>,

    pub rx: Option<Receiver<WorkerEvent>>,
    pub scan_rx: Option<Receiver<ScanResult>>,
    pub scan_handle: Option<std::thread::JoinHandle<()>>,
    pub scan_cancel: Option<Arc<AtomicBool>>,
    pub last_scanned_path: String,
}

impl AppState {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = AppSettings::load();
        let input_path = settings
            .last_input
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let output_path = settings
            .last_output
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut me = Self {
            max_size_kb: settings.max_size_kb,
            format: settings.format,
            min_quality: settings.min_quality,
            max_quality: settings.max_quality,
            scale_step: settings.scale_step,
            preserve_structure: settings.preserve_structure,
            skip_if_smaller: settings.skip_if_smaller,
            recursive: settings.recursive,
            delete_source: settings.delete_source,
            write_log: settings.write_log,
            max_scales: settings.max_scales,
            settings,
            input_path,
            output_path,
            total: 0,
            processed: 0,
            failed_count: 0,
            current_file: String::new(),
            bytes_in: 0,
            bytes_out: 0,
            scan_count: 0,
            scan_bytes: 0,
            scanning: false,
            failed_files: Vec::new(),
            info_messages: Vec::new(),
            running: false,
            started_at: None,
            finished_elapsed: None,
            cancel_flag: None,
            rx: None,
            scan_rx: None,
            scan_handle: None,
            scan_cancel: None,
            last_scanned_path: String::new(),
        };
        if !me.input_path.is_empty() {
            me.start_scan(me.input_path.clone());
        }
        me
    }

    pub fn start_scan(&mut self, path: String) {
        if path == self.last_scanned_path && !self.scanning {
            return;
        }
        if let Some(c) = &self.scan_cancel {
            c.store(true, Ordering::Relaxed);
        }
        self.last_scanned_path = path.clone();
        self.scanning = true;
        self.scan_count = 0;
        self.scan_bytes = 0;

        let (tx, rx) = crossbeam_channel::unbounded();
        self.scan_rx = Some(rx);

        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_cancel = Some(cancel.clone());

        let pb = PathBuf::from(path);
        if let Some(old) = self.scan_handle.take() {
            std::thread::spawn(move || { let _ = old.join(); });
        }
        self.scan_handle = Some(std::thread::spawn(move || {
            let (count, total_bytes) = scan_directory(&pb, &cancel);
            let _ = tx.send(ScanResult::Done {
                count,
                total_bytes,
                cancelled: cancel.load(Ordering::Relaxed),
            });
        }));
    }

    pub fn poll_scan(&mut self) {
        if let Some(rx) = self.scan_rx.as_ref() {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    ScanResult::Done { count, total_bytes, cancelled } => {
                        self.scan_count = count;
                        self.scan_bytes = total_bytes;
                        self.scanning = false;
                        self.last_scanned_path = self.input_path.clone();
                        if cancelled {
                            log::debug!("scan cancelled");
                        }
                    }
                }
            }
            if !self.scanning {
                self.scan_rx = None;
                self.scan_handle = None;
                self.scan_cancel = None;
            }
        }
    }

    pub fn start(&mut self) {
        if self.running {
            return;
        }
        let input = PathBuf::from(self.input_path.trim());
        let output = PathBuf::from(self.output_path.trim());
        if input.as_os_str().is_empty() || output.as_os_str().is_empty() {
            return;
        }

    let min_quality = self.min_quality.min(100);
    let max_quality = self.max_quality.min(100);
    if min_quality > max_quality {
        self.info_messages.push("质量范围无效: 最小值不能大于最大值".into());
        return;
    }

    let opts = CompressOptions {
        input: input.clone(),
        output: output.clone(),
        max_size: SizeLimit::from_kb(self.max_size_kb),
        format: self.format,
        min_quality,
        max_quality,
        scale_step: self.scale_step,
        max_scales: self.max_scales,
        preserve_structure: self.preserve_structure,
        skip_if_smaller: self.skip_if_smaller,
        recursive: self.recursive,
        delete_source: self.delete_source,
    };

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = Some(cancel.clone());
        self.running = true;
        self.processed = 0;
        self.failed_count = 0;
        self.bytes_in = 0;
        self.bytes_out = 0;
        self.current_file.clear();
        self.started_at = Some(std::time::Instant::now());
        self.finished_elapsed = None;
        self.failed_files.clear();
        self.info_messages.clear();

        let (tx, rx) = crossbeam_channel::unbounded();
        self.rx = Some(rx);

        let cancel_clone = cancel.clone();
        let input_thread = input.clone();
        let output_thread = output.clone();
        let opts_thread = opts.clone();
        let write_log = self.write_log;
        let tx_for_panic = tx.clone();
        let _handle = std::thread::spawn(move || {
            let reporter = WorkerReporter(tx, cancel_clone);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::pipeline::compress_directory(
                    &input_thread,
                    &output_thread,
                    &opts_thread,
                    &reporter,
                )
            }));
            match result {
                Ok(Ok(report)) => {
                    if write_log {
                        if let Some(path) = crate::log::log_file_path() {
                            if let Err(e) =
                                crate::log::write_log_file(&report, &opts_thread, &path)
                            {
                                log::warn!("写入日志失败: {}", e);
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    log::error!("pipeline error: {}", e);
                }
                Err(panic) => {
                    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                        format!("worker panic: {}", s)
                    } else if let Some(s) = panic.downcast_ref::<String>() {
                        format!("worker panic: {}", s)
                    } else {
                        "worker panic: <unknown>".to_string()
                    };
                    log::error!("{}", msg);
                    let _ = tx_for_panic.send(WorkerEvent::Finished {
                        report: crate::pipeline::CompressReport::default(),
                        cancelled: false,
                    });
                }
            }
        });

        let mut s = self.settings.clone();
        s.last_input = Some(input);
        s.last_output = Some(output);
        s.max_size_kb = self.max_size_kb;
        s.format = self.format;
        s.min_quality = self.min_quality;
        s.max_quality = self.max_quality;
        s.scale_step = self.scale_step;
        s.preserve_structure = self.preserve_structure;
        s.skip_if_smaller = self.skip_if_smaller;
        s.recursive = self.recursive;
        s.delete_source = self.delete_source;
        s.max_scales = self.max_scales;
        s.write_log = self.write_log;
        let _ = s.save();
        self.settings = s;
    }

    pub fn cancel(&mut self) {
        if let Some(f) = &self.cancel_flag {
            f.store(true, Ordering::Relaxed);
        }
    }

    pub fn poll_events(&mut self) {
        let events: Vec<WorkerEvent> = match self.rx.as_ref() {
            Some(rx) => rx.try_iter().collect(),
            None => return,
        };
        for ev in events {
            match ev {
                WorkerEvent::Started { total } => {
                    self.total = total;
                }
                WorkerEvent::FileStart { name } => {
                    self.current_file = name.display().to_string();
                }
                WorkerEvent::FileDone { name, ok, msg } => {
                    self.processed += 1;
                    if !ok {
                        self.failed_count += 1;
                        let m = msg.unwrap_or_else(|| "未知错误".into());
                        self.failed_files.push((name, m));
                    }
                }
                WorkerEvent::Finished { report, cancelled } => {
                    self.bytes_in = report.bytes_in;
                    self.bytes_out = report.bytes_out;
                    self.running = false;
                    self.current_file.clear();
                    let elapsed = self.started_at.map(|t| t.elapsed()).unwrap_or_default();
                    self.finished_elapsed = Some(elapsed.as_secs_f64());
                    self.rx = None;

                    use crate::source::SourceAction;
                    match &report.source_action {
                        SourceAction::NotRequested => {}
                        SourceAction::Deleted => {
                            self.info_messages.push("源文件已删除".into());
                        }
                        SourceAction::Skipped { reason } => {
                            self.info_messages
                                .push(format!("源文件未删除: {}", reason));
                        }
                        SourceAction::Errored { error } => {
                            self.info_messages
                                .push(format!("源文件删除失败: {}", error));
                        }
                    }
                    if cancelled {
                        self.info_messages.push("已取消".into());
                    }
                }
            }
        }
    }

    pub fn open_output(&mut self) {
        let path = PathBuf::from(self.output_path.trim());
        if path.as_os_str().is_empty() {
            return;
        }
        std::fs::create_dir_all(&path).ok();
        let _ = open::that(&path);
    }
}

fn scan_directory(dir: &Path, cancel: &Arc<AtomicBool>) -> (usize, u64) {
    let mut count = 0usize;
    let mut bytes = 0u64;
    scan_recursive(dir, &mut count, &mut bytes, cancel);
    (count, bytes)
}

fn scan_recursive(
    dir: &Path,
    count: &mut usize,
    bytes: &mut u64,
    cancel: &Arc<AtomicBool>,
) {
    if cancel.load(Ordering::Relaxed) {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("scan_recursive: read_dir({}) failed: {}", dir.display(), e);
            return;
        }
    };
    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let p = entry.path();
        if p.is_dir() {
            scan_recursive(&p, count, bytes, cancel);
        } else if crate::discovery::is_supported(&p) {
            *count += 1;
            if let Ok(meta) = std::fs::metadata(&p) {
                *bytes += meta.len();
            }
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_scan();
        self.poll_events();

        if self.running || self.scanning {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }

        let prev_input = self.input_path.clone();
        super::draw::draw_ui(ui, self);
        if self.input_path != prev_input {
            self.start_scan(self.input_path.clone());
        }
    }

    fn clear_color(&self, visuals: &eframe::egui::Visuals) -> [f32; 4] {
        visuals.window_fill.to_normalized_gamma_f32()
    }
}

