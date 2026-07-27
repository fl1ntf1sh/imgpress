use crate::config::CompressOptions;
use crate::discovery::FileTask;
use crate::input::ImageLabel;
use std::path::PathBuf;

pub fn path_for(task: &FileTask, label: &ImageLabel, opts: &CompressOptions) -> PathBuf {
    match label {
        ImageLabel::Single => task.output.clone(),
        ImageLabel::Page { index } => page_output_path(task, *index, opts),
    }
}

fn page_output_path(task: &FileTask, index: usize, opts: &CompressOptions) -> PathBuf {
    let output_stem = task
        .output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let source_ext = task
        .input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let ext = crate::discovery::output_extension(opts.format);
    let parent = task.output.parent().unwrap_or(&task.output);
    let target_name = format!("{}_{}_page{}.{}", output_stem, source_ext, index, ext);
    crate::discovery::unique_in_dir(parent, &target_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_image_uses_task_output() {
        let task = FileTask {
            input: PathBuf::from("input/photo.png"),
            output: PathBuf::from("output/photo.jpg"),
        };
        let opts = CompressOptions::default();

        let path = path_for(&task, &ImageLabel::Single, &opts);

        assert_eq!(path, PathBuf::from("output/photo.jpg"));
    }

    #[test]
    fn page_uses_output_stem_source_format_and_page_index() {
        let task = FileTask {
            input: PathBuf::from("input/张三/report.pdf"),
            output: PathBuf::from("output/张三_report.jpg"),
        };
        let opts = CompressOptions::default();

        let path = path_for(&task, &ImageLabel::Page { index: 2 }, &opts);

        assert_eq!(path, PathBuf::from("output/张三_report_pdf_page2.jpg"));
    }
}
