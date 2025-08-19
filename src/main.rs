pub mod api;
pub mod app;
mod themes;
pub mod tui;

use crate::app::app_state::AppState;
use crate::app::TuiPage;
use crate::tui::auth::run_auth_page;
use crate::tui::home::run_home_page;
use crate::tui::help::run_help_page;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, prelude::Backend, Terminal};
use std::io::{self};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_state = Arc::new(Mutex::new(AppState::new()));

    run_app(&mut terminal, app_state).await?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<()> {
    let mut current_page = TuiPage::Home;

    loop {
        let next_page = match current_page {
            TuiPage::Home => run_home_page(terminal, app_state.clone()).await?,
            TuiPage::Auth => Some(run_auth_page(terminal, app_state.clone()).await?),
            TuiPage::Chat => tui::chat::run_chat_page(terminal, app_state.clone()).await?,
            TuiPage::Settings => {
                tui::settings::run_settings_page(terminal, app_state.clone()).await?
            }
            TuiPage::Help => run_help_page(terminal, app_state.clone()).await?,
            TuiPage::Exit => None,
        };

        if let Some(page) = next_page {
            current_page = page;
        } else {
            break;
        }

        if app_state.lock().await.should_exit_app {
            break;
        }
    }

    Ok(())
}
