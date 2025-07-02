use crate::app::AppState;
use crate::tui::TuiPage; // Import TuiPage enum
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

pub async fn run_home_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    loop {
        // Draw the UI
        terminal.draw(|f| {
            draw_home_ui::<B>(f, app_state.clone());
        })?;

        // Handle events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(TuiPage::Exit), // Global exit
                        KeyCode::Char('c') => return Ok(TuiPage::Chat), // Transition to Chat page
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Draws the home page UI.
fn draw_home_ui<B: Backend>(f: &mut Frame, app_state: Arc<Mutex<AppState>>) {
    let size = f.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Home Page ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(ratatui::style::Color::Green));

    f.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints(
            [
                Constraint::Length(3), // User info
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Instructions
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(f.area());

    let user_info_text = {
        let state = app_state.lock().unwrap();
        if let Some(username) = &state.username {
            format!(
                "Welcome, {} {}",
                username,
                state.user_icon.as_deref().unwrap_or("")
            )
        } else {
            "Not logged in.".to_string()
        }
    };

    let user_info_paragraph = Paragraph::new(Line::from(user_info_text))
        .block(Block::default().borders(Borders::ALL).title("User Info"))
        .style(Style::default().fg(ratatui::style::Color::LightGreen))
        .alignment(Alignment::Center);
    f.render_widget(user_info_paragraph, chunks[0]);

    let instructions = Paragraph::new(Line::from("Press 'C' to go to Chat, 'Q' to quit."))
        .style(Style::default().fg(ratatui::style::Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);
}
