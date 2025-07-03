mod api;
mod app;
mod tui;

use app::AppState;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let app_state = Arc::new(Mutex::new(AppState::new()));
    // starting the tui, wish me luck
    if let Err(e) = tui::run_tui(app_state.clone()).await {
        eprintln!("TUI application error: {:?}", e);
    }

    Ok(())
}
