use crate::app::AppState; // Import AppState for shared application data
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self},
    sync::{Arc, Mutex}, // For shared mutable access to AppState
    time::Duration,
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
                let next_page = auth_tui::run_auth_page(&mut terminal, app_state.clone()).await?;
                current_page = next_page;
            }
            TuiPage::Home => {
                let next_page = home_tui::run_home_page(&mut terminal, app_state.clone()).await?;
                current_page = next_page;
            }
            TuiPage::Chat => {
                let next_page = chat_tui::run_chat_page(&mut terminal, app_state.clone()).await?;
                current_page = next_page;
            }
            TuiPage::Exit => break, // Exit the loop if the page signals to exit
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    current_page = TuiPage::Exit;
                }
            }
        }
    }

    disable_raw_mode()?; // Disable raw mode
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?; // Leave alternate screen
    terminal.show_cursor()?; // Show cursor again

    Ok(())
}
