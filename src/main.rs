mod api;
mod app;
mod tui;

use app::AppState;
use std::sync::{Arc, Mutex};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_state = Arc::new(Mutex::new(AppState::new()));
    println!("Starting TUI application...");
    if let Err(e) = tui::run_tui(app_state.clone()).await {
        eprintln!("TUI application error: {:?}", e);
    }

    println!("TUI application exited.");

    Ok(())
}
