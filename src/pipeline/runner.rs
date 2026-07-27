use crate::config::{CompressOptions, Format};
use crate::discovery::collect_files;
use crate::pipeline::report::{CompressReport, OrganizeAction};
use crate::pipeline::task::{process_task, TaskOutcome};
use crate::progress::ProgressReporter;
use crate::source::SourceAction;
use crate::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct Accumulator {
    success: std::sync::atomic::AtomicUsize,
    bytes_in: std::sync::atomic::AtomicU64,
    bytes_out: std::sync::atomic::AtomicU64,
    failed: Mutex<Vec<(PathBuf, String)>>,
    run_log: Mutex<Vec<(usize, String)>>,
}

pub fn compress_directory(
    input: &Path,
    output: &Path,
    opts: &CompressOptions,
    progress: &dyn ProgressReporter,
) -> Result<CompressReport> {
    use crate::compressor::Compressor;

    std::fs::create_dir_all(output)?;
    let collection = collect_files(input, output, opts.recursive, opts.format)?;
    let tasks = collection.tasks;
    let total = tasks.len();
    let scan_failed_len = collection.scan_failed.len();
    progress.on_start(total);

    for task in &tasks {
        if let Some(parent) = task.output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let compressor = Compressor::new(match opts.format {
        Format::Jpeg => Box::new(crate::codec::jpeg::JpegCodec::new()),
        Format::WebP => Box::new(crate::codec::webp::WebPCodec::new()),
    });

    let mut initial_log = vec![(
        0,
        format!(
            "开始扫描，发现 {} 个文件，{} 个目录扫描失败",
            total, scan_failed_len
        ),
    )];
    initial_log.extend(
        collection
            .scan_failed
            .iter()
            .enumerate()
            .map(|(index, (path, msg))| {
                (index + 1, format!("扫描失败: {} - {}", path.display(), msg))
            }),
    );

    let acc = Accumulator {
        success: std::sync::atomic::AtomicUsize::new(0),
        bytes_in: std::sync::atomic::AtomicU64::new(0),
        bytes_out: std::sync::atomic::AtomicU64::new(0),
        failed: Mutex::new(Vec::new()),
        run_log: Mutex::new(initial_log),
    };

    tasks.par_iter().enumerate().for_each(|(index, task)| {
        if progress.is_cancelled() {
            return;
        }
        let log_index = scan_failed_len + index + 1;
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
                acc.success
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                acc.bytes_in
                    .fetch_add(in_size, std::sync::atomic::Ordering::Relaxed);
                let total_out: u64 = out_files.iter().map(|(_, sz)| sz).sum();
                acc.bytes_out
                    .fetch_add(total_out, std::sync::atomic::Ordering::Relaxed);
                let outputs = out_files
                    .iter()
                    .map(|(path, size)| format!("{} ({} bytes)", path.display(), size))
                    .collect::<Vec<_>>()
                    .join(", ");
                acc.run_log.lock().unwrap().push((
                    log_index,
                    format!("完成: {} -> {}", task.input.display(), outputs),
                ));
                progress.on_file_done(&task.input, true, None);
            }
            TaskOutcome::Failed { msg, in_size } => {
                if let Some(s) = in_size {
                    acc.bytes_in
                        .fetch_add(s, std::sync::atomic::Ordering::Relaxed);
                }
                acc.failed
                    .lock()
                    .unwrap()
                    .push((task.input.clone(), msg.clone()));
                acc.run_log.lock().unwrap().push((
                    log_index,
                    format!("失败: {} - {}", task.input.display(), msg),
                ));
                progress.on_file_done(&task.input, false, Some(&msg));
            }
            TaskOutcome::Cancelled => {
                acc.run_log
                    .lock()
                    .unwrap()
                    .push((log_index, format!("取消: {}", task.input.display())));
                progress.on_file_done(&task.input, false, Some("已取消"));
            }
        }
    });

    let mut failed = collection.scan_failed;
    failed.extend(acc.failed.into_inner().unwrap());
    let mut run_log = acc.run_log.into_inner().unwrap();
    run_log.sort_by_key(|(index, _)| *index);
    let run_log = run_log
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>();

    let mut final_report = CompressReport {
        total,
        success: acc.success.load(std::sync::atomic::Ordering::Relaxed),
        failed,
        bytes_in: acc.bytes_in.load(std::sync::atomic::Ordering::Relaxed),
        bytes_out: acc.bytes_out.load(std::sync::atomic::Ordering::Relaxed),
        source_action: SourceAction::NotRequested,
        organize_action: OrganizeAction::NotRequested,
        run_log,
    };

    update_source_action(input, output, opts, progress, total, &mut final_report);
    update_organize_action(output, opts, total, &mut final_report);

    progress.on_finish(&final_report);
    Ok(final_report)
}

fn update_organize_action(
    output: &Path,
    opts: &CompressOptions,
    total: usize,
    report: &mut CompressReport,
) {
    if !opts.organize_after_success {
        return;
    }

    if !report.failed.is_empty() {
        report.organize_action = OrganizeAction::Skipped {
            reason: format!("有 {} 个文件失败", report.failed.len()),
        };
    } else if report.success != total {
        report.organize_action = OrganizeAction::Skipped {
            reason: "部分任务被取消".into(),
        };
    } else if report.success == 0 {
        report.organize_action = OrganizeAction::Skipped {
            reason: "没有成功处理的文件".into(),
        };
    } else {
        match crate::output::organize_output_root_by_name(output) {
            Ok(result) => {
                report.run_log.push(format!(
                    "整理输出文件: 移动 {} 个，跳过 {} 个",
                    result.moved, result.skipped
                ));
                report.organize_action = OrganizeAction::Organized {
                    moved: result.moved,
                    skipped: result.skipped,
                };
            }
            Err(e) => {
                report.run_log.push(format!("整理输出文件失败: {}", e));
                report.organize_action = OrganizeAction::Errored {
                    error: e.to_string(),
                };
            }
        }
    }
}

fn update_source_action(
    input: &Path,
    output: &Path,
    opts: &CompressOptions,
    progress: &dyn ProgressReporter,
    total: usize,
    report: &mut CompressReport,
) {
    if !opts.delete_source {
        return;
    }

    if !report.failed.is_empty() {
        report.source_action = SourceAction::Skipped {
            reason: format!("有 {} 个文件失败", report.failed.len()),
        };
    } else if report.success != total {
        report.source_action = SourceAction::Skipped {
            reason: "部分任务被取消".into(),
        };
    } else if report.success == 0 {
        report.source_action = SourceAction::Skipped {
            reason: "没有成功处理的文件".into(),
        };
    } else if !progress.confirm_delete_source(input) {
        report.source_action = SourceAction::Skipped {
            reason: "用户取消删除源文件".into(),
        };
    } else {
        match crate::source::delete_source(input, output) {
            Ok(()) => {
                log::info!("已删除源: {}", input.display());
                report.source_action = SourceAction::Deleted;
            }
            Err(e) => {
                log::warn!("删除源文件失败 ({}): {}", input.display(), e);
                report.source_action = SourceAction::Errored {
                    error: e.to_string(),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "imgpress_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn compress_directory_reports_scan_failures() {
        let input = temp_path("missing_scan_input");
        let output = temp_path("missing_scan_output");
        let opts = CompressOptions {
            input: input.clone(),
            output: output.clone(),
            ..CompressOptions::default()
        };

        let report =
            compress_directory(&input, &output, &opts, &crate::progress::CliProgress).unwrap();

        assert_eq!(report.total, 0);
        assert_eq!(report.success, 0);
        assert_eq!(report.failed.len(), 1);
        assert!(report.run_log.iter().any(|line| line.contains("扫描失败")));

        let _ = std::fs::remove_dir_all(&output);
    }
}
