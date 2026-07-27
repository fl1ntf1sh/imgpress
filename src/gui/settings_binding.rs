use super::slint_app::MainWindow;
use crate::config::Format;
use crate::settings::AppSettings;

pub(super) fn load_settings(app: &MainWindow) {
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
    app.set_skip_if_smaller(settings.skip_if_smaller);
    app.set_recursive(settings.recursive);
    app.set_delete_source(settings.delete_source);
    app.set_organize_after_success(settings.organize_after_success);
    app.set_summary("就绪".into());
    app.set_run_log("等待开始任务。".into());
    app.set_delete_prompt_visible(false);
    app.set_delete_countdown_text("剩余 15 秒".into());
}

pub(super) fn save_settings_from_ui(opts: &crate::config::CompressOptions) {
    let settings = AppSettings {
        last_input: Some(opts.input.clone()),
        last_output: Some(opts.output.clone()),
        max_size_kb: opts.max_size.bytes.saturating_div(1024) as u32,
        format: opts.format,
        min_quality: opts.min_quality,
        max_quality: opts.max_quality,
        scale_step: opts.scale_step,
        skip_if_smaller: opts.skip_if_smaller,
        recursive: opts.recursive,
        delete_source: opts.delete_source,
        organize_after_success: opts.organize_after_success,
        max_scales: opts.max_scales,
    };
    let _ = settings.save();
}
