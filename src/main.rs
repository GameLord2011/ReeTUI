mod api;
mod app;
mod tui;

use app::AppState;
use log::error;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Logger initialization removed to disable logging.
    log::debug!("ReeTUI application started.");

    let app_state = Arc::new(Mutex::new(AppState::new()));

    if let Err(e) = tui::run_tui(app_state.clone()).await {
        error!("TUI application error: {:?}", e);
    }

    let cache_dir = std::env::temp_dir().join("ReeTUI_cache");
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    Ok(())
}
