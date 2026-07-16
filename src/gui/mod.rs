pub mod state;
pub mod widgets;
use crate::Result;

pub fn run() -> Result<()> {
    let icon = load_icon();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([820.0, 820.0])
        .with_min_inner_size([700.0, 600.0])
        .with_title("imgpress")
        .with_app_id("imgpress");
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    options.glow_options.vsync = true;

    eframe::run_native(
        "imgpress",
        options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            cc.egui_ctx.set_theme(eframe::egui::Theme::Light);
            Ok(Box::new(state::AppState::new(cc)))
        }),
    )
    .map_err(|e| crate::Error::Other(format!("eframe error: {}", e)))
}

fn load_icon() -> Option<std::sync::Arc<eframe::egui::IconData>> {
    let bytes = include_bytes!("../../assets/icon.ico");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Ico)
        .ok()?
        .into_rgba8();
    let (width, height) = img.dimensions();
    Some(std::sync::Arc::new(eframe::egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }))
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
                std::sync::Arc::new(eframe::egui::FontData {
                    font: std::borrow::Cow::Owned(bytes),
                    index: *index,
                    tweak: Default::default(),
                }),
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