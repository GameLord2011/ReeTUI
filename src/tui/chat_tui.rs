use crate::api::models::{BroadcastMessage, Channel};
use crate::api::websocket::{self, ServerMessage};
use crate::app::AppState;
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_tungstenite::tungstenite;

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let (mut ws_writer, mut ws_reader) = {
        let state = app_state.lock().unwrap();
        let token = state
            .auth_token
            .clone()
            .expect("Auth token not found for WebSocket connection");
        websocket::connect(&token)
            .await
            .expect("Failed to connect to WebSocket")
    };

    loop {
        terminal.draw(|f| {
            draw_chat_ui::<B>(f, app_state.clone(), &input_text, &mut channel_list_state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(TuiPage::Exit),
                        KeyCode::Char('h') => return Ok(TuiPage::Home),
                        KeyCode::Tab => {
                            let mut state = app_state.lock().unwrap();
                            let i = match channel_list_state.selected() {
                                Some(i) => {
                                    if i >= state.channels.len() - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            channel_list_state.select(Some(i));
                            if let Some(selected_channel) = state.channels.get(i).cloned() {
                                state.set_current_channel(selected_channel);
                            }
                        }
                        KeyCode::Up => {
                            let mut state = app_state.lock().unwrap();
                            let i = match channel_list_state.selected() {
                                Some(i) => {
                                    if i == 0 {
                                        state.channels.len() - 1
                                    } else {
                                        i - 1
                                    }
                                }
                                None => 0,
                            };
                            channel_list_state.select(Some(i));
                            if let Some(selected_channel) = state.channels.get(i).cloned() {
                                state.set_current_channel(selected_channel);
                            }
                        }
                        KeyCode::Down => {
                            let mut state = app_state.lock().unwrap();
                            let i = match channel_list_state.selected() {
                                Some(i) => {
                                    if i >= state.channels.len() - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            channel_list_state.select(Some(i));
                            if let Some(selected_channel) = state.channels.get(i).cloned() {
                                state.set_current_channel(selected_channel);
                            }
                        }
                        KeyCode::Enter => {
                            if !input_text.is_empty() {
                                let mut state = app_state.lock().unwrap();
                                if let Some(current_channel) = &state.current_channel {
                                    let channel_id = current_channel.id.clone();
                                    let content = input_text.clone();
                                    if let Err(e) = websocket::send_message(
                                        &mut ws_writer,
                                        &channel_id,
                                        &content,
                                    )
                                    .await
                                    {
                                        eprintln!("Failed to send message: {:?}", e);
                                    }
                                    let new_message = BroadcastMessage {
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
                                input_text.clear();
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

        if let Ok(Some(msg)) =
            tokio::time::timeout(Duration::from_millis(10), ws_reader.next()).await
        {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    let server_message = websocket::parse_server_message(&text);
                    let mut state = app_state.lock().unwrap();
                    match server_message {
                        ServerMessage::ChatMessage(chat_msg) => {
                            state.add_message(chat_msg);
                        }
                        ServerMessage::ChannelUpdate(channel_broadcast) => {
                            let channel = Channel {
                                id: channel_broadcast.id,
                                name: channel_broadcast.name,
                                icon: channel_broadcast.icon,
                            };
                            state.add_or_update_channel(channel);
                        }
                        ServerMessage::ChannelDelete(channel_id) => {
                            state.remove_channel(&channel_id);
                        }
                        ServerMessage::Unknown(unknown_msg) => {
                            eprintln!("Received unknown WebSocket message: {}", unknown_msg);
                        }
                    }
                }
                Ok(tungstenite::Message::Ping(_)) => {
                    if let Err(e) = ws_writer
                        .send(tungstenite::Message::Pong(vec![].into()))
                        .await
                    {
                        eprintln!("Failed to send pong: {:?}", e);
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                    return Ok(TuiPage::Exit);
                }
                _ => {} // ignore other message types, they're not as cool as text messages
            }
        }
    }
}

fn draw_chat_ui<B: Backend>(
    f: &mut Frame,
    app_state: Arc<Mutex<AppState>>,
    input_text: &str,
    channel_list_state: &mut ListState,
) {
    let size = f.area();
    let state = app_state.lock().unwrap();

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            " Chat - {}",
            state.username.as_deref().unwrap_or("Guest")
        ))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(ratatui::style::Color::LightBlue));

    f.render_widget(main_block, size);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(80),
            ]
            .as_ref(),
        )
        .split(f.area());

    let channels_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("Channels")
        .style(Style::default().fg(ratatui::style::Color::Cyan));

    let channel_items: Vec<ListItem> = state
        .channels
        .iter()
        .map(|channel| {
            let is_current = state
                .current_channel
                .as_ref()
                .map_or(false, |c| c.id == channel.id);
            let style = if is_current {
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(ratatui::style::Color::Yellow)
            } else {
                Style::default().fg(ratatui::style::Color::White)
            };
            ListItem::new(format!("{} {}", channel.icon, channel.name)).style(style)
        })
        .collect();

    let channels_list = List::new(channel_items)
        .block(channels_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    f.render_stateful_widget(channels_list, chunks[0], channel_list_state);

    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(format!(
            "Messages - {}",
            state
                .current_channel
                .as_ref()
                .map_or("No Channel Selected".to_string(), |c| c.name.clone())
        ))
        .style(Style::default().fg(ratatui::style::Color::White));
    f.render_widget(messages_block, chat_chunks[0]);

    let messages_content = {
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
        .block(Block::default())
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(messages_paragraph, chat_chunks[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("Input")
        .style(Style::default().fg(ratatui::style::Color::Yellow));

    let input_paragraph = Paragraph::new(Line::from(input_text.to_string())).block(input_block);
    f.render_widget(input_paragraph, chat_chunks[1]);

    let instructions = Paragraph::new(Line::from(
        "Press <Enter> to send, <Tab>/<Up>/<Down> to switch channels, 'H' for Home, 'Q' to quit.",
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
