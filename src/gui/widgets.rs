use eframe::egui::{self, Color32, RichText, Ui, Vec2};

pub fn section(ui: &mut Ui, title: &str, icon: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(4.0);
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}  {}", icon, title))
                        .strong()
                        .size(13.0),
                );
            });
            ui.add_space(4.0);
            add(ui);
        });
}

pub fn path_row(ui: &mut Ui, label: &str, value: &mut String, dir: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0));
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(ui.available_width() - 90.0)
                .hint_text(if dir { "选择文件夹..." } else { "选择文件..." })
                .font(egui::TextStyle::Body),
        );
        if resp.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
        {
            resp.request_focus();
        }
        let btn = ui
            .add_sized([80.0, 24.0], egui::Button::new("浏览..."))
            .on_hover_text(if dir { "选择文件夹" } else { "选择文件" });
        if btn.clicked() {
            let picked = if dir {
                rfd::FileDialog::new().pick_folder()
            } else {
                rfd::FileDialog::new().pick_file()
            };
            if let Some(p) = picked {
                *value = p.display().to_string();
            }
        }
    });
}

pub fn info_line(ui: &mut Ui, icon: &str, text: &str, color: Option<Color32>) {
    let rt = match color {
        Some(c) => RichText::new(format!("{}  {}", icon, text)).color(c).size(12.0),
        None => RichText::new(format!("{}  {}", icon, text))
            .color(Color32::from_gray(140))
            .size(12.0),
    };
    ui.label(rt);
}

pub fn stat_row(ui: &mut Ui, items: &[(&str, &str)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        for (label, value) in items {
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(*label)
                        .color(Color32::from_gray(140))
                        .size(11.0),
                );
                ui.label(RichText::new(*value).strong().size(14.0));
            });
        }
    });
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

pub fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".into();
    }
    let s = secs as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).strong())
        .min_size(Vec2::new(120.0, 28.0))
        .fill(if enabled {
            Color32::from_rgb(60, 130, 220)
        } else {
            Color32::from_gray(70)
        });
    ui.add_enabled(enabled, btn)
}

pub fn danger_button(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).strong())
        .min_size(Vec2::new(100.0, 28.0))
        .fill(if enabled {
            Color32::from_rgb(200, 80, 80)
        } else {
            Color32::from_gray(70)
        });
    ui.add_enabled(enabled, btn)
}

