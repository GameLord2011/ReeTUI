use crate::app::AppState;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self},
    sync::{Arc, Mutex},
};

pub mod auth_tui;
pub mod chat_tui;
pub mod home_tui;
pub mod themes;

// SIMPLE

#[derive(Debug, PartialEq, Eq)]
pub enum TuiPage {
    Auth,
    Home,
    Chat,
    Exit,
}
pub async fn run_tui(app_state: Arc<Mutex<AppState>>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut current_page = TuiPage::Auth;

    loop {
        match current_page {
            TuiPage::Auth => {
                current_page = auth_tui::run_auth_page(&mut terminal, app_state.clone()).await?;
            }
            TuiPage::Home => {
                current_page = home_tui::run_home_page(&mut terminal, app_state.clone()).await?;
            }
            TuiPage::Chat => {
                current_page = chat_tui::run_chat_page(&mut terminal, app_state.clone()).await?;
            }
            TuiPage::Exit => break,
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
