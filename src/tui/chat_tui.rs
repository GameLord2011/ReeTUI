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

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();

    loop {
        // Draw the UI
        terminal.draw(|f| {
            draw_chat_ui::<B>(f, app_state.clone(), &input_text);
        })?;

        // Handle events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(TuiPage::Exit), // Global exit
                        KeyCode::Char('h') => return Ok(TuiPage::Home), // Transition to Home page
                        KeyCode::Enter => {
                            // Simulate sending a message
                            if !input_text.is_empty() {
                                let mut state = app_state.lock().unwrap();
                                if let Some(current_channel) = &state.current_channel {
                                    // In a real app, you'd send this message via WebSocket
                                    // For now, just add it to the state for display
                                    let new_message = crate::api::models::BroadcastMessage {
                                        user: state
                                            .username
                                            .clone()
                                            .unwrap_or_else(|| "Unknown".to_string()),
                                        icon: state
                                            .user_icon
                                            .clone()
                                            .unwrap_or_else(|| "❓".to_string()),
                                        content: input_text.clone(),
                                        timestamp: chrono::Utc::now().timestamp(),
                                        channel_id: current_channel.id.clone(),
                                        channel_name: current_channel.name.clone(),
                                        channel_icon: current_channel.icon.clone(),
                                    };
                                    state.add_message(new_message);
                                }
                                input_text.clear(); // Clear input after sending
                            }
                        }
                        KeyCode::Backspace => {
                            input_text.pop();
                        }
                        KeyCode::Char(c) => {
                            input_text.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Draws the chat page UI.
fn draw_chat_ui<B: Backend>(f: &mut Frame, app_state: Arc<Mutex<AppState>>, input_text: &str) {
    let size = f.area();

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Chat ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(ratatui::style::Color::LightBlue));

    f.render_widget(main_block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1) // Margin inside the main block
        .constraints(
            [
                Constraint::Min(1),    // Messages display area
                Constraint::Length(3), // Input field
            ]
            .as_ref(),
        )
        .split(f.area());

    // Messages display area
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("Messages")
        .style(Style::default().fg(ratatui::style::Color::White));
    f.render_widget(messages_block, chunks[0]);

    let messages_content = {
        let state = app_state.lock().unwrap();
        if let Some(current_channel) = &state.current_channel {
            if let Some(messages) = state.get_messages_for_channel(&current_channel.id) {
                messages
                    .iter()
                    .map(|msg| Line::from(format!("{} {}: {}", msg.icon, msg.user, msg.content)))
                    .collect::<Vec<Line>>()
            } else {
                vec![Line::from("No messages in this channel yet.")]
            }
        } else {
            vec![Line::from("Select a channel to see messages.")]
        }
    };

    let messages_paragraph = Paragraph::new(messages_content)
        .block(Block::default()) // No additional borders, already handled by messages_block
        .wrap(ratatui::widgets::Wrap { trim: false }); // Allow text wrapping
    f.render_widget(messages_paragraph, chunks[0]);

    // Input field
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("Input")
        .style(Style::default().fg(ratatui::style::Color::Yellow));

    let input_paragraph = Paragraph::new(Line::from(input_text.to_string())).block(input_block);
    f.render_widget(input_paragraph, chunks[1]);

    // Instructions
    let instructions = Paragraph::new(Line::from(
        "Press <Enter> to send, 'H' for Home, 'Q' to quit.",
    ))
    .style(Style::default().fg(ratatui::style::Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(
        instructions,
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(size)[1],
    );
}
