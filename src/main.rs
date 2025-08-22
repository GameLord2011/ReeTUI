pub mod api;
pub mod app;
pub mod config;
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

    let config = crate::config::load_config();

    let initial_page = if !config.tutorial_seen {
        TuiPage::Help
    } else if config.token.is_none() {
        TuiPage::Auth
    } else {
        TuiPage::Chat
    };

    let app_state = Arc::new(Mutex::new(AppState::new(config)));

    let config_path_debug = {
        let mut config_dir = dirs::config_dir().unwrap();
        let mut debug_path_string = format!("Initial config_dir: {:?}\n", config_dir);
        config_dir.push("reetui");
        debug_path_string.push_str(&format!("After 'reetui' push: {:?}\n", config_dir));
        config_dir.push("reetui.json");
        debug_path_string.push_str(&format!("Final path: {:?}", config_dir));
        debug_path_string
    };

    app_state
        .lock()
        .await
        .notification_manager
        .add(
            "Config Path Debug".to_string(),
            config_path_debug,
            crate::tui::notification::notification::NotificationType::Info,
            Some(std::time::Duration::from_secs(20)),
            app_state.clone(),
        )
        .await;

    run_app(&mut terminal, app_state.clone(), initial_page).await?;

    // Save config before exiting
    let app_state_locked = app_state.lock().await;
    if app_state_locked.config.token.is_some() {
        crate::config::save_config(&app_state_locked.config);
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
    initial_page: TuiPage,
) -> io::Result<()> {
    let mut current_page = initial_page;

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
