mod api;
mod app;
mod tui;

use app::AppState;
use log::error;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logger
    let log_file = std::fs::File::create("log.log").expect("Could not create log file");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_module("tokio_tungstenite", log::LevelFilter::Off)
        .filter_module("tungstenite", log::LevelFilter::Off)
        .init();
    log::debug!("ReeTUI application started.");

    let app_state = Arc::new(tokio::sync::Mutex::new(AppState::new()));

    if let Err(e) = tui::run_tui(app_state.clone()).await {
        error!("TUI application error: {:?}", e);
    }

    let cache_dir = std::env::temp_dir().join("ReeTUI_cache");
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    Ok(())
}
