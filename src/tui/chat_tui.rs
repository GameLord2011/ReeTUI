use crate::api::models::Channel;
use crate::api::websocket::{self, ServerMessage};
use crate::app::{AppState, PopupType};
use crate::tui::themes::{get_theme, rgb_to_color, ThemeName};
use crate::tui::TuiPage;
use chrono::{TimeZone, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::{SinkExt, StreamExt};
use ratatui::style::Stylize;
use ratatui::widgets::Clear;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    hash::{Hash, Hasher},
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;

fn get_color_for_user(username: &str) -> Color {
    let colors = [
        Color::Rgb(255, 0, 255),
        Color::Rgb(139, 0, 255),
        Color::Rgb(0, 191, 255),
        Color::Rgb(0, 255, 127),
        Color::Rgb(255, 215, 0),
        Color::Rgb(255, 105, 180),
        Color::Rgb(255, 69, 0),
        Color::Rgb(50, 205, 50),
    ];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    username.hash(&mut hasher);
    let hash = hasher.finish();
    colors[(hash % colors.len() as u64) as usize]
}

const ICONS: [&str; 11] = ["󰱨", "󰱩", "󱃞", "󰱫", "󰱬", "󰱮", "󰱰", "󰽌", "󰱱", "󰱸", "󰇹"];

#[derive(Default)]
struct CreateChannelForm {
    name: String,
    input_focused: CreateChannelInput,
    selected_icon_index: usize,
}

#[derive(PartialEq, Default, Clone, Copy)]
enum CreateChannelInput {
    #[default]
    Name,
    Icon,
    CreateButton, // a button to rule them all
}

impl CreateChannelForm {
    fn new() -> Self {
        Self {
            name: String::new(),
            input_focused: CreateChannelInput::Name,
            selected_icon_index: 0,
        }
    }

    fn next_input(&mut self) {
        self.input_focused = match self.input_focused {
            CreateChannelInput::Name => CreateChannelInput::Icon,
            CreateChannelInput::Icon => CreateChannelInput::CreateButton,
            CreateChannelInput::CreateButton => CreateChannelInput::Name, // Cycle back to Name
        };
    }

    fn previous_input(&mut self) {
        self.input_focused = match self.input_focused {
            CreateChannelInput::Name => CreateChannelInput::CreateButton,
            CreateChannelInput::Icon => CreateChannelInput::Name,
            CreateChannelInput::CreateButton => CreateChannelInput::Icon,
        };
    }

    fn next_icon(&mut self) {
        self.selected_icon_index = (self.selected_icon_index + 1) % ICONS.len();
    }

    fn previous_icon(&mut self) {
        self.selected_icon_index = (self.selected_icon_index + ICONS.len() - 1) % ICONS.len();
    }

    fn get_selected_icon(&self) -> String {
        ICONS[self.selected_icon_index].to_string()
    }
}

struct ThemeSettingsForm {
    selected_theme_index: usize,
    themes: Vec<ThemeName>,
    list_state: ListState,
}

impl ThemeSettingsForm {
    fn new(current_theme: ThemeName) -> Self {
        let themes = vec![
            ThemeName::Default,
            ThemeName::Oceanic,
            ThemeName::Forest,
            ThemeName::Monochrome,
            ThemeName::CatppuccinMocha,
            ThemeName::Dracula,
            ThemeName::SolarizedDark,
            ThemeName::GruvboxDark,
            ThemeName::Nord,
        ];
        let selected_theme_index = themes
            .iter()
            .position(|&t| format!("{:?}", t) == format!("{:?}", current_theme))
            .unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(selected_theme_index));

        Self {
            selected_theme_index,
            themes,
            list_state,
        }
    }

    fn next_theme(&mut self) {
        self.selected_theme_index = (self.selected_theme_index + 1) % self.themes.len();
        self.list_state.select(Some(self.selected_theme_index));
    }

    fn previous_theme(&mut self) {
        self.selected_theme_index =
            (self.selected_theme_index + self.themes.len() - 1) % self.themes.len();
        self.list_state.select(Some(self.selected_theme_index));
    }

    fn get_selected_theme(&self) -> ThemeName {
        self.themes[self.selected_theme_index]
    }
}

enum WsCommand {
    Message { channel_id: String, content: String },
    Pong,
}

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let mut create_channel_form = CreateChannelForm::new();
    let mut theme_settings_form = ThemeSettingsForm::new(app_state.lock().unwrap().current_theme);

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

    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<WsCommand>();

    // WebSocket writer task
    tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            match command {
                WsCommand::Message { channel_id, content } => {
                    if websocket::send_message(&mut ws_writer, &channel_id, &content)
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send message via websocket");
                        break;
                    }
                }
                WsCommand::Pong => {
                    if ws_writer
                        .send(tungstenite::Message::Pong(vec![].into()))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send pong");
                        break;
                    }
                }
            }
        }
    });

    // Request initial history for "home" channel
    if command_tx
        .send(WsCommand::Message {
            channel_id: "home".to_string(),
            content: "/get_history home".to_string(),
        })
        .is_err()
    {
        app_state
            .lock()
            .unwrap()
            .set_error_message("Failed to send command".to_string(), 3000);
    }

    loop {
        app_state.lock().unwrap().clear_expired_error();

        terminal.draw(|f| {
            draw_chat_ui::<B>(
                f,
                &mut app_state.lock().unwrap(),
                &input_text,
                &mut channel_list_state,
                &mut create_channel_form,
                &mut theme_settings_form,
            );
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut state = app_state.lock().unwrap();
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
                                KeyCode::Up => {
                                    state.selected_setting_index =
                                        state.selected_setting_index.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    state.selected_setting_index =
                                        (state.selected_setting_index + 1).min(2);
                                    // 0: Themes, 1: Deconnection, 2: Help
                                }
                                KeyCode::Enter => match state.selected_setting_index {
                                    0 => {
                                        state.popup_state.popup_type = PopupType::SetTheme;
                                        theme_settings_form =
                                            ThemeSettingsForm::new(state.current_theme);
                                    }
                                    1 => {
                                        state.popup_state.popup_type = PopupType::Deconnection;
                                    }
                                    2 => {
                                        state.popup_state.popup_type = PopupType::Help;
                                    }
                                    _ => {}
                                },
                                KeyCode::Esc => {
                                    state.popup_state.show = false;
                                    state.popup_state.popup_type = PopupType::None;
                                    state.selected_setting_index = 0; // Reset selected setting
                                }
                                _ => {}
                            },
                            PopupType::CreateChannel => {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.popup_state.show = false;
                                        state.popup_state.popup_type = PopupType::None;
                                        create_channel_form = CreateChannelForm::new();
                                    }
                                    KeyCode::Tab => {
                                        create_channel_form.next_input();
                                    }
                                    KeyCode::Up => {
                                        // Allow Up/Down to navigate fields
                                        create_channel_form.previous_input();
                                    }
                                    KeyCode::Down => {
                                        // Allow Up/Down to navigate fields
                                        create_channel_form.next_input();
                                    }
                                    KeyCode::Backspace => match create_channel_form.input_focused {
                                        CreateChannelInput::Name => {
                                            create_channel_form.name.pop();
                                        }
                                        _ => {} // Backspace does nothing for Icon or Button
                                    },
                                    KeyCode::Char(c) => match create_channel_form.input_focused {
                                        CreateChannelInput::Name => {
                                            create_channel_form.name.push(c);
                                        }
                                        _ => {} // Chars do nothing for Icon or Button
                                    },
                                    KeyCode::Left => {
                                        if create_channel_form.input_focused
                                            == CreateChannelInput::Icon
                                        {
                                            create_channel_form.previous_icon();
                                        }
                                    }
                                    KeyCode::Right => {
                                        if create_channel_form.input_focused
                                            == CreateChannelInput::Icon
                                        {
                                            create_channel_form.next_icon();
                                        }
                                    }
                                    KeyCode::Enter => {
                                        match create_channel_form.input_focused {
                                            CreateChannelInput::Name | CreateChannelInput::Icon => {
                                                create_channel_form.next_input();
                                                // Move to next field
                                            }
                                            CreateChannelInput::CreateButton => {
                                                if !create_channel_form.name.is_empty() {
                                                    let channel_name =
                                                        create_channel_form.name.clone();
                                                    let channel_icon =
                                                        create_channel_form.get_selected_icon();

                                                    let command = format!(
                                                        "/approve_channel {} {}",
                                                        channel_name, channel_icon
                                                    );
                                                    if command_tx
                                                        .send(WsCommand::Message {
                                                            channel_id: "home".to_string(),
                                                            content: command,
                                                        })
                                                        .is_err()
                                                    {
                                                        state.set_error_message(
                                                            format!(
                                                                "Failed to create channel"
                                                            ),
                                                            3000,
                                                        );
                                                    } else {
                                                        state.set_error_message(
                                                            format!(
                                                                "Channel '{}' created!",
                                                                channel_name
                                                            ),
                                                            3000,
                                                        );
                                                        state.popup_state.show = false;
                                                        state.popup_state.popup_type =
                                                            PopupType::None;
                                                        create_channel_form =
                                                            CreateChannelForm::new();
                                                    }
                                                } else {
                                                    state.set_error_message(
                                                        "Channel name cannot be empty!"
                                                            .to_string(),
                                                        3000,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            PopupType::SetTheme => match key.code {
                                KeyCode::Up => {
                                    theme_settings_form.previous_theme();
                                }
                                KeyCode::Down => {
                                    theme_settings_form.next_theme();
                                }
                                KeyCode::Enter => {
                                    state.current_theme = theme_settings_form.get_selected_theme();
                                    state.popup_state.show = false;
                                    state.popup_state.popup_type = PopupType::None;
                                }
                                KeyCode::Esc => {
                                    state.popup_state.show = false;
                                    state.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Deconnection => match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    state.clear_user_auth();
                                    return Ok(TuiPage::Auth);
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    state.popup_state.popup_type = PopupType::Settings;
                                }
                                _ => {}
                            },
                            PopupType::Help => {
                                // NEW: Help popup keybindings
                                match key.code {
                                    KeyCode::Esc => {
                                        state.popup_state.show = false;
                                        state.popup_state.popup_type = PopupType::None;
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
                                state.popup_state.show = true;
                                state.popup_state.popup_type = PopupType::Quit;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state.popup_state.show = true;
                                state.popup_state.popup_type = PopupType::Settings;
                            }
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state.popup_state.show = true;
                                state.popup_state.popup_type = PopupType::CreateChannel;
                                create_channel_form = CreateChannelForm::new();
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
                                    let channel_id = selected_channel.id.clone();
                                    let messages_loaded = state
                                        .messages
                                        .get(&channel_id)
                                        .map_or(false, |v| !v.is_empty());
                                    state.set_current_channel(selected_channel);

                                    if !messages_loaded {
                                        if command_tx
                                            .send(WsCommand::Message {
                                                channel_id: channel_id.clone(),
                                                content: format!("/get_history {}", channel_id),
                                            })
                                            .is_err()
                                        {
                                            state.set_error_message(
                                                "Failed to send command".to_string(),
                                                3000,
                                            );
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    state.scroll_messages_up();
                                } else {
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
                                        let channel_id = selected_channel.id.clone();
                                        let messages_loaded = state
                                            .messages
                                            .get(&channel_id)
                                            .map_or(false, |v| !v.is_empty());
                                        state.set_current_channel(selected_channel);

                                        if !messages_loaded {
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id: channel_id.clone(),
                                                    content: format!(
                                                        "/get_history {}",
                                                        channel_id
                                                    ),
                                                })
                                                .is_err()
                                            {
                                                state.set_error_message(
                                                    "Failed to send command".to_string(),
                                                    3000,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    state.scroll_messages_down();
                                } else {
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
                                        let channel_id = selected_channel.id.clone();
                                        let messages_loaded = state
                                            .messages
                                            .get(&channel_id)
                                            .map_or(false, |v| !v.is_empty());
                                        state.set_current_channel(selected_channel);

                                        if !messages_loaded {
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id: channel_id.clone(),
                                                    content: format!(
                                                        "/get_history {}",
                                                        channel_id
                                                    ),
                                                })
                                                .is_err()
                                            {
                                                state.set_error_message(
                                                    "Failed to send command".to_string(),
                                                    3000,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if !input_text.is_empty() {
                                    if let Some(current_channel) = &state.current_channel {
                                        let channel_id = current_channel.id.clone();
                                        let content = input_text.clone();
                                        if command_tx
                                            .send(WsCommand::Message { channel_id, content })
                                            .is_err()
                                        {
                                            state.set_error_message(
                                                "Failed to send message".to_string(),
                                                3000,
                                            );
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
                    if command_tx.send(WsCommand::Pong).is_err() {
                        let mut state = app_state.lock().unwrap();
                        state.set_error_message("Failed to send pong command".to_string(), 3000);
                        eprintln!("Failed to send pong command");
                    }
                }
                Err(e) => {
                    let mut state = app_state.lock().unwrap();
                    state.set_error_message(format!("WebSocket error: {:?}", e), 5000);
                    eprintln!("WebSocket error: {:?}", e);
                }
                _ => {}
            }
        }
    }
}

fn draw_chat_ui<B: Backend>(
    f: &mut Frame,
    state: &mut AppState,
    input_text: &str,
    channel_list_state: &mut ListState,
    create_channel_form: &mut CreateChannelForm,
    theme_settings_form: &mut ThemeSettingsForm,
) {
    let size = f.area();
    let current_theme = get_theme(state.current_theme);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(size);

    let channels_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Channels")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.border_focus))
                .bg(rgb_to_color(&current_theme.background)),
        );

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
                    .fg(rgb_to_color(&current_theme.accent))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(format!("{} {}", channel.icon, channel.name)).style(style)
        })
        .collect();

    let channels_list = List::new(channel_items)
        .block(channels_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(channels_list, chunks[0], channel_list_state);

    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(chunks[1]);

    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            "Messages - {}",
            state
                .current_channel
                .as_ref()
                .map_or("No Channel Selected".to_string(), |c| c.name.clone())
        ))
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.text))
                .bg(rgb_to_color(&current_theme.background)),
        );

    let inner_messages_area = messages_block.inner(chat_chunks[0]);
    f.render_widget(messages_block, chat_chunks[0]);

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
                                Span::styled(
                                    "│ ",
                                    Style::default().fg(rgb_to_color(&current_theme.dim)),
                                ),
                                Span::raw(line.to_string()).fg(rgb_to_color(&current_theme.text)),
                            ]));
                        }
                    } else {
                        let header_spans = vec![
                            Span::styled("╭ ", Style::default().fg(user_color)),
                            Span::styled(
                                format!("{} ", msg.icon),
                                Style::default().fg(rgb_to_color(&current_theme.text)),
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
                                Style::default().fg(rgb_to_color(&current_theme.dim)),
                            ));
                        } else {
                            header_line_spans.push(Span::raw(" "));
                            header_line_spans.push(Span::styled(
                                timestamp_str.clone(),
                                Style::default().fg(rgb_to_color(&current_theme.dim)),
                            ));
                        }

                        formatted_lines.push(Line::from(header_line_spans));

                        for line in
                            textwrap::wrap(&msg.content, inner_messages_area.width as usize - 2)
                        {
                            formatted_lines.push(Line::from(vec![
                                Span::styled(
                                    "│ ",
                                    Style::default().fg(rgb_to_color(&current_theme.dim)),
                                ),
                                Span::raw(line.to_string()).fg(rgb_to_color(&current_theme.text)),
                            ]));
                        }
                    }
                    last_user = Some(msg.user.clone());
                }
            } else {
                formatted_lines.push(Line::from(Span::styled(
                    "No messages in this channel yet.",
                    Style::default().fg(rgb_to_color(&current_theme.dim)),
                )));
            }
        } else {
            formatted_lines.push(Line::from(Span::styled(
                "Select a channel to see messages.",
                Style::default().fg(rgb_to_color(&current_theme.dim)),
            )));
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

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Input")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.input_border_active))
                .bg(rgb_to_color(&current_theme.background)),
        );

    let input_lines = input_text.split('\n').count();
    let input_height = (input_lines as u16 + 2).min(chat_chunks[1].height);
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(input_height)])
        .split(chat_chunks[1])[1];

    let input_paragraph = Paragraph::new(Text::from(input_text.to_string()))
        .block(input_block)
        .style(Style::default().fg(rgb_to_color(&current_theme.input_text_active)));
    f.render_widget(input_paragraph, input_area);

    if state.popup_state.show {
        let popup_title = match state.popup_state.popup_type {
            PopupType::Quit => "Quit",
            PopupType::Settings => "Settings",
            PopupType::CreateChannel => "Create Channel",
            PopupType::SetTheme => "Select Theme",
            PopupType::Deconnection => "Deconnection",
            PopupType::Help => "Help - Commands", // NEW: Help popup title
            PopupType::None => "",                // Should not be displayed
        };

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(popup_title)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.popup_border))
                    .bg(rgb_to_color(&current_theme.background)),
            );

        let area = match state.popup_state.popup_type {
            PopupType::Quit | PopupType::Deconnection => {
                let width = (size.width as f32 * 0.60) as u16;
                let height = (size.height as f32 * 0.20) as u16;
                centered_rect(width, height, size)
            }
            PopupType::Settings => {
                let width = (size.width as f32 * 0.50) as u16;
                let height = (size.height as f32 * 0.40) as u16;
                centered_rect(width, height, size)
            }
            PopupType::CreateChannel => {
                let required_width = (ICONS.len() * 3) as u16 + 2 + 2; // Icon selector width + margins + borders
                let required_height = 14; // 3 (name) + 3 (icon) + 1 (spacer) + 3 (button) + 2 (form_layout margins) + 2 (popup_block borders)
                centered_rect(required_width, required_height, size)
            }
            PopupType::SetTheme => {
                let required_width = (ICONS.len() * 3) as u16 + 2 + 2; // Icon selector width + margins + borders
                let num_themes = theme_settings_form.themes.len() as u16;
                let required_height = num_themes + 6; // Title (1) + List (num_themes + 2 for padding) + Hint (2) + margins (1)
                centered_rect(required_width, required_height, size)
            }
            PopupType::Help => {
                let width = (size.width as f32 * 0.70) as u16;
                let height = (size.height as f32 * 0.70) as u16;
                centered_rect(width, height, size)
            }
            PopupType::None => Rect::default(),
        };

        f.render_widget(Clear, area);
        f.render_widget(&popup_block, area);

        match state.popup_state.popup_type {
            PopupType::Quit => {
                draw_quit_popup(f, state, area, &popup_block);
            }
            PopupType::Settings => {
                draw_settings_popup(f, state, area, &popup_block);
            }
            PopupType::CreateChannel => {
                draw_create_channel_popup(f, state, area, create_channel_form, &popup_block);
            }
            PopupType::SetTheme => {
                draw_set_theme_popup(f, state, area, theme_settings_form, &popup_block);
            }
            PopupType::Deconnection => {
                draw_deconnection_popup(f, state, area, &popup_block);
            }
            PopupType::Help => {
                draw_help_popup(f, state, area, &popup_block);
            }
            _ => { /* No specific rendering for other popup types yet */ }
        }
    }

    if let Some(error_msg) = &state.error_message {
        let error_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.error))
                    .bg(rgb_to_color(&current_theme.background)),
            );

        let error_paragraph = Paragraph::new(Line::from(error_msg.clone()))
            .style(Style::default().fg(rgb_to_color(&current_theme.text)))
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

fn draw_help_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let commands = vec![
        "General:",
        "  :                    - Open Quit popup",
        "  Ctrl+S               - Open Settings popup",
        "  Ctrl+N               - Open Create Channel popup",
        "  Tab                  - Switch to next channel",
        "  Ctrl+Up/Down         - Scroll messages",
        "  Up/Down              - Switch channels",
        "  Enter                - Send message",
        "  Backspace            - Delete last char in input",
        "",
        "Popups (varies per popup):",
        "  Esc                  - Close popup / Cancel",
        "  Enter                - Confirm / Select / Create",
        "  Tab/Up/Down          - Navigate fields/options (in forms)",
        "  Left/Right           - Select icon (in Create Channel)",
        "  Q/q (Quit popup)     - Confirm quit",
        "  Y/y (Deconn popup)   - Confirm deconnection",
        "  N/n (Deconn popup)   - Cancel deconnection",
        "  T/t (Settings)       - Open Themes",
        "  D/d (Settings)       - Open Deconnection",
        "  H/h (Settings)       - Open Help (this page)",
    ];

    let formatted_commands: Vec<Line> = commands
        .iter()
        .map(|&s| {
            Line::from(Span::styled(
                s,
                Style::default().fg(rgb_to_color(&current_theme.text)),
            ))
        })
        .collect();

    let commands_paragraph = Paragraph::new(formatted_commands)
        .alignment(Alignment::Left)
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 2, 2))); // Add padding inside popup

    f.render_widget(commands_paragraph, popup_block.inner(area));
}

fn draw_quit_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "  Are you sure you want to quit?",
            Style::default().fg(rgb_to_color(&current_theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            "  (Q)uit / (Esc) Cancel",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}

fn draw_deconnection_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "  Are you sure you want to disconnect?",
            Style::default().fg(rgb_to_color(&current_theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            "  (Y)es / (N)o / (Esc) Cancel",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}

fn draw_set_theme_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    theme_settings_form: &mut ThemeSettingsForm,
    popup_block: &Block,
) {
    let current_theme = get_theme(state.current_theme);
    let theme_items: Vec<ListItem> = theme_settings_form
        .themes
        .iter()
        .enumerate()
        .map(|(i, &theme_name)| {
            let is_selected = i == theme_settings_form.selected_theme_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(format!("{:?}", theme_name)).style(style)
        })
        .collect();

    let inner_area = popup_block.inner(area);

    // Calculate dynamic height for the list
    let num_themes = theme_settings_form.themes.len() as u16;
    let required_list_height = num_themes + 2; // 2 for top/bottom padding of the list block

    // Define overall layout for the popup content
    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                    // Title
            Constraint::Length(required_list_height), // Theme list
            Constraint::Min(0),                       // Spacer
            Constraint::Length(2),                    // Hint (including empty line)
        ])
        .margin(1) // Margin inside the popup_block.inner(area)
        .split(inner_area);

    // Render Title
    let title_paragraph = Paragraph::new(Text::styled(
        "Select Theme",
        Style::default()
            .fg(rgb_to_color(&current_theme.accent))
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);
    f.render_widget(title_paragraph, content_layout[0]);

    // Render Theme List
    let theme_list_width = (ICONS.len() * 3) as u16; // Match width of icon selector
    let list_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(theme_list_width),
            Constraint::Min(0),
        ])
        .split(content_layout[1]); // Render in the theme list area

    let theme_list_block = Block::default().border_type(BorderType::Rounded).style(
        Style::default()
            .fg(rgb_to_color(&current_theme.popup_border))
            .bg(rgb_to_color(&current_theme.background)),
    );

    let theme_list = List::new(theme_items)
        .block(theme_list_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active)),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(
        theme_list,
        list_area_h[1],
        &mut theme_settings_form.list_state,
    );

    // Render Hint
    let hint_paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "  (Up/Down) Navigate / (Enter) Select / (Esc) Cancel",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hint_paragraph, content_layout[3]);
}

fn draw_settings_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let settings_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Min(0),    // Options
            Constraint::Length(1), // Hint
        ])
        .margin(1)
        .split(inner_area);

    let title_paragraph = Paragraph::new(Text::styled(
        "Settings",
        Style::default()
            .fg(rgb_to_color(&current_theme.accent))
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);
    f.render_widget(title_paragraph, settings_layout[0]);

    let options = ["Themes", "Deconnection", "Help / Commands"];
    let option_items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, &option)| {
            let is_selected = i == state.selected_setting_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .bg(rgb_to_color(&current_theme.button_bg_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(option).style(style)
        })
        .collect();

    let options_list = List::new(option_items)
        .block(Block::default())
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(">> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_setting_index));
    f.render_stateful_widget(options_list, settings_layout[1], &mut list_state);

    let hint_paragraph = Paragraph::new(Text::styled(
        "  (Esc) Cancel",
        Style::default().fg(rgb_to_color(&current_theme.accent)),
    ))
    .alignment(Alignment::Center);
    f.render_widget(hint_paragraph, settings_layout[2]);
}

fn draw_create_channel_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    create_channel_form: &mut CreateChannelForm,
    popup_block: &Block,
) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let form_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // For Name input
            Constraint::Length(3), // For Icon input/selector
            Constraint::Length(1), // Spacer for button
            Constraint::Length(3), // For Create button
            Constraint::Min(0),    // Spacer
        ])
        .margin(1)
        .split(inner_area);

    let icons_row_width = (ICONS.len() * 3) as u16; // 2 chars + 1 space per icon

    // Name Input
    let name_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(icons_row_width),
            Constraint::Min(0),
        ])
        .split(form_layout[0]);

    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Channel Name")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_border_inactive))
            },
        );
    let name_paragraph = Paragraph::new(create_channel_form.name.clone())
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.input_text_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_text_inactive))
            },
        )
        .block(name_block);
    f.render_widget(name_paragraph, name_area_h[1]);

    // Icon Input/Selector

    let icon_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Channel Icon")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Icon {
                Style::default().fg(rgb_to_color(&current_theme.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_border_inactive))
            },
        );

    let icon_selector_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(icons_row_width),
            Constraint::Min(0),
        ])
        .split(form_layout[1]);

    let len = ICONS.len();
    let center = create_channel_form.selected_icon_index;
    let display_range = 3; // Number of icons to show on each side of the center

    let mut spans = Vec::with_capacity(display_range * 2 + 1);

    for i in
        (center as isize - display_range as isize)..(center as isize + display_range as isize + 1)
    {
        let actual_index = (i % len as isize + len as isize) % len as isize;
        let icon_char = ICONS[actual_index as usize];
        if actual_index == center as isize {
            spans.push(Span::styled(
                icon_char,
                Style::default()
                    .fg(rgb_to_color(&current_theme.accent))
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(
                icon_char,
                Style::default().fg(rgb_to_color(&current_theme.dim)),
            ));
        }
        if i != center as isize + display_range as isize {
            spans.push(Span::raw("   "));
        }
    }

    let icon_paragraph = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(icon_block);
    f.render_widget(icon_paragraph, icon_selector_area_h[1]);

    // Create Button
    let create_button_style =
        if create_channel_form.input_focused == CreateChannelInput::CreateButton {
            Style::default().fg(rgb_to_color(&current_theme.button_text_active))
        } else {
            Style::default().fg(rgb_to_color(&current_theme.button))
        };
    let create_button_paragraph =
        Paragraph::new(Text::styled("  Create Channel  ", create_button_style))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(create_button_style),
            ); // Apply border style based on focus

    let button_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(20), // Width of the button
            Constraint::Min(0),
        ])
        .split(form_layout[3]); // Place in the button row

    f.render_widget(create_button_paragraph, button_area_h[1]);

    // Cursor positioning
    if create_channel_form.input_focused == CreateChannelInput::Name {
        let cursor_x = name_area_h[1].x + 1 + create_channel_form.name.len() as u16;
        let cursor_y = name_area_h[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = (r.width.saturating_sub(width)) / 2;
    let y = (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}