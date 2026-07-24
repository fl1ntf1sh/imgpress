use crate::config::CompressOptions;
use crate::discovery::FileTask;
use crate::input::ImageLabel;
use std::path::PathBuf;

pub fn path_for(task: &FileTask, label: &ImageLabel, opts: &CompressOptions) -> PathBuf {
    match label {
        ImageLabel::Single => task.output.clone(),
        ImageLabel::Page { index } => page_output_path(task, *index, opts),
        ImageLabel::Embedded { index } => embedded_output_path(task, *index, opts),
    }
}

fn page_output_path(task: &FileTask, index: usize, opts: &CompressOptions) -> PathBuf {
    let stem = task
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let ext = crate::discovery::output_extension(opts.format);
    let parent = task.output.parent().unwrap_or(&task.output);
    let target_name = format!("{}_page{}.{}", stem, index, ext);
    crate::discovery::unique_in_dir(parent, &target_name)
}

fn embedded_output_path(task: &FileTask, index: usize, opts: &CompressOptions) -> PathBuf {
    let stem = task
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let ext = crate::discovery::output_extension(opts.format);
    let parent = task.output.parent().unwrap_or(&task.output);
    let target_name = format!("{}_image{}.{}", stem, index, ext);
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
    fn page_uses_input_stem_and_page_index() {
        let task = FileTask {
            input: PathBuf::from("input/report.pdf"),
            output: PathBuf::from("output/report.jpg"),
        };
        let opts = CompressOptions::default();

        let path = path_for(&task, &ImageLabel::Page { index: 2 }, &opts);

        assert_eq!(path, PathBuf::from("output/report_page2.jpg"));
    }

    #[test]
    fn embedded_image_uses_input_stem_and_image_index() {
        let task = FileTask {
            input: PathBuf::from("input/report.docx"),
            output: PathBuf::from("output/report.jpg"),
        };
        let opts = CompressOptions::default();

        let path = path_for(&task, &ImageLabel::Embedded { index: 3 }, &opts);

        assert_eq!(path, PathBuf::from("output/report_image3.jpg"));
    }
}
