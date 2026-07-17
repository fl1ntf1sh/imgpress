use crate::app_data_dir;
use crate::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct FileTask {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn collect_files(
    input: &Path,
    output: &Path,
    recursive: bool,
    preserve_structure: bool,
    format: crate::config::Format,
) -> Result<Vec<FileTask>> {
    let mut tasks = Vec::new();
    if input.is_file() {
        if is_supported(input) {
            let out = unique_output(output, input, preserve_structure, format);
            tasks.push(FileTask {
                input: input.to_path_buf(),
                output: out,
            });
        }
        return Ok(tasks);
    }

    walk(input, input, output, recursive, preserve_structure, format, &mut tasks)?;
    Ok(tasks)
}

fn walk(
    input_root: &Path,
    dir: &Path,
    output_root: &Path,
    recursive: bool,
    preserve_structure: bool,
    format: crate::config::Format,
    tasks: &mut Vec<FileTask>,
) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("read_dir({}) failed: {}", dir.display(), e);
            return Ok(());
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                walk(input_root, &path, output_root, recursive, preserve_structure, format, tasks)?;
            }
            continue;
        }

        if !is_supported(&path) {
            continue;
        }

        let rel = path.strip_prefix(input_root).unwrap_or(&path);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let ext = output_extension(format);

        let (out_dir, target_name) = if preserve_structure {
            let parent_rel = rel.parent().unwrap_or(Path::new(""));
            let d = if parent_rel.as_os_str().is_empty() {
                output_root.to_path_buf()
            } else {
                output_root.join(parent_rel)
            };
            (d, format!("{}.{}", stem, ext))
        } else {
            let prefix = flatten_prefix(rel);
            let name = if prefix.is_empty() {
                format!("{}.{}", stem, ext)
            } else {
                format!("{}_{}.{}", prefix, stem, ext)
            };
            (output_root.to_path_buf(), name)
        };

        let out_path = unique_in_dir(&out_dir, &target_name);
        tasks.push(FileTask {
            input: path,
            output: out_path,
        });
    }
    Ok(())
}

fn flatten_prefix(rel: &Path) -> String {
    rel.parent()
        .unwrap_or(Path::new(""))
        .iter()
        .filter_map(|c| c.to_str())
        .collect::<Vec<_>>()
        .join("_")
}

fn unique_in_dir(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = Path::new(name).extension().and_then(|s| s.to_str()).unwrap_or("jpg");
    for i in 1..u16::MAX {
        let new_name = format!("{}_{}.{}", stem, i, ext);
        let p = dir.join(&new_name);
        if !p.exists() {
            return p;
        }
    }
    log::warn!("unique_in_dir exhausted: {}, fallback to original", dir.join(name).display());
    candidate
}

fn unique_output(
    output_root: &Path,
    input: &Path,
    preserve_structure: bool,
    format: crate::config::Format,
) -> PathBuf {
    let ext = output_extension(format);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");

    let (out_dir, name) = if preserve_structure {
        let parent = input.parent().unwrap_or(Path::new(""));
        let d = if parent.as_os_str().is_empty() {
            output_root.to_path_buf()
        } else {
            output_root.join(parent)
        };
        (d, format!("{}.{}", stem, ext))
    } else {
        let prefix = flatten_prefix(input);
        let name = if prefix.is_empty() {
            format!("{}.{}", stem, ext)
        } else {
            format!("{}_{}.{}", prefix, stem, ext)
        };
        (output_root.to_path_buf(), name)
    };

    unique_in_dir(&out_dir, &name)
}

fn output_extension(format: crate::config::Format) -> &'static str {
    format.extension()
}

const SUPPORTED_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tiff", "tif", "gif", "ico",
    "ppm", "pgm", "pbm", "pdf",
];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| SUPPORTED_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext)))
}

#[derive(Debug, Clone, Default)]
pub enum SourceAction {
    #[default]
    NotRequested,
    Deleted,
    Skipped { reason: String },
    Errored { error: String },
}

#[derive(Debug, Clone, Default)]
pub struct CompressReport {
    pub total: usize,
    pub success: usize,
    pub failed: Vec<(PathBuf, String)>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub source_action: SourceAction,
}

enum TaskOutcome {
    Ok { in_size: u64, out_files: Vec<(PathBuf, u64)> },
    Failed { msg: String, in_size: Option<u64> },
    Cancelled,
}

struct Accumulator {
    success: std::sync::atomic::AtomicUsize,
    bytes_in: std::sync::atomic::AtomicU64,
    bytes_out: std::sync::atomic::AtomicU64,
    failed: Mutex<Vec<(PathBuf, String)>>,
}

pub fn compress_directory(
    input: &Path,
    output: &Path,
    opts: &crate::config::CompressOptions,
    progress: &dyn crate::progress::ProgressReporter,
) -> Result<CompressReport> {
    use crate::compressor::Compressor;

    std::fs::create_dir_all(output)?;
    let tasks = collect_files(input, output, opts.recursive, opts.preserve_structure, opts.format)?;
    let total = tasks.len();
    progress.on_start(total);

    for task in &tasks {
        if let Some(parent) = task.output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let compressor = Compressor::new(match opts.format {
        crate::config::Format::Jpeg => Box::new(crate::codec::jpeg::JpegCodec::new()),
        crate::config::Format::WebP => Box::new(crate::codec::webp::WebPCodec::new()),
    });

    let acc = Accumulator {
        success: std::sync::atomic::AtomicUsize::new(0),
        bytes_in: std::sync::atomic::AtomicU64::new(0),
        bytes_out: std::sync::atomic::AtomicU64::new(0),
        failed: Mutex::new(Vec::new()),
    };

    tasks.par_iter().for_each(|task| {
        if progress.is_cancelled() {
            return;
        }
        progress.on_file_start(&task.input);

        let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            process_task(task, opts, &compressor, progress)
        })) {
            Ok(outcome) => outcome,
            Err(panic) => TaskOutcome::Failed {
                msg: panic_message(&panic),
                in_size: None,
            },
        };
        match outcome {
            TaskOutcome::Ok { in_size, out_files } => {
                acc.success.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                acc.bytes_in.fetch_add(in_size, std::sync::atomic::Ordering::Relaxed);
                let total_out: u64 = out_files.iter().map(|(_, sz)| sz).sum();
                acc.bytes_out.fetch_add(total_out, std::sync::atomic::Ordering::Relaxed);
                progress.on_file_done(&task.input, true, None);
            }
            TaskOutcome::Failed { msg, in_size } => {
                if let Some(s) = in_size {
                    acc.bytes_in.fetch_add(s, std::sync::atomic::Ordering::Relaxed);
                }
                acc.failed.lock().unwrap().push((task.input.clone(), msg.clone()));
                progress.on_file_done(&task.input, false, Some(&msg));
            }
                TaskOutcome::Cancelled => {
                    progress.on_file_done(&task.input, false, Some("已取消"));
                }
            }
        });

    let mut final_report = CompressReport {
        total,
        success: acc.success.load(std::sync::atomic::Ordering::Relaxed),
        failed: acc.failed.into_inner().unwrap(),
        bytes_in: acc.bytes_in.load(std::sync::atomic::Ordering::Relaxed),
        bytes_out: acc.bytes_out.load(std::sync::atomic::Ordering::Relaxed),
        source_action: SourceAction::NotRequested,
    };

    if opts.delete_source {
        if !final_report.failed.is_empty() {
            final_report.source_action = SourceAction::Skipped {
                reason: format!("有 {} 个文件失败", final_report.failed.len()),
            };
        } else if final_report.success != total {
            final_report.source_action = SourceAction::Skipped {
                reason: "部分任务被取消".into(),
            };
        } else if final_report.success == 0 {
            final_report.source_action = SourceAction::Skipped {
                reason: "没有成功处理的文件".into(),
            };
        } else {
            match delete_source(input, output) {
                Ok(()) => {
                    log::info!("已删除源: {}", input.display());
                    final_report.source_action = SourceAction::Deleted;
                }
                Err(e) => {
                    log::warn!("删除源文件失败 ({}): {}", input.display(), e);
                    final_report.source_action = SourceAction::Errored {
                        error: e.to_string(),
                    };
                }
            }
        }
    }

    progress.on_finish(&final_report);
    Ok(final_report)
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        format!("处理过程中发生 panic: {}", s)
    } else if let Some(s) = panic.downcast_ref::<String>() {
        format!("处理过程中发生 panic: {}", s)
    } else {
        "处理过程中发生未知 panic".to_string()
    }
}

pub fn log_file_path() -> Option<PathBuf> {
    Some(app_data_dir()?.join("log.txt"))
}

pub fn write_log_file(
    report: &CompressReport,
    opts: &crate::config::CompressOptions,
    path: &Path,
) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (date, time) = format_unix_time(now);
    writeln!(f)?;
    writeln!(f, "=== Run {} {} ===", date, time)?;
    writeln!(f, "Input:       {}", opts.input.display())?;
    writeln!(f, "Output:      {}", opts.output.display())?;
    writeln!(f, "Target:      {} KB", opts.max_size.bytes / 1024)?;
    writeln!(f, "Format:      {:?}", opts.format)?;
    writeln!(f, "Quality:     {}-{}", opts.min_quality, opts.max_quality)?;
    writeln!(f, "Scale step:  {}", opts.scale_step)?;
    writeln!(f, "Structure:   {}", opts.preserve_structure)?;
    writeln!(f, "Skip small:  {}", opts.skip_if_smaller)?;
    writeln!(f, "Recursive:   {}", opts.recursive)?;
    writeln!(f, "Delete src:  {}", opts.delete_source)?;
    writeln!(f)?;
    writeln!(f, "Total:       {}", report.total)?;
    writeln!(f, "Success:     {}", report.success)?;
    writeln!(f, "Failed:      {}", report.failed.len())?;
    writeln!(f, "Bytes in:    {}", report.bytes_in)?;
    writeln!(f, "Bytes out:   {}", report.bytes_out)?;
    writeln!(f, "Source:")?;
    match &report.source_action {
        SourceAction::NotRequested => {
            writeln!(f, "  not requested")?;
        }
        SourceAction::Deleted => {
            writeln!(f, "  deleted")?;
        }
        SourceAction::Skipped { reason } => {
            writeln!(f, "  skipped: {}", reason)?;
        }
        SourceAction::Errored { error } => {
            writeln!(f, "  error: {}", error)?;
        }
    }
    if !report.failed.is_empty() {
        writeln!(f)?;
        writeln!(f, "--- Failed files ---")?;
        for (path, msg) in &report.failed {
            writeln!(f, "{} - {}", path.display(), msg)?;
        }
    }
    f.flush()
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn format_unix_time(secs: u64) -> (String, String) {
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let diy = if is_leap_year(year) { 366 } else { 365 };
        if remaining < diy {
            break;
        }
        remaining -= diy;
        year += 1;
    }
    let dims: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &d in &dims {
        if remaining < d {
            break;
        }
        remaining -= d;
        month += 1;
    }
    let day = remaining + 1;
    (
        format!("{:04}-{:02}-{:02}", year, month, day),
        format!("{:02}:{:02}:{:02}", h, m, s),
    )
}

fn delete_source(input: &Path, output: &Path) -> std::io::Result<()> {
    if input.is_file() {
        return std::fs::remove_file(input);
    }
    if !input.is_dir() {
        return Ok(());
    }

    let skip_path = std::fs::canonicalize(output)
        .ok()
        .filter(|out| {
            std::fs::canonicalize(input)
                .map(|inp| out.starts_with(&inp))
                .unwrap_or(false)
        });

    for entry in std::fs::read_dir(input)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ref skip) = skip_path {
            if std::fs::canonicalize(&path)
                .map(|p| p.starts_with(skip))
                .unwrap_or(false)
            {
                log::info!("跳过输出目录: {}", path.display());
                continue;
            }
        }
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(e) = result {
            log::warn!("删除 {} 失败: {}", path.display(), e);
        }
    }
    Ok(())
}

fn process_task(
    task: &FileTask,
    opts: &crate::config::CompressOptions,
    compressor: &crate::compressor::Compressor,
    progress: &dyn crate::progress::ProgressReporter,
) -> TaskOutcome {
    if progress.is_cancelled() {
        return TaskOutcome::Cancelled;
    }

    let in_meta = match std::fs::metadata(&task.input) {
        Ok(m) => m,
        Err(e) => {
            return TaskOutcome::Failed {
                msg: format!("读取文件信息失败: {}", e),
                in_size: None,
            };
        }
    };
    let in_size = in_meta.len();

    let ext = task
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext == "pdf" {
        return process_pdf(task, opts, compressor, progress, in_size);
    }

    let img = match image::open(&task.input) {
        Ok(i) => i,
        Err(e) => {
            return TaskOutcome::Failed {
                msg: format!("解码失败: {}", e),
                in_size: Some(in_size),
            };
        }
    };

    if opts.skip_if_smaller && in_size <= opts.max_size.bytes {
        if let Err(e) = std::fs::copy(&task.input, &task.output) {
            return TaskOutcome::Failed {
                msg: format!("复制失败: {}", e),
                in_size: Some(in_size),
            };
        }
        return TaskOutcome::Ok {
            in_size,
            out_files: vec![(task.output.clone(), in_size)],
        };
    }

    let compressed = match compressor.compress_to_size(
        &img,
        opts.max_size.bytes,
        opts.min_quality,
        opts.max_quality,
        opts.scale_step,
        opts.max_scales,
        progress,
    ) {
        Ok(b) => b,
        Err(crate::Error::Cancelled) => return TaskOutcome::Cancelled,
        Err(e) => {
            return TaskOutcome::Failed {
                msg: e.to_string(),
                in_size: Some(in_size),
            };
        }
    };

    if let Err(e) = std::fs::write(&task.output, &compressed) {
        return TaskOutcome::Failed {
            msg: format!("写入失败: {}", e),
            in_size: Some(in_size),
        };
    }

    TaskOutcome::Ok {
        in_size,
        out_files: vec![(task.output.clone(), compressed.len() as u64)],
    }
}

fn process_pdf(
    task: &FileTask,
    opts: &crate::config::CompressOptions,
    compressor: &crate::compressor::Compressor,
    progress: &dyn crate::progress::ProgressReporter,
    in_size: u64,
) -> TaskOutcome {
    let pages = match crate::pdf::render_pdf_pages(&task.input) {
        Ok(p) => p,
        Err(e) => {
            return TaskOutcome::Failed {
                msg: format!("PDF 解析: {}", e),
                in_size: Some(in_size),
            };
        }
    };

    if pages.is_empty() {
        return TaskOutcome::Failed {
            msg: "PDF 没有可提取的页面".into(),
            in_size: Some(in_size),
        };
    }

    let stem = task
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let parent = task.output.parent().unwrap_or(&task.output);
    if let Err(e) = std::fs::create_dir_all(parent) {
        return TaskOutcome::Failed {
            msg: format!("无法创建输出目录: {}", e),
            in_size: Some(in_size),
        };
    }

    let ext = output_extension(opts.format);
    let out_files: Vec<(PathBuf, u64)> = pages
        .par_iter()
        .enumerate()
        .filter_map(|(idx, img)| {
            if progress.is_cancelled() {
                return None;
            }
            let target_name = format!("{}_page{}.{}", stem, idx + 1, ext);
            let out_path = unique_in_dir(parent, &target_name);
            let compressed = compressor
                .compress_to_size(
                    img,
                    opts.max_size.bytes,
                    opts.min_quality,
                    opts.max_quality,
                    opts.scale_step,
                    opts.max_scales,
                    progress,
                )
                .ok()?;
            if let Err(e) = std::fs::write(&out_path, &compressed) {
                log::warn!("写入 {} 失败: {}", out_path.display(), e);
                return None;
            }
            Some((out_path, compressed.len() as u64))
        })
        .collect();

    if progress.is_cancelled() {
        return TaskOutcome::Cancelled;
    }

    let total_pages = pages.len();
    let failures = total_pages - out_files.len();
    if failures > 0 {
        log::warn!(
            "{}: {}/{} 页面失败",
            task.input.display(),
            failures,
            total_pages
        );
    }

    if out_files.is_empty() {
        return TaskOutcome::Failed {
            msg: format!("PDF 所有 {} 个页面均失败", failures),
            in_size: Some(in_size),
        };
    }

    TaskOutcome::Ok {
        in_size,
        out_files,
    }
}