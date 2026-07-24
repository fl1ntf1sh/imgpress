use super::slint_app::MainWindow;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

pub(super) type DeletePrompt = Arc<Mutex<Option<mpsc::Sender<bool>>>>;

pub(super) fn new_prompt() -> DeletePrompt {
    Arc::new(Mutex::new(None))
}

pub(super) fn install_handlers(app: &MainWindow, prompt: DeletePrompt) {
    let confirm_prompt = prompt.clone();
    app.on_confirm_delete_source(move || {
        send_response(&confirm_prompt, true);
    });

    app.on_cancel_delete_source(move || {
        send_response(&prompt, false);
    });
}

pub(super) fn cancel_pending(prompt: &DeletePrompt) {
    send_response(prompt, false);
}

pub(super) fn confirm_with_timeout(
    ui: slint::Weak<MainWindow>,
    prompt: &DeletePrompt,
    input: &Path,
) -> bool {
    let (tx, rx) = mpsc::channel();
    *prompt.lock().unwrap() = Some(tx);

    show_prompt(&ui, input);
    let confirmed = wait_for_response(&ui, rx).unwrap_or(true);

    *prompt.lock().unwrap() = None;
    hide_prompt(&ui);
    confirmed
}

fn send_response(prompt: &DeletePrompt, value: bool) {
    if let Some(sender) = prompt.lock().unwrap().take() {
        let _ = sender.send(value);
    }
}

fn show_prompt(ui: &slint::Weak<MainWindow>, input: &Path) {
    let input = input.display().to_string();
    let ui = ui.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            ui.set_delete_countdown_text("剩余 15 秒".into());
            ui.set_delete_prompt_visible(true);
            ui.set_summary("等待确认删除源文件".into());
            ui.set_run_log(format!("等待确认是否删除源文件：{}", input).into());
        }
    });
}

fn hide_prompt(ui: &slint::Weak<MainWindow>) {
    let ui = ui.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            ui.set_delete_prompt_visible(false);
        }
    });
}

fn wait_for_response(ui: &slint::Weak<MainWindow>, rx: mpsc::Receiver<bool>) -> Option<bool> {
    for remaining in (0..15).rev() {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(value) => return Some(value),
            Err(mpsc::RecvTimeoutError::Timeout) => update_countdown(ui, remaining),
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
    None
}

fn update_countdown(ui: &slint::Weak<MainWindow>, remaining: i32) {
    let text = format!("剩余 {} 秒", remaining);
    let ui = ui.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            ui.set_delete_countdown_text(text.into());
        }
    });
}
