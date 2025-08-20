use std::io;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::app::app_state::AppState;
use crate::app::TuiPage;

pub mod events;
pub mod page;
pub mod state;

pub async fn run_help_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<Option<TuiPage>> {
    let mut app_state_locked = app_state.lock().await;

    // Check for Nerd Font
    if let Ok(output) = Command::new("fc-list").output().await {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            if output_str.to_lowercase().contains("nerd font") {
                app_state_locked.help_state.show_font_check_page = false;
            }
        }
    }

    // Check for chafa
    if let Ok(output) = Command::new("which").arg("chafa").output().await {
        if output.status.success() {
            app_state_locked.help_state.show_chafa_check_page = false;
        }
    }

    drop(app_state_locked);

    loop {
        let mut app_state_locked = app_state.lock().await;
        terminal.draw(|f| {
            let size = f.area();
            page::render_help_page(f, &mut app_state_locked, size);
        })?;

        if let Some(next_page) = app_state_locked.next_page.take() {
            return Ok(Some(next_page));
        }
        drop(app_state_locked);

        if event::poll(Duration::from_millis(25))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut app_state_locked = app_state.lock().await;
                    if let Some(page) = events::handle_key_events(key, &mut app_state_locked) {
                        return Ok(Some(page));
                    }
                }
            }
        }

        let mut app_state_locked = app_state.lock().await;
        app_state_locked.help_state.info_text_animation_progress += 1;
        if app_state_locked.help_state.gauge_animation_active {
            app_state_locked.help_state.gauge_animation_progress += 0.05; // 20 steps * 25ms = 500ms
            if app_state_locked.help_state.gauge_animation_progress >= 1.0 {
                app_state_locked.help_state.gauge_animation_progress = 1.0;
                app_state_locked.help_state.gauge_animation_active = false;
            }
        }
    }
}