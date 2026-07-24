use super::slint_app::MainWindow;
use crate::config::{validate_options, CompressOptions, Format, SizeLimit};
use std::path::PathBuf;

pub(super) fn options_from_ui(app: &MainWindow) -> std::result::Result<CompressOptions, String> {
    let max_size_kb = parse_u32(app.get_max_size_kb_text().as_str(), "目标大小")?;
    let min_quality = parse_u8(app.get_min_quality_text().as_str(), "最低质量")?;
    let max_quality = parse_u8(app.get_max_quality_text().as_str(), "最高质量")?;
    let scale_step_percent = parse_u32(app.get_scale_step_percent_text().as_str(), "缩放步长")?;
    let max_scales = parse_u32(app.get_max_scales_text().as_str(), "缩放轮数")?;

    let opts = CompressOptions {
        input: PathBuf::from(app.get_input_path().trim().to_string()),
        output: PathBuf::from(app.get_output_path().trim().to_string()),
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
    };
    validate_options(&opts)?;
    Ok(opts)
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
