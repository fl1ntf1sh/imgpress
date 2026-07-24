use slint::ComponentHandle;
use std::path::{Path, PathBuf};

slint::include_modules!();

pub fn run() -> crate::Result<()> {
    let app =
        MainWindow::new().map_err(|e| crate::Error::Other(format!("slint init error: {}", e)))?;
    let runtime = crate::gui::worker::Runtime::default();
    let delete_prompt = crate::gui::delete_confirm::new_prompt();

    crate::gui::settings_binding::load_settings(&app);
    bind_callbacks(&app, runtime, delete_prompt);

    app.run()
        .map_err(|e| crate::Error::Other(format!("slint runtime error: {}", e)))
}

fn bind_callbacks(
    app: &MainWindow,
    runtime: crate::gui::worker::Runtime,
    delete_prompt: crate::gui::delete_confirm::DeletePrompt,
) {
    let weak = app.as_weak();
    app.on_browse_input(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(app) = weak.upgrade() {
                app.set_input_path(path.display().to_string().into());
            }
        }
    });

    let weak = app.as_weak();
    app.on_browse_output(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(app) = weak.upgrade() {
                app.set_output_path(path.display().to_string().into());
            }
        }
    });

    let weak = app.as_weak();
    let runtime_for_start = runtime.clone();
    let prompt_for_start = delete_prompt.clone();
    app.on_start(move || {
        if let Some(app) = weak.upgrade() {
            crate::gui::worker::start_compression(&app, &runtime_for_start, &prompt_for_start);
        }
    });

    let weak = app.as_weak();
    let runtime_for_cancel = runtime.clone();
    let prompt_for_cancel = delete_prompt.clone();
    app.on_cancel(move || {
        if let Some(cancel) = runtime_for_cancel.lock().unwrap().as_ref() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        crate::gui::delete_confirm::cancel_pending(&prompt_for_cancel);
        if let Some(app) = weak.upgrade() {
            app.set_summary("正在取消...".into());
        }
    });

    let weak = app.as_weak();
    app.on_open_output(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(app.get_output_path().to_string());
        if path.as_os_str().is_empty() {
            return;
        }
        std::fs::create_dir_all(&path).ok();
        let _ = open::that(path);
    });

    app.on_open_log_dir(move || {
        let Some(path) =
            crate::log::log_file_path().and_then(|path| path.parent().map(Path::to_path_buf))
        else {
            return;
        };
        std::fs::create_dir_all(&path).ok();
        let _ = open::that(path);
    });

    crate::gui::delete_confirm::install_handlers(app, delete_prompt);
}
