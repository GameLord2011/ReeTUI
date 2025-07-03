use crate::app::AppState; // Import AppState for shared application data
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self},
    sync::{Arc, Mutex}, // For shared mutable access to AppState
};

pub mod auth_tui;
pub mod chat_tui;
pub mod home_tui;
pub mod themes;

#[derive(Debug, PartialEq, Eq)]
pub enum TuiPage {
    Auth,
    Home,
    Chat,
    Exit, // Special state to signal application exit
}
pub async fn run_tui(app_state: Arc<Mutex<AppState>>) -> io::Result<()> {
    enable_raw_mode()?; // Enable raw mode for direct input handling
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?; // Enter alternate screen buffer

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut current_page = TuiPage::Auth; // Start with the authentication page

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
            TuiPage::Exit => break, // Exit the loop if the page signals to exit
        }
    }

    disable_raw_mode()?; // Disable raw mode
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?; // Leave alternate screen
    terminal.show_cursor()?; // Show cursor again

    Ok(())
}
