use super::state::AppState;
use super::widgets;
use crate::config::Format;
use eframe::egui::{Color32, RichText, Vec2};
use std::path::Path;

fn short_path(p: &Path) -> String {
    let s = p.display().to_string();
    if s.chars().count() <= 40 {
        return s;
    }
    match p.file_name() {
        Some(name) => format!("...{}", name.to_string_lossy()),
        None => s,
    }
}

pub fn draw_ui(ui: &mut eframe::egui::Ui, state: &mut AppState) {
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

            ui.horizontal(|ui| {
                let half = ui.available_width() / 2.0;
                ui.vertical(|ui| {
                    ui.set_width(half);
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
                                [70.0, 16.0],
                                eframe::egui::Slider::new(&mut state.min_quality, 1..=state.max_quality)
                                    .show_value(false),
                            );
                            ui.label("—");
                            ui.add_sized(
                                [70.0, 16.0],
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
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("最大缩放轮数").size(12.0));
                            ui.add(
                                eframe::egui::DragValue::new(&mut state.max_scales)
                                    .range(1..=50)
                                    .max_decimals(0),
                            );
                        });
                    });
                });
                ui.vertical(|ui| {
                    ui.set_width(half);
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
                        ui.checkbox(
                            &mut state.write_log,
                            "生成日志文件",
                        );
                        if let Some(path) = crate::log::log_file_path() {
                            ui.label(
                                eframe::egui::RichText::new(format!(
                                    "保存到: {}",
                                    path.display()
                                ))
                                .color(eframe::egui::Color32::from_gray(140))
                                .size(11.0),
                            );
                        }
                    });
                });
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

            if !state.failed_files.is_empty() || !state.info_messages.is_empty() {
                ui.add_space(8.0);
                eframe::egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        let count = state.failed_files.len() + state.info_messages.len();
                        ui.label(
                            RichText::new(format!("⚠  失败/信息 ({} 条)", count))
                                .strong()
                                .size(13.0),
                        );
                        ui.add_space(2.0);
                        eframe::egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (path, msg) in &state.failed_files {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} - {}",
                                            short_path(path),
                                            msg
                                        ))
                                        .color(Color32::from_rgb(220, 100, 100))
                                        .size(11.5),
                                    );
                                }
                                for msg in &state.info_messages {
                                    ui.label(
                                        RichText::new(msg)
                                            .color(Color32::from_rgb(220, 170, 80))
                                            .size(11.5),
                                    );
                                }
                            });
                    });
            }
            ui.add_space(8.0);
        });
}
