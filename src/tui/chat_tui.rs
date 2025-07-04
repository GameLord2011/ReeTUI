use crate::api::models::Channel;
use crate::api::websocket::{self, ServerMessage};
use crate::app::{AppState, PopupType}; // Import PopupType
use crate::tui::TuiPage;
use chrono::{TimeZone, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}; // Import KeyModifiers
use futures_util::{SinkExt, StreamExt};
use ratatui::style::Stylize;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style}, // Import Color
    text::{Line, Span, Text},        // Import Text
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph}, // Import Clear
    Frame,
    Terminal,
};
use std::{
    hash::{Hash, Hasher},
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_tungstenite::tungstenite; // Import for timestamp formatting

fn get_color_for_user(username: &str) -> Color {
    // A palette of vibrant, gradient-like colors
    let colors = [
        Color::Rgb(255, 0, 255),   // Magenta
        Color::Rgb(139, 0, 255),   // Dark Violet
        Color::Rgb(0, 191, 255),   // Deep Sky Blue
        Color::Rgb(0, 255, 127),   // Spring Green
        Color::Rgb(255, 215, 0),   // Gold
        Color::Rgb(255, 105, 180), // Hot Pink
        Color::Rgb(255, 69, 0),    // Orange Red
        Color::Rgb(50, 205, 50),   // Lime Green
    ];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    username.hash(&mut hasher);
    let hash = hasher.finish();
    colors[(hash % colors.len() as u64) as usize]
}

#[derive(Default)]
struct CreateChannelForm {
    name: String,
    icon: String,
    input_focused: CreateChannelInput,
}

#[derive(PartialEq, Default)]
enum CreateChannelInput {
    #[default]
    Name,
    Icon,
}

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let mut create_channel_form = CreateChannelForm::default(); // Initialize form state

    // Initialize WebSocket connection
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
        // Clear expired error messages before drawing
        app_state.lock().unwrap().clear_expired_error();

        terminal.draw(|f| {
            draw_chat_ui::<B>(
                f,
                app_state.clone(),
                &input_text,
                &mut channel_list_state,
                &create_channel_form, // Pass form state
            );
        })?;

        // Event handling
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut state = app_state.lock().unwrap();
                    // Handle pop-up specific key events first
                    if state.popup_state.show {
                        match state.popup_state.popup_type {
                            PopupType::Quit => match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    return Ok(TuiPage::Exit);
                                }
                                KeyCode::Esc => {
                                    state.popup_state.show = false;
                                    state.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Settings => match key.code {
                                KeyCode::Esc => {
                                    state.popup_state.show = false;
                                    state.popup_state.popup_type = PopupType::None;
                                }
                                _ => {
                                    state.set_error_message(
                                        "Settings not yet implemented!".to_string(),
                                        3000,
                                    );
                                }
                            },
                            PopupType::CreateChannel => {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.popup_state.show = false;
                                        state.popup_state.popup_type = PopupType::None;
                                        create_channel_form = CreateChannelForm::default();
                                        // Reset form
                                    }
                                    KeyCode::Tab => {
                                        create_channel_form.input_focused =
                                            match create_channel_form.input_focused {
                                                CreateChannelInput::Name => {
                                                    CreateChannelInput::Icon
                                                }
                                                CreateChannelInput::Icon => {
                                                    CreateChannelInput::Name
                                                }
                                            };
                                    }
                                    KeyCode::Backspace => match create_channel_form.input_focused {
                                        CreateChannelInput::Name => {
                                            create_channel_form.name.pop();
                                        }
                                        CreateChannelInput::Icon => {
                                            create_channel_form.icon.pop();
                                        }
                                    },
                                    KeyCode::Char(c) => match create_channel_form.input_focused {
                                        CreateChannelInput::Name => {
                                            create_channel_form.name.push(c);
                                        }
                                        CreateChannelInput::Icon => {
                                            create_channel_form.icon.push(c);
                                        }
                                    },
                                    KeyCode::Enter => {
                                        if !create_channel_form.name.is_empty()
                                            && !create_channel_form.icon.is_empty()
                                        {
                                            let channel_name = create_channel_form.name.clone();
                                            let channel_icon = create_channel_form.icon.clone();
                                            let channel_id = uuid::Uuid::new_v4().to_string(); // Generate random ID

                                            let command = format!(
                                                "/create_channel {} {} {}",
                                                channel_id, channel_name, channel_icon
                                            );
                                            drop(state); // Release lock before await
                                            if let Err(e) = websocket::send_message(
                                                &mut ws_writer,
                                                "home", // Send create channel command to a default channel or a command channel
                                                &command,
                                            )
                                            .await
                                            {
                                                let mut state = app_state.lock().unwrap();
                                                state.set_error_message(
                                                    format!("Failed to create channel: {:?}", e),
                                                    3000,
                                                );
                                                eprintln!("Failed to create channel: {:?}", e);
                                            } else {
                                                let mut state = app_state.lock().unwrap();
                                                state.set_error_message(
                                                    format!("Channel '{}' created!", channel_name),
                                                    3000,
                                                );
                                                state.popup_state.show = false;
                                                state.popup_state.popup_type = PopupType::None;
                                                create_channel_form = CreateChannelForm::default();
                                                // Reset form
                                            }
                                        }
                                        else {
                                            state.set_error_message(
                                                "Channel name and icon cannot be empty!"
                                                    .to_string(),
                                                3000,
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // Normal chat page key events
                        match key.code {
                            KeyCode::Char(':') => {
                                // New key for pop-up menu
                                state.popup_state.show = true;
                                state.popup_state.popup_type = PopupType::Quit; // Default to Quit
                            }
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+N for Create Channel
                                state.popup_state.show = true;
                                state.popup_state.popup_type = PopupType::CreateChannel;
                                create_channel_form = CreateChannelForm::default();
                                // Reset form
                            }
                            KeyCode::Tab => {
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
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Up to scroll messages up
                                    state.scroll_messages_up();
                                } else {
                                    // Normal Up arrow for channel selection
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
                            }
                            KeyCode::Down => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Down to scroll messages down
                                    state.scroll_messages_down();
                                } else {
                                    // Normal Down arrow for channel selection
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
                            }
                            KeyCode::Enter => {
                                if !input_text.is_empty() {
                                    if let Some(current_channel) = &state.current_channel {
                                        let channel_id = current_channel.id.clone();
                                        let content = input_text.clone();
                                        // Release the lock before the await call
                                        drop(state);
                                        if let Err(e) = websocket::send_message(
                                            &mut ws_writer,
                                            &channel_id,
                                            &content,
                                        )
                                        .await
                                        {
                                            let mut state = app_state.lock().unwrap(); // Re-acquire lock
                                            state.set_error_message(
                                                format!("Failed to send message: {:?}", e),
                                                3000,
                                            );
                                            eprintln!("Failed to send message: {:?}", e);
                                        }
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
        }

        // WebSocket message reception
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
                            state.set_error_message(
                                format!("Unknown WS message: {}", unknown_msg),
                                3000,
                            );
                            eprintln!("Received unknown WebSocket message: {}", unknown_msg);
                        }
                    }
                }
                Ok(tungstenite::Message::Ping(_)) => {
                    if let Err(e) = ws_writer
                        .send(tungstenite::Message::Pong(vec![].into()))
                        .await
                    {
                        let mut state = app_state.lock().unwrap();
                        state.set_error_message(format!("Failed to send pong: {:?}", e), 3000);
                        eprintln!("Failed to send pong: {:?}", e);
                    }
                }
                Err(e) => {
                    let mut state = app_state.lock().unwrap();
                    state.set_error_message(format!("WebSocket error: {:?}", e), 5000);
                    eprintln!("WebSocket error: {:?}", e);
                    // Consider if you want to exit on WS error or attempt reconnect
                    // return Ok(TuiPage::Exit);
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
    create_channel_form: &CreateChannelForm,
) {
    let size = f.area();
    let mut state = app_state.lock().unwrap();

    // Main layout - no outer border on the main block
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(size); // Use full frame area, no margin here

    // Channels pane
    let channels_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded) // Rounded corners
        .title("Channels")
        .style(Style::default().fg(Color::Cyan));

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
                    .fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{} {}", channel.icon, channel.name)).style(style)
        })
        .collect();

    let channels_list = List::new(channel_items)
        .block(channels_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    f.render_stateful_widget(channels_list, chunks[0], channel_list_state);

    // Chat pane (messages + input)
    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(chunks[1]);

    // Messages box
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded) // Rounded corners
        .title(format!(
            "Messages - {}",
            state
                .current_channel
                .as_ref()
                .map_or("No Channel Selected".to_string(), |c| c.name.clone())
        ))
        .style(Style::default().fg(Color::White));

    let inner_messages_area = messages_block.inner(chat_chunks[0]);
    f.render_widget(messages_block, chat_chunks[0]);

    // Create a block to calculate message lines and scroll offset
    let (formatted_lines, new_scroll_offset) = {
        let mut formatted_lines: Vec<Line> = Vec::new();
        let mut new_scroll_offset = state.message_scroll_offset;

        if let Some(current_channel) = &state.current_channel {
            if let Some(messages) = state.get_messages_for_channel(&current_channel.id) {
                let mut last_user: Option<String> = None;

                for msg in messages.iter() {
                    let timestamp_str = Utc
                        .timestamp_opt(msg.timestamp, 0)
                        .unwrap()
                        .format("%H:%M")
                        .to_string();

                    let user_color = get_color_for_user(&msg.user);

                    if last_user.as_ref() == Some(&msg.user) {
                        for line in
                            textwrap::wrap(&msg.content, inner_messages_area.width as usize - 2)
                        {
                            formatted_lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                                Span::raw(line.to_string()),
                            ]));
                        }
                    } else {
                        let header_spans = vec![
                            Span::styled("╭ ", Style::default().fg(user_color)),
                            Span::styled(
                                format!("{} ", msg.icon),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                &msg.user,
                                Style::default().fg(user_color).add_modifier(Modifier::BOLD),
                            ),
                        ];

                        let header_width = header_spans.iter().map(|s| s.width()).sum::<usize>();
                        let available_width = inner_messages_area.width as usize;
                        let timestamp_width = timestamp_str.len();
                        let mut header_line_spans = header_spans;

                        if available_width > header_width + timestamp_width + 1 {
                            let padding = available_width
                                .saturating_sub(header_width)
                                .saturating_sub(timestamp_width);
                            header_line_spans.push(Span::raw(" ".repeat(padding)));
                            header_line_spans.push(Span::styled(
                                timestamp_str.clone(),
                                Style::default().fg(Color::DarkGray),
                            ));
                        } else {
                            header_line_spans.push(Span::raw(" "));
                            header_line_spans.push(Span::styled(
                                timestamp_str.clone(),
                                Style::default().fg(Color::DarkGray),
                            ));
                        }

                        formatted_lines.push(Line::from(header_line_spans));

                        for line in
                            textwrap::wrap(&msg.content, inner_messages_area.width as usize - 2)
                        {
                            formatted_lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                                Span::raw(line.to_string()),
                            ]));
                        }
                    }
                    last_user = Some(msg.user.clone());
                }
            } else {
                formatted_lines.push(Line::from("No messages in this channel yet."));
            }
        } else {
            formatted_lines.push(Line::from("Select a channel to see messages."));
        }

        let message_count = formatted_lines.len();
        if new_scroll_offset >= message_count {
            new_scroll_offset = message_count.saturating_sub(1);
        }

        (formatted_lines, new_scroll_offset)
    };

    let messages_to_render = {
        let message_count = formatted_lines.len();
        let view_height = inner_messages_area.height as usize;
        let scroll_offset = new_scroll_offset;

        let start_index = message_count.saturating_sub(view_height + scroll_offset);
        let end_index = message_count.saturating_sub(scroll_offset);

        if message_count > view_height {
            formatted_lines[start_index..end_index].to_vec()
        } else {
            formatted_lines
        }
    };

    let messages_paragraph =
        Paragraph::new(messages_to_render).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(messages_paragraph, inner_messages_area);

    state.message_scroll_offset = new_scroll_offset;

    // Input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded) // Rounded corners
        .title("Input")
        .style(Style::default().fg(Color::Yellow));

    let input_lines = input_text.split('\n').count();
    let input_height = (input_lines as u16 + 2).min(chat_chunks[1].height);
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(input_height),
        ])
        .split(chat_chunks[1])[1];

    let input_paragraph = Paragraph::new(Text::from(input_text.to_string())).block(input_block);
    f.render_widget(input_paragraph, input_area);

    // Render pop-up if active
    if state.popup_state.show {
        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Menu")
            .style(Style::default().fg(Color::LightCyan))
            .bg(Color::DarkGray);

        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(&popup_block, area);

        let popup_text = match state.popup_state.popup_type {
            PopupType::Quit => Paragraph::new(vec![
                Line::from(""),
                Line::from(Line::styled(
                    "  Are you sure you want to quit?",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Line::styled(
                    "  (Q)uit / (Esc) Cancel",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .alignment(Alignment::Center),
            PopupType::Settings => Paragraph::new(vec![
                Line::from(""),
                Line::from(Line::styled(
                    "  Settings options will go here.",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Line::styled(
                    "  (Esc) Cancel",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .alignment(Alignment::Center),
            PopupType::CreateChannel => {
                let name_style = if create_channel_form.input_focused == CreateChannelInput::Name {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let icon_style = if create_channel_form.input_focused == CreateChannelInput::Icon {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Line::styled(
                        "  Create New Channel",
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Name: ", name_style),
                        Span::styled(
                            create_channel_form.name.clone(),
                            name_style.add_modifier(Modifier::UNDERLINED),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Icon: ", icon_style),
                        Span::styled(
                            create_channel_form.icon.clone(),
                            icon_style.add_modifier(Modifier::UNDERLINED),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Line::styled(
                        "  (Enter) Create / (Tab) Switch Field / (Esc) Cancel",
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .alignment(Alignment::Left)
                .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)))
            }
            _ => Paragraph::new(""),
        };
        f.render_widget(popup_text, popup_block.inner(area));

        if state.popup_state.popup_type == PopupType::CreateChannel {
            let cursor_x;
            let cursor_y;
            let inner_area = popup_block.inner(area);

            match create_channel_form.input_focused {
                CreateChannelInput::Name => {
                    cursor_x = inner_area.x
                        + 2
                        + "  Name: ".len() as u16
                        + create_channel_form.name.len() as u16;
                    cursor_y = inner_area.y + 3;
                }
                CreateChannelInput::Icon => {
                    cursor_x = inner_area.x
                        + 2
                        + "  Icon: ".len() as u16
                        + create_channel_form.icon.len() as u16;
                    cursor_y = inner_area.y + 4;
                }
            }
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    if let Some(error_msg) = &state.error_message {
        let error_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Red).bg(Color::DarkGray));

        let error_paragraph = Paragraph::new(Line::from(error_msg.clone()))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(error_block);

        let error_width = (error_msg.len() + 4) as u16;
        let error_height = 3;

        let error_area = Rect::new(
            size.width.saturating_sub(error_width).saturating_sub(1),
            1,
            error_width,
            error_height,
        );
        f.render_widget(Clear, error_area);
        f.render_widget(error_paragraph, error_area);
    }

    if !state.popup_state.show {
        let input_cursor_x = chat_chunks[1].x + 1 + input_text.len() as u16;
        let input_cursor_y = chat_chunks[1].y + 1;
        f.set_cursor_position((input_cursor_x, input_cursor_y));
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}