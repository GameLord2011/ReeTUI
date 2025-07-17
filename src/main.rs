mod api;
mod app;
mod tui;

use app::AppState;
use env_logger::{Builder, Target};
use log::error;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut builder = Builder::new();
    builder.filter_level(log::LevelFilter::Debug);

    if let Ok(log_file_path) = std::env::var("REEE_LOG_FILE") {
        if let Ok(file) = std::fs::File::create(&log_file_path) {
            builder.target(Target::Pipe(Box::new(file)));
        } else {
            eprintln!("Warning: Could not create log file at {}", log_file_path);
            builder.target(Target::Stderr); 
        }
    } else {
        builder.target(Target::Stderr); 
    }

    builder.init();

    log::debug!("ReeTUI application started.");

    let app_state = Arc::new(Mutex::new(AppState::new()));
    
    if let Err(e) = tui::run_tui(app_state.clone()).await {
        error!("TUI application error: {:?}", e);
    }

    Ok(())
}
