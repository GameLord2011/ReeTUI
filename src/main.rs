pub mod api;
pub mod app;
mod themes;
pub mod tui;

use crate::app::app_state::AppState;
use crate::tui::auth::run_auth_page;
use crate::tui::home::run_home_page;
use crate::app::TuiPage;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend,prelude::Backend, Terminal};
use std::io::{self};
use std::sync::Arc;
use tokio::sync::Mutex;
use clap::Parser; 
use log::{LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let stdout_appender = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {M} - {m}{n}")))
        .build();

    if args.debug {
        log::info!("Debug logging enabled. Logs will be written to log.log");
        let file_appender = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S%.3f)} {h({l})} {M} - {m}{n}")))
            .build("log.log")
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout_appender)))
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(
                Root::builder()
                    .appender("stdout")
                    .appender("file")
                    .build(LevelFilter::Debug),
            )
            .unwrap();
        log4rs::init_config(config).unwrap();
    } else {
        log::info!("Debug logging disabled. No logs will be written to log.log");
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout_appender)))
            .build(
                Root::builder()
                    .appender("stdout")
                    .build(LevelFilter::Off),
            )
            .unwrap();
        log4rs::init_config(config).unwrap();
    }

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
            TuiPage::Home => {
                let result = run_home_page(terminal, app_state.clone()).await?;
                if let Some(TuiPage::Settings) = result {
                }
                result
            }
            TuiPage::Auth => {
                let result = Some(run_auth_page(terminal, app_state.clone()).await?);
                if let Some(TuiPage::Settings) = result {
                }
                result
            }
            TuiPage::Chat => {
                let result = tui::chat::run_chat_page(terminal, app_state.clone()).await?;
                result
            }
            TuiPage::Settings => {
                tui::settings::run_settings_page(terminal, app_state.clone()).await?
            }
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