#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::new_without_default)]

mod cmd;
mod controller;
mod data;
mod delegate;
mod error;
mod ui;
mod webapi;
mod widget;

use druid::AppLauncher;
use env_logger::{Builder, Env};
use webapi::{WebApi, LocalTrackManager};

use crate::{
    data::{AppState, Config},
    delegate::Delegate,
};

const ENV_LOG: &str = "PSST_LOG";
const ENV_LOG_STYLE: &str = "PSST_LOG_STYLE";

fn main() {
    // Setup logging from the env variables, with defaults.
    Builder::from_env(
        Env::new()
            // `ureq` is a bit too noisy, log only warnings by default.
            .filter_or(ENV_LOG, "info,ureq::unit=warn")
            .write_style(ENV_LOG_STYLE),
    )
    .init();

    let config = Config::load().unwrap_or_default();
    let state = AppState::default_with_config(config);
    WebApi::new(
        state.session.clone(),
        Config::proxy().as_deref(),
        Config::cache_dir()
    ).install_as_global();

    LocalTrackManager::new().install_as_global();

    let delegate;
    let launcher;
    if state.config.has_credentials() {
        // If the user is logged in get their local tracks for the WebApi
        let mut track_manager = LocalTrackManager::global().lock().unwrap();
        track_manager.read_new_user(state.config.get_username().unwrap());
        drop(track_manager);

        // Credentials are configured, open the main window.
        let window = ui::main_window();
        delegate = Delegate::with_main(window.id);
        launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);
    } else {
        // No configured credentials, open the preferences.
        let window = ui::preferences_window();
        delegate = Delegate::with_preferences(window.id);
        launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);
    };

    launcher
        .delegate(delegate)
        .launch(state)
        .expect("Application launch");
}
