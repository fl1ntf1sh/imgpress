use crate::Result;
use crate::discovery::{collect_files, FileTask};
use crate::source::SourceAction;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
                msg: crate::error::format_panic(&panic),
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
            match crate::source::delete_source(input, output) {
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

    let ext = crate::discovery::output_extension(opts.format);
    let out_files: Vec<(PathBuf, u64)> = pages
        .par_iter()
        .enumerate()
        .filter_map(|(idx, img)| {
            if progress.is_cancelled() {
                return None;
            }
            let target_name = format!("{}_page{}.{}", stem, idx + 1, ext);
            let out_path = crate::discovery::unique_in_dir(parent, &target_name);
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
                .map_err(|e| {
                    log::warn!("压缩 {} 失败: {}", out_path.display(), e);
                    e
                })
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
