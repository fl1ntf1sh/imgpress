mod delete_confirm;
mod run_log;
mod settings_binding;
mod slint_app;
mod ui_options;
mod worker;

use crate::Result;

pub fn run() -> Result<()> {
    slint_app::run()
}
