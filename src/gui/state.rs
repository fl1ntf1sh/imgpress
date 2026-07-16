use crate::settings::AppSettings;
use crate::config::{CompressOptions, Format, SizeLimit};
use crossbeam_channel::{Receiver, Sender};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::widgets;

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
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub recursive: bool,
    pub delete_source: bool,

    pub total: usize,
    pub processed: usize,
    pub failed_count: usize,
    pub current_file: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub scan_count: usize,
    pub scan_bytes: u64,
    pub scanning: bool,
    pub log_lines: Vec<String>,
    pub running: bool,
    pub started_at: Option<std::time::Instant>,
    pub finished_elapsed: Option<f64>,
    pub cancel_flag: Option<Arc<AtomicBool>>,

    pub rx: Option<Receiver<WorkerEvent>>,
    pub worker_handle: Option<std::thread::JoinHandle<()>>,
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
            log_lines: Vec::new(),
            running: false,
            started_at: None,
            finished_elapsed: None,
            cancel_flag: None,
            rx: None,
            worker_handle: None,
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

        let (tx, rx): (Sender<ScanResult>, Receiver<ScanResult>) = crossbeam_channel::unbounded();
        self.scan_rx = Some(rx);

        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_cancel = Some(cancel.clone());

        let pb = PathBuf::from(path);
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
            self.log("错误: 必须设置源文件夹和输出文件夹");
            return;
        }

        let opts = CompressOptions {
            input: input.clone(),
            output: output.clone(),
            max_size: SizeLimit::from_kb(self.max_size_kb),
            format: self.format,
            min_quality: self.min_quality,
            max_quality: self.max_quality,
            scale_step: self.scale_step,
            max_scales: 8,
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
        self.log(&format!(
            "开始: {} → {} (目标 {} KB)",
            input.display(),
            output.display(),
            self.max_size_kb
        ));

        let (tx, rx): (Sender<WorkerEvent>, Receiver<WorkerEvent>) = crossbeam_channel::unbounded();
        self.rx = Some(rx);

        let cancel_clone = cancel.clone();
        let input_thread = input.clone();
        let output_thread = output.clone();
        let handle = std::thread::spawn(move || {
            struct Reporter(Sender<WorkerEvent>, Arc<AtomicBool>);
            impl crate::progress::ProgressReporter for Reporter {
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
            let reporter = Reporter(tx, cancel_clone);
            let _ = crate::pipeline::compress_directory(&input_thread, &output_thread, &opts, &reporter);
        });
        self.worker_handle = Some(handle);

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
        let _ = s.save();
        self.settings = s;
    }

    pub fn cancel(&mut self) {
        if let Some(f) = &self.cancel_flag {
            f.store(true, Ordering::Relaxed);
            self.log("正在取消...");
        }
    }

    pub fn poll_events(&mut self) {
        let events: Vec<WorkerEvent> = match self.rx.as_ref() {
            Some(rx) => rx.try_recv().into_iter().collect(),
            None => return,
        };
        for ev in events {
            match ev {
                WorkerEvent::Started { total } => {
                    self.total = total;
                    self.log(&format!("找到 {} 个文件", total));
                }
                WorkerEvent::FileStart { name } => {
                    self.current_file = name.display().to_string();
                }
                WorkerEvent::FileDone { name, ok, msg } => {
                    self.processed += 1;
                    if ok {
                        if self.processed % 10 == 0 || self.processed == self.total {
                            self.log(&format!("✓ {}/{}  {}", self.processed, self.total, short_path(&name)));
                        }
                    } else {
                        self.failed_count += 1;
                        let m = msg.unwrap_or_else(|| "未知错误".into());
                        self.log(&format!("✗ {} - {}", short_path(&name), m));
                    }
                }
                WorkerEvent::Finished { report, cancelled } => {
                    self.bytes_in = report.bytes_in;
                    self.bytes_out = report.bytes_out;
                    self.running = false;
                    self.current_file.clear();
                    let elapsed = self.started_at.map(|t| t.elapsed()).unwrap_or_default();
                    self.finished_elapsed = Some(elapsed.as_secs_f64());
                    let status = if cancelled { "已取消" } else { "完成" };
                    self.log(&format!(
                        "{}: 总 {} · 成功 {} · 失败 {} · 用时 {:?}",
                        status,
                        report.total,
                        report.success,
                        report.failed.len(),
                        elapsed
                    ));
                    self.rx = None;
                }
            }
        }
    }

    pub fn log(&mut self, line: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 86_400)
            .unwrap_or(0);
        let h = now / 3600;
        let m = (now % 3600) / 60;
        let s = now % 60;
        self.log_lines
            .push(format!("[{:02}:{:02}:{:02}] {}", h, m, s, line));
        if self.log_lines.len() > 300 {
            let drop = self.log_lines.len() - 300;
            self.log_lines.drain(0..drop);
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

fn short_path(p: &Path) -> String {
    let s = p.display().to_string();
    if s.len() > 60 {
        format!("...{}", &s[s.len() - 57..])
    } else {
        s
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
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let p = entry.path();
        if p.is_dir() {
            scan_recursive(&p, count, bytes, cancel);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase();
            if matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif" | "gif" | "ico"
            ) {
                *count += 1;
                if let Ok(meta) = std::fs::metadata(&p) {
                    *bytes += meta.len();
                }
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
        draw_ui(ui, self);
        if self.input_path != prev_input {
            self.start_scan(self.input_path.clone());
        }
    }

    fn clear_color(&self, visuals: &eframe::egui::Visuals) -> [f32; 4] {
        visuals.window_fill.to_normalized_gamma_f32()
    }
}

fn draw_ui(ui: &mut eframe::egui::Ui, state: &mut AppState) {
    use eframe::egui::{Align, Color32, RichText, Vec2};

    eframe::egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("imgpress")
                                .size(22.0)
                                .strong()
                                .color(Color32::from_rgb(100, 160, 240)),
                        );
                        ui.label(
                            RichText::new("图片压缩 · 精确控制目标大小")
                                .color(Color32::from_gray(140))
                                .size(12.0),
                        );
                    });
                });
                ui.add_space(4.0);

                widgets::section(ui, "路径", "📁", |ui| {
                    widgets::path_row(ui, "源文件夹", &mut state.input_path, true);
                    if state.scanning {
                        widgets::info_line(ui, "🔍", "正在扫描...", None);
                    } else if state.scan_count > 0 {
                        widgets::info_line(
                            ui,
                            "📊",
                            &format!(
                                "{} 个文件 · {}",
                                state.scan_count,
                                widgets::format_bytes(state.scan_bytes)
                            ),
                            Some(Color32::from_rgb(120, 180, 140)),
                        );
                    } else if !state.input_path.trim().is_empty() {
                        widgets::info_line(ui, "⚠", "未找到支持的图片文件", Some(Color32::from_rgb(220, 170, 80)));
                    }
                    ui.add_space(4.0);
                    widgets::path_row(ui, "输出文件夹", &mut state.output_path, true);
                    widgets::info_line(
                        ui,
                        "💡",
                        if state.preserve_structure {
                            "将保留源目录的子文件夹结构"
                        } else {
                            "所有文件将扁平化到输出根目录"
                        },
                        None,
                    );
                });

                widgets::section(ui, "压缩参数", "🖼", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("目标大小").size(12.0));
                        ui.add(
                            eframe::egui::DragValue::new(&mut state.max_size_kb)
                                .range(1..=50_000)
                                .suffix(" KB")
                                .max_decimals(0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("输出格式").size(12.0));
                        ui.radio_value(&mut state.format, Format::Jpeg, "JPEG");
                        ui.radio_value(&mut state.format, Format::WebP, "WebP");
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("质量范围").size(12.0));
                        ui.add(
                            eframe::egui::DragValue::new(&mut state.min_quality)
                                .range(1..=100),
                        );
                        ui.add_sized(
                            [100.0, 16.0],
                            eframe::egui::Slider::new(&mut state.min_quality, 1..=state.max_quality)
                                .show_value(false),
                        );
                        ui.label("—");
                        ui.add_sized(
                            [100.0, 16.0],
                            eframe::egui::Slider::new(
                                &mut state.max_quality,
                                state.min_quality..=100,
                            )
                            .show_value(false),
                        );
                        ui.add(
                            eframe::egui::DragValue::new(&mut state.max_quality)
                                .range(state.min_quality..=100),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("缩放步长").size(12.0));
                        ui.add(
                            eframe::egui::DragValue::new(&mut state.scale_step)
                                .range(0.1..=0.99)
                                .max_decimals(2),
                        );
                    });
                });

                widgets::section(ui, "选项", "⚙", |ui| {
                    ui.checkbox(
                        &mut state.preserve_structure,
                        "保留源目录的子文件夹结构",
                    );
                    ui.checkbox(
                        &mut state.skip_if_smaller,
                        "已小于目标的文件直接复制（不再压缩）",
                    );
                    ui.checkbox(
                        &mut state.recursive,
                        "递归扫描子文件夹",
                    );
                    ui.checkbox(
                        &mut state.delete_source,
                        "全部成功后删除源文件（不可恢复）",
                    );
                });

                widgets::section(ui, "进度", "📊", |ui| {
                    let frac = if state.total > 0 {
                        (state.processed as f32 / state.total as f32).min(1.0)
                    } else {
                        0.0
                    };
                    ui.add(
                        eframe::egui::ProgressBar::new(frac)
                            .desired_width(ui.available_width() - 16.0)
                            .show_percentage()
                            .animate(true),
                    );
                    ui.add_space(4.0);

                    let elapsed = state
                        .finished_elapsed
                        .or_else(|| state.started_at.map(|t| t.elapsed().as_secs_f64()))
                        .unwrap_or(0.0);
                    let rate = if elapsed > 0.5 {
                        state.processed as f64 / elapsed
                    } else {
                        0.0
                    };
                    let remaining_secs = if state.running
                        && rate > 0.01
                        && state.total > state.processed
                    {
                        (state.total - state.processed) as f64 / rate
                    } else {
                        0.0
                    };

                    let stats: Vec<(&str, String)> = vec![
                        ("完成", format!("{} / {}", state.processed, state.total)),
                        ("失败", state.failed_count.to_string()),
                        ("用时", widgets::format_duration(elapsed)),
                        ("速率", if rate > 0.01 { format!("{:.1} 张/秒", rate) } else { "--".into() }),
                        ("预计剩余", widgets::format_duration(remaining_secs)),
                    ];
                    let stats_ref: Vec<(&str, &str)> = stats
                        .iter()
                        .map(|(k, v)| (*k, v.as_str()))
                        .collect();
                    widgets::stat_row(ui, &stats_ref);

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("输入")
                                .color(Color32::from_gray(140))
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(widgets::format_bytes(state.bytes_in))
                                .strong()
                                .size(13.0),
                        );
                        ui.label(
                            RichText::new("→ 输出")
                                .color(Color32::from_gray(140))
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(widgets::format_bytes(state.bytes_out))
                                .strong()
                                .size(13.0),
                        );
                        if state.bytes_in > 0 {
                            let ratio =
                                (state.bytes_out as f64 / state.bytes_in as f64) * 100.0;
                            let saved = 100.0 - ratio;
                            ui.label(
                                RichText::new(format!("(节省 {:.0}%)", saved))
                                    .color(Color32::from_rgb(120, 200, 140))
                                    .size(12.0),
                            );
                        }
                    });

                    if !state.current_file.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("当前")
                                    .color(Color32::from_gray(140))
                                    .size(12.0),
                            );
                            ui.label(
                                RichText::new(short_path(Path::new(&state.current_file)))
                                    .size(12.0)
                                    .color(Color32::from_rgb(220, 220, 220)),
                            );
                        });
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let start_ok = !state.running
                        && !state.input_path.trim().is_empty()
                        && !state.output_path.trim().is_empty();
                    if widgets::primary_button(ui, "▶  开始", start_ok).clicked() {
                        state.start();
                    }
                    ui.add_space(6.0);
                    if widgets::danger_button(ui, "■  取消", state.running).clicked() {
                        state.cancel();
                    }
                    ui.add_space(6.0);
                    let open_ok = !state.output_path.trim().is_empty();
                    if ui
                        .add_enabled(open_ok, eframe::egui::Button::new("📂  打开输出").min_size(Vec2::new(120.0, 28.0)))
                        .clicked()
                    {
                        state.open_output();
                    }
                });

                ui.add_space(8.0);
                eframe::egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("📝  日志 ({} 条)", state.log_lines.len()))
                                    .strong()
                                    .size(13.0),
                            );
                            ui.with_layout(eframe::egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("清空").clicked() {
                                    state.log_lines.clear();
                                }
                            });
                        });
                        ui.add_space(2.0);
                        eframe::egui::ScrollArea::vertical()
                            .max_height(80.0)
                            .stick_to_bottom(true)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for line in &state.log_lines {
                                    ui.label(
                                        RichText::new(line)
                                            .monospace()
                                            .size(11.5)
                                            .color(Color32::from_gray(200)),
                                    );
                                }
                            });
                    });
                ui.add_space(8.0);
            });
}