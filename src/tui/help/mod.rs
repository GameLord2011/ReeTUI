use std::io;
use std::sync::Arc;

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
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
    app_state_locked.help_state.total_pages = 3; // Set total pages
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

        if event::poll(std::time::Duration::from_millis(100))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut app_state_locked = app_state.lock().await;
                    if let Some(page) = events::handle_key_events(key, &mut app_state_locked) {
                        return Ok(Some(page));
                    }
                    
                    drop(app_state_locked);
                }
            }
        }
    }
}
