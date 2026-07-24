use crate::config::CompressOptions;
use crate::discovery::FileTask;
use crate::progress::ProgressReporter;
use std::path::PathBuf;

pub(super) enum TaskOutcome {
    Ok {
        in_size: u64,
        out_files: Vec<(PathBuf, u64)>,
    },
    Failed {
        msg: String,
        in_size: Option<u64>,
    },
    Cancelled,
}

pub(super) fn process_task(
    task: &FileTask,
    opts: &CompressOptions,
    compressor: &crate::compressor::Compressor,
    progress: &dyn ProgressReporter,
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

    if opts.skip_if_smaller
        && in_size <= opts.max_size.bytes
        && !crate::input::is_multi_image_input(&task.input)
    {
        if let Err(e) = std::fs::copy(&task.input, &task.output) {
            return TaskOutcome::Failed {
                msg: format!("复制失败: {}", e),
                in_size: Some(in_size),
            };
        }
        progress.on_file_progress(&task.input, 1, 1);
        return TaskOutcome::Ok {
            in_size,
            out_files: vec![(task.output.clone(), in_size)],
        };
    }

    let images = match crate::input::extract_images(&task.input) {
        Ok(images) => images,
        Err(e) => {
            return TaskOutcome::Failed {
                msg: processing_error_message(e),
                in_size: Some(in_size),
            };
        }
    };

    if images.is_empty() {
        return TaskOutcome::Failed {
            msg: "没有可压缩的图片".into(),
            in_size: Some(in_size),
        };
    }

    let total_images = images.len();
    let out_files: Vec<(PathBuf, u64)> = images
        .iter()
        .enumerate()
        .filter_map(|(index, img)| {
            if progress.is_cancelled() {
                return None;
            }
            let out_path = crate::output::path_for(task, &img.label, opts);
            if let Some(parent) = out_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::warn!("创建输出目录 {} 失败: {}", parent.display(), e);
                    progress.on_file_progress(&task.input, index + 1, total_images);
                    return None;
                }
            }
            let Some(compressed) = compressor
                .compress_to_size(&img.image, opts, progress)
                .map_err(|e| {
                    log::warn!("压缩 {} 失败: {}", out_path.display(), e);
                    e
                })
                .ok()
            else {
                progress.on_file_progress(&task.input, index + 1, total_images);
                return None;
            };
            if let Err(e) = std::fs::write(&out_path, &compressed) {
                log::warn!("写入 {} 失败: {}", out_path.display(), e);
                progress.on_file_progress(&task.input, index + 1, total_images);
                return None;
            }
            progress.on_file_progress(&task.input, index + 1, total_images);
            Some((out_path, compressed.len() as u64))
        })
        .collect();

    if progress.is_cancelled() {
        return TaskOutcome::Cancelled;
    }

    let failures = images.len() - out_files.len();
    if failures > 0 {
        log::warn!(
            "{}: {}/{} 图片失败",
            task.input.display(),
            failures,
            images.len()
        );
    }

    if out_files.is_empty() {
        return TaskOutcome::Failed {
            msg: format!("所有 {} 个图片均失败", failures),
            in_size: Some(in_size),
        };
    }

    TaskOutcome::Ok { in_size, out_files }
}

fn processing_error_message(error: crate::Error) -> String {
    match error {
        crate::Error::Image(msg)
        | crate::Error::Encode(msg)
        | crate::Error::Pdf(msg)
        | crate::Error::Other(msg) => msg,
        crate::Error::Io(e) => e.to_string(),
        crate::Error::Cancelled => "已取消".into(),
    }
}
