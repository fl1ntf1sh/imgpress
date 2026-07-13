pub mod state;
pub mod widgets;
use crate::Result;

pub fn run() -> Result<()> {
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([820.0, 760.0])
            .with_min_inner_size([700.0, 600.0])
            .with_title("imgpress")
            .with_app_id("imgpress"),
        ..Default::default()
    };
    options.vsync = true;

    eframe::run_native(
        "imgpress",
        options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(state::AppState::new(cc)))
        }),
    )
    .map_err(|e| crate::Error::Other(format!("eframe error: {}", e)))
}

fn install_cjk_font(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();

    let candidates: &[(&str, u32)] = &[
        ("C:\\Windows\\Fonts\\msyh.ttc", 0),
        ("C:\\Windows\\Fonts\\msyhbd.ttc", 0),
        ("C:\\Windows\\Fonts\\simhei.ttf", 0),
        ("C:\\Windows\\Fonts\\simsun.ttc", 0),
        ("/System/Library/Fonts/PingFang.ttc", 0),
        ("/System/Library/Fonts/STHeiti Medium.ttc", 0),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
        ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
    ];

    for (path, index) in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk".to_owned(),
                eframe::egui::FontData {
                    font: std::borrow::Cow::Owned(bytes),
                    index: *index,
                    tweak: Default::default(),
                },
            );
            log::info!("loaded CJK font: {} (index {})", path, index);
            break;
        }
    }

    if let Some(family) = fonts.families.get_mut(&eframe::egui::FontFamily::Proportional) {
        family.insert(0, "cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&eframe::egui::FontFamily::Monospace) {
        family.insert(0, "cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn apply_theme(ctx: &eframe::egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.override_text_color = Some(eframe::egui::Color32::from_rgb(225, 225, 230));
    visuals.widgets.noninteractive.bg_fill = eframe::egui::Color32::from_rgb(40, 44, 52);
    visuals.widgets.inactive.bg_fill = eframe::egui::Color32::from_rgb(50, 54, 62);
    visuals.widgets.hovered.bg_fill = eframe::egui::Color32::from_rgb(60, 66, 78);
    visuals.widgets.active.bg_fill = eframe::egui::Color32::from_rgb(70, 100, 150);
    visuals.widgets.noninteractive.bg_stroke =
        eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(60, 64, 72));
    visuals.widgets.inactive.bg_stroke =
        eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(70, 74, 82));
    visuals.widgets.hovered.bg_stroke =
        eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(90, 110, 150));
    visuals.selection.bg_fill = eframe::egui::Color32::from_rgb(60, 110, 180);
    visuals.hyperlink_color = eframe::egui::Color32::from_rgb(110, 170, 230);
    visuals.window_fill = eframe::egui::Color32::from_rgb(30, 33, 40);
    visuals.panel_fill = eframe::egui::Color32::from_rgb(30, 33, 40);
    visuals.faint_bg_color = eframe::egui::Color32::from_rgb(40, 44, 52);
    visuals.extreme_bg_color = eframe::egui::Color32::from_rgb(20, 22, 28);
    style.spacing.item_spacing = eframe::egui::vec2(8.0, 6.0);
    style.spacing.window_margin = eframe::egui::Margin::same(12.0);
    ctx.set_style(style);
}