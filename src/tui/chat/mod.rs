pub mod create_channel_form;
pub mod gif_renderer;
pub mod image_handler;
pub mod message_parsing;
pub mod popups;
pub mod theme_settings_form;
pub mod ui;
pub mod utils;
pub mod ws_command;

#[cfg(test)]
pub mod tests;

use crate::api::models::BroadcastMessage;
use crate::api::websocket; // ServerMessage unused
use crate::app::{AppState, NotificationType, PopupType};
use crate::tui::chat::create_channel_form::{CreateChannelForm, CreateChannelInput};
use crate::tui::chat::message_parsing::{
    replace_shortcodes_with_emojis, should_show_emoji_popup, should_show_mention_popup,
};
use crate::tui::chat::theme_settings_form::ThemeSettingsForm;
use crate::tui::chat::ui::draw_chat_ui;
use crate::tui::chat::ws_command::WsCommand;
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use emojis;
use ratatui::{prelude::Backend, widgets::ListState, Terminal}; // All used
use regex::Regex; // Used for mention/emoji regex
use serde_json; // Used for debug JSON
use std::{io, path::PathBuf, sync::Arc, time::Duration}; // All used
use tokio::sync::mpsc;
use unicode_segmentation::UnicodeSegmentation;

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<Option<TuiPage>> {
    log::debug!("chat_mod: run_chat_page started.");
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let mut create_channel_form = CreateChannelForm::new();
    let mut theme_settings_form = ThemeSettingsForm::new(app_state.lock().await.current_theme);
    let mut file_manager = popups::file_manager::FileManager::new(
        popups::file_manager::FileManagerMode::LocalUpload,
        Vec::new(),
    );

    let (mut ws_writer, ws_reader) = {
        let state = app_state.lock().await;
        let token = state
            .auth_token
            .clone()
            .expect("Auth token not found for WebSocket connection");
        websocket::connect(&token)
            .await
            .expect("Failed to connect to WebSocket")
    };

    // No need to request channel list after connecting; server sends it automatically

    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (filecommand_tx, mut file_command_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<(String, u8)>();
    let http_client = reqwest::Client::new();

    // --- WebSocket and File Command Handling Tasks (Unchanged) ---
    tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            match command {
                WsCommand::Message {
                    channel_id,
                    content,
                } => {
                    if websocket::send_message(&mut ws_writer, &channel_id, &content)
                        .await
                        .is_err()
                    {
                        log::error!("Failed to send message via websocket");
                        break;
                    }
                }
                // Other WsCommand variants if any, removed for brevity in example
                _ => {}
            }
        }
    });

    // --- File Command Handling Task: handle file uploads/downloads ---
    let app_state_clone2 = app_state.clone();
    let http_client_for_file_commands = http_client.clone();
    let progress_tx2 = progress_tx.clone();
    tokio::spawn(async move {
        use crate::api::file_api;
        while let Some(command) = file_command_rx.recv().await {
            match command {
                WsCommand::UploadFile {
                    channel_id,
                    file_path,
                } => {
                    let token = {
                        let state = app_state_clone2.lock().await;
                        state.auth_token.clone()
                    };
                    if let Some(token) = token {
                        match file_api::upload_file(
                            &http_client_for_file_commands,
                            &token,
                            &channel_id,
                            file_path,
                            progress_tx2.clone(),
                        )
                        .await
                        {
                            Ok(_file_id) => {
                                let mut state = app_state_clone2.lock().await;
                                state.set_notification(
                                    "File Upload Success".to_string(),
                                    "File uploaded successfully!".to_string(),
                                    NotificationType::Success,
                                    3,
                                );
                                // Optionally, refresh channel messages here
                            }
                            Err(e) => {
                                let mut state = app_state_clone2.lock().await;
                                state.set_notification(
                                    "File Upload Error".to_string(),
                                    format!("Failed to upload file: {}", e),
                                    NotificationType::Error,
                                    3,
                                );
                            }
                        }
                    }
                }
                WsCommand::DownloadFile { file_id, file_name } => {
                    let app_state_clone3 = app_state_clone2.clone();
                    let progress_tx3 = progress_tx2.clone();
                    let http_client_clone = http_client_for_file_commands.clone();
                    tokio::spawn(async move {
                        match file_api::download_file(
                            &http_client_clone,
                            &file_id,
                            &file_name,
                            progress_tx3.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                let mut state = app_state_clone3.lock().await;
                                state.set_notification(
                                    "File Download Success".to_string(),
                                    format!("File '{}' downloaded successfully!", file_name),
                                    NotificationType::Success,
                                    3,
                                );
                            }
                            Err(e) => {
                                let mut state = app_state_clone3.lock().await;
                                state.set_notification(
                                    "File Download Error".to_string(),
                                    format!("Failed to download file: {}", e),
                                    NotificationType::Error,
                                    3,
                                );
                            }
                        }
                    });
                }
                _ => {}
            }
        }
    });

    // Handle download progress updates
    let app_state_clone_for_progress = app_state.clone();
    tokio::spawn(async move {
        while let Some((_file_id, progress)) = progress_rx.recv().await {
            let mut state = app_state_clone_for_progress.lock().await;
            state.set_download_progress_popup(progress);
            if progress == 100 {
                state.popup_state.show = false;
                state.popup_state.popup_type = PopupType::None;
            }
        }
    });

    // --- WebSocket Reader Task: handle incoming messages (including ChannelList) ---
    let app_state_clone = app_state.clone();
    let command_tx_bg = command_tx.clone();
    let http_client_for_websocket_reader = http_client.clone();
    tokio::spawn(async move {
        use crate::api::websocket::ServerMessage;
        use futures_util::StreamExt;
        let mut ws_reader = ws_reader;
        while let Some(Ok(msg)) = ws_reader.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                    let mut state = app_state_clone.lock().await;
                    match server_msg {
                        ServerMessage::ChannelList(wrapper) => {
                            state.channels = wrapper.channels;
                            // Auto-select the first channel and request its history
                            if let Some(first_channel) = state.channels.get(0).cloned() {
                                let channel_id = first_channel.id.clone();
                                state.set_current_channel(first_channel);
                                // Send a history request for the first channel
                                let _ = command_tx_bg.send(
                                    crate::tui::chat::ws_command::WsCommand::Message {
                                        channel_id: channel_id.clone(),
                                        content: format!("/get_history {} 0", channel_id),
                                    },
                                );
                            }
                        }
                        ServerMessage::Broadcast(message) => {
                            let is_image = message.is_image.unwrap_or(false);
                            state.add_message(message.clone());
                            if is_image {
                                let app_state_clone = app_state_clone.clone();
                                let http_client = http_client_for_websocket_reader.clone();
                                let msg = message.clone();
                                tokio::spawn(async move {
                                    // Use a default height, e.g., 20 lines
                                    crate::tui::chat::image_handler::process_image_message(
                                        app_state_clone,
                                        msg,
                                        &http_client,
                                        20,
                                    )
                                    .await;
                                });
                            }
                        }
                        ServerMessage::History(wrapper) => {
                            let history = wrapper.history;
                            let channel_id = history.channel_id.clone();
                            let messages = history.messages.clone();
                            state.prepend_history(&channel_id, messages.clone());
                            state
                                .channel_history_state
                                .insert(channel_id, (history.offset, history.has_more));
                            // For each image message in history, spawn a chafa conversion
                            for message in messages {
                                if message.is_image.unwrap_or(false) {
                                    let app_state_clone = app_state_clone.clone();
                                    let http_client = http_client_for_websocket_reader.clone();
                                    let msg = message.clone();
                                    tokio::spawn(async move {
                                        crate::tui::chat::image_handler::process_image_message(
                                            app_state_clone,
                                            msg,
                                            &http_client,
                                            20,
                                        )
                                        .await;
                                    });
                                }
                            }
                        }
                        ServerMessage::UserList(wrapper) => {
                            state.active_users = wrapper.users;
                        }
                        ServerMessage::ChannelUpdate(channel) => {
                            state.add_or_update_channel(channel);
                        }
                        ServerMessage::Notification {
                            title,
                            message,
                            notification_type,
                        } => {
                            state.set_notification(title, message, notification_type, 3);
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Main event loop
    loop {
        let mut state_guard = app_state.lock().await;

        // Prepare filtered users and emojis for UI
        let filtered_users: Vec<String> = state_guard.active_users.clone();
        let filtered_emojis: Vec<String> = emojis::iter().map(|e| e.to_string()).collect();
        let mention_regex = Regex::new(r"@[a-zA-Z0-9_]+( -)?").unwrap();
        let emoji_regex = Regex::new(r":[a-zA-Z0-9_]+:").unwrap();

        terminal.draw(|f| {
            draw_chat_ui::<B>(
                f,
                &mut state_guard,
                &input_text,
                &mut channel_list_state,
                &mut create_channel_form,
                &mut theme_settings_form,
                &mut file_manager,
                &filtered_users,
                &filtered_emojis,
                &mention_regex,
                &emoji_regex,
            );
        })?;

        // --- GIF Animation Frame Advancement ---
        let now = std::time::Instant::now();
        let mut needs_redraw = false;
        for (_file_id, anim_arc) in state_guard.active_animations.iter_mut() {
            if let Ok(mut anim) = anim_arc.lock() {
                if anim.frames.len() > 1 {
                    if anim.last_frame_time.is_none() {
                        anim.last_frame_time = Some(now);
                    }

                    let last = anim.last_frame_time.unwrap();
                    let delay = anim.delays.get(anim.current_frame).copied().unwrap_or(100) as u64;

                    if now.duration_since(last).as_millis() >= delay.into() {
                        anim.current_frame = (anim.current_frame + 1) % anim.frames.len();
                        anim.last_frame_time = Some(now);
                        needs_redraw = true;
                    }
                }
            }
        }
        if needs_redraw {
            terminal.draw(|f| {
                draw_chat_ui::<B>(
                    f,
                    &mut state_guard,
                    &input_text,
                    &mut channel_list_state,
                    &mut create_channel_form,
                    &mut theme_settings_form,
                    &mut file_manager,
                    &filtered_users,
                    &filtered_emojis,
                    &mention_regex,
                    &emoji_regex,
                );
            })?;
        }

        let timeout = Duration::from_millis(16);
        if !event::poll(timeout)? {
            continue;
        }

        let event = event::read()?;

        // Handle mouse scroll events
        if let Event::Mouse(mouse_event) = event {
            match mouse_event.kind {
                MouseEventKind::ScrollUp => {
                    // Scroll up messages
                    let rendered_count = state_guard
                        .rendered_messages
                        .get(
                            &state_guard
                                .current_channel
                                .as_ref()
                                .map(|c| c.id.clone())
                                .unwrap_or_default(),
                        )
                        .map_or(0, |v| v.len());
                    let view_height = state_guard.last_chat_view_height.max(1);
                    state_guard.scroll_messages_up(rendered_count, view_height);
                }
                MouseEventKind::ScrollDown => {
                    state_guard.scroll_messages_down();
                }
                _ => {}
            }
            continue;
        }

        // Handle global key events, even when a popup is not active
        if let Event::Key(key) = event {
            match key.kind {
                KeyEventKind::Press => {
                    // Only handle actions on key press

                    let current_popup_type = state_guard.popup_state.popup_type;

                    match current_popup_type {
                        PopupType::Quit => {
                            log::debug!("PopupType::Quit branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    return Ok(Some(TuiPage::Exit)); // Return Option<TuiPage> as per new signature
                                }
                                KeyCode::Esc => {
                                    log::debug!("PopupType::Quit dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            }
                        }
                        PopupType::Settings => {
                            log::debug!("PopupType::Settings branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Up => {
                                    state_guard.selected_setting_index =
                                        state_guard.selected_setting_index.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    state_guard.selected_setting_index =
                                        (state_guard.selected_setting_index + 1).min(2);
                                }
                                KeyCode::Enter => match state_guard.selected_setting_index {
                                    0 => {
                                        state_guard.popup_state.popup_type = PopupType::SetTheme;
                                        theme_settings_form =
                                            ThemeSettingsForm::new(state_guard.current_theme);
                                    }
                                    1 => {
                                        state_guard.popup_state.popup_type =
                                            PopupType::Deconnection;
                                    }
                                    2 => {
                                        state_guard.popup_state.popup_type = PopupType::Help;
                                    }
                                    _ => {}
                                },
                                KeyCode::Esc => {
                                    log::debug!("PopupType::Settings dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_setting_index = 0;
                                }
                                _ => {}
                            }
                        }
                        PopupType::CreateChannel => {
                            log::debug!("PopupType::CreateChannel branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Esc => {
                                    log::debug!("PopupType::CreateChannel dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    create_channel_form = CreateChannelForm::new();
                                }
                                KeyCode::Tab => {
                                    create_channel_form.next_input();
                                }
                                KeyCode::Up => {
                                    create_channel_form.previous_input();
                                }
                                KeyCode::Down => {
                                    create_channel_form.next_input();
                                }
                                KeyCode::Backspace => match create_channel_form.input_focused {
                                    CreateChannelInput::Name => {
                                        create_channel_form.name.pop();
                                    }
                                    _ => {}
                                },
                                KeyCode::Char(c) => match create_channel_form.input_focused {
                                    CreateChannelInput::Name => {
                                        create_channel_form.name.push(c);
                                    }
                                    _ => {}
                                },
                                KeyCode::Left => {
                                    if create_channel_form.input_focused == CreateChannelInput::Icon
                                    {
                                        create_channel_form.previous_icon();
                                    }
                                }
                                KeyCode::Right => {
                                    if create_channel_form.input_focused == CreateChannelInput::Icon
                                    {
                                        create_channel_form.next_icon();
                                    }
                                }
                                KeyCode::Enter => match create_channel_form.input_focused {
                                    CreateChannelInput::Name | CreateChannelInput::Icon => {
                                        create_channel_form.next_input();
                                    }
                                    CreateChannelInput::CreateButton => {
                                        if !create_channel_form.name.is_empty() {
                                            let channel_name = create_channel_form.name.clone();
                                            let channel_icon =
                                                create_channel_form.get_selected_icon();
                                            let command = format!(
                                                "/propose_channel {} {}",
                                                channel_name, channel_icon
                                            );
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id: "home".to_string(),
                                                    content: command,
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_notification(
                                                    "Channel Creation Error".to_string(),
                                                    "Failed to create channel".to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            } else {
                                                state_guard.set_notification(
                                                    "Channel Creation Success".to_string(),
                                                    format!("Channel '{}' created!", channel_name),
                                                    NotificationType::Success,
                                                    3,
                                                );
                                                state_guard.popup_state.show = false;
                                                state_guard.popup_state.popup_type =
                                                    PopupType::None;
                                                create_channel_form = CreateChannelForm::new();
                                            }
                                        } else {
                                            state_guard.set_notification(
                                                "Channel Creation Warning".to_string(),
                                                "Channel name cannot be empty!".to_string(),
                                                NotificationType::Warning,
                                                3,
                                            );
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                        PopupType::SetTheme => {
                            log::debug!("PopupType::SetTheme branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Up => {
                                    theme_settings_form.previous_theme();
                                    state_guard.current_theme =
                                        theme_settings_form.get_selected_theme();
                                }
                                KeyCode::Down => {
                                    theme_settings_form.next_theme();
                                    state_guard.current_theme =
                                        theme_settings_form.get_selected_theme();
                                }
                                KeyCode::Enter => {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                KeyCode::Esc => {
                                    log::debug!("PopupType::SetTheme dismissed with Esc");
                                    state_guard.current_theme = theme_settings_form.original_theme;
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            }
                        }
                        PopupType::Deconnection => {
                            log::debug!("PopupType::Deconnection branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    state_guard.clear_user_auth();
                                    return Ok(Some(TuiPage::Auth)); // return Option<TuiPage> as per new signature
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    log::debug!("PopupType::Deconnection dismissed with Esc or N");
                                    state_guard.popup_state.popup_type = PopupType::Settings;
                                }
                                _ => {}
                            }
                        }
                        PopupType::Help => {
                            log::debug!("PopupType::Help branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Esc => {
                                    log::debug!("PopupType::Help dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            }
                        }
                        PopupType::Mentions => {
                            log::debug!("PopupType::Mentions branch, key: {:?}", key.code);
                            let filtered_users: Vec<String> = state_guard
                                .active_users
                                .iter()
                                .filter(|user| {
                                    user.to_lowercase()
                                        .contains(&state_guard.mention_query.to_lowercase())
                                })
                                .cloned()
                                .collect();
                            let num_filtered_users = filtered_users.len();
                            match key.code {
                                KeyCode::Up => {
                                    if num_filtered_users > 0 {
                                        state_guard.selected_mention_index = (state_guard
                                            .selected_mention_index
                                            + num_filtered_users
                                            - 1)
                                            % num_filtered_users;
                                    } else {
                                        state_guard.selected_mention_index = 0;
                                    }
                                }
                                KeyCode::Down => {
                                    if num_filtered_users > 0 {
                                        state_guard.selected_mention_index =
                                            (state_guard.selected_mention_index + 1)
                                                % num_filtered_users;
                                    } else {
                                        state_guard.selected_mention_index = 0;
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(user) =
                                        filtered_users.get(state_guard.selected_mention_index)
                                    {
                                        let query_start =
                                            input_text.rfind('@').unwrap_or(input_text.len());
                                        input_text
                                            .replace_range(query_start.., &format!("@{} ", user));
                                        state_guard.cursor_position = input_text.len();
                                    }
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_mention_index = 0;
                                    state_guard.mention_query.clear();
                                }
                                KeyCode::Esc => {
                                    log::debug!("PopupType::Mentions dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_mention_index = 0;
                                    state_guard.mention_query.clear();
                                }
                                KeyCode::Backspace => {
                                    if state_guard.cursor_position > 0 {
                                        let old_pos = state_guard.cursor_position;
                                        let new_pos = input_text[..old_pos]
                                            .grapheme_indices(true)
                                            .last()
                                            .map(|(i, _)| i)
                                            .unwrap_or(0);
                                        input_text.replace_range(new_pos..old_pos, "");
                                        state_guard.cursor_position = new_pos;
                                    }
                                    if let Some(last_at_idx) = input_text.rfind('@') {
                                        state_guard.mention_query =
                                            input_text[last_at_idx + 1..].to_string();
                                    } else {
                                        state_guard.mention_query.clear();
                                    }
                                    let new_filtered_users: Vec<String> = state_guard
                                        .active_users
                                        .iter()
                                        .filter(|user| {
                                            user.to_lowercase()
                                                .contains(&state_guard.mention_query.to_lowercase())
                                        })
                                        .cloned()
                                        .collect();
                                    let new_num_filtered_users = new_filtered_users.len();
                                    if new_num_filtered_users > 0 {
                                        state_guard.selected_mention_index = state_guard
                                            .selected_mention_index
                                            .min(new_num_filtered_users.saturating_sub(1));
                                    } else {
                                        state_guard.selected_mention_index = 0;
                                    }
                                    if !should_show_mention_popup(&input_text) {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                        state_guard.mention_query.clear();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    input_text.insert(state_guard.cursor_position, c);
                                    state_guard.cursor_position += c.len_utf8();
                                    state_guard.mention_query.push(c);
                                    let new_filtered_users: Vec<String> = state_guard
                                        .active_users
                                        .iter()
                                        .filter(|user| {
                                            user.to_lowercase()
                                                .contains(&state_guard.mention_query.to_lowercase())
                                        })
                                        .cloned()
                                        .collect();
                                    let new_num_filtered_users = new_filtered_users.len();
                                    if new_num_filtered_users > 0 {
                                        state_guard.selected_mention_index = state_guard
                                            .selected_mention_index
                                            .min(new_num_filtered_users.saturating_sub(1));
                                    } else {
                                        state_guard.selected_mention_index = 0;
                                    }
                                    if !should_show_mention_popup(&input_text) {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                        state_guard.mention_query.clear();
                                    }
                                }
                                _ => {}
                            }
                        }
                        PopupType::Emojis => {
                            log::debug!("PopupType::Emojis branch, key: {:?}", key.code);
                            let filtered_emojis: Vec<String> = emojis::iter()
                                .filter(|emoji| {
                                    emoji
                                        .shortcodes()
                                        .any(|sc| sc.contains(&state_guard.emoji_query))
                                })
                                .map(|emoji| emoji.to_string())
                                .collect();
                            let num_filtered_emojis = filtered_emojis.len();

                            if num_filtered_emojis == 0 && !state_guard.emoji_query.is_empty() {
                                state_guard.popup_state.show = false;
                                state_guard.popup_state.popup_type = PopupType::None;
                                state_guard.emoji_query.clear();
                            }
                            match key.code {
                                KeyCode::Up => {
                                    if num_filtered_emojis > 0 {
                                        state_guard.selected_emoji_index = (state_guard
                                            .selected_emoji_index
                                            + num_filtered_emojis
                                            - 1)
                                            % num_filtered_emojis;
                                    } else {
                                        state_guard.selected_emoji_index = 0;
                                    }
                                }
                                KeyCode::Down => {
                                    if num_filtered_emojis > 0 {
                                        state_guard.selected_emoji_index =
                                            (state_guard.selected_emoji_index + 1)
                                                % num_filtered_emojis;
                                    } else {
                                        state_guard.selected_emoji_index = 0;
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(emoji_str) =
                                        filtered_emojis.get(state_guard.selected_emoji_index)
                                    {
                                        if let Some(query_start) = input_text.rfind(':') {
                                            input_text.replace_range(query_start.., emoji_str);
                                            state_guard.cursor_position =
                                                query_start + emoji_str.len();
                                        } else {
                                            input_text.push_str(emoji_str);
                                            state_guard.cursor_position = input_text.len();
                                        }
                                    }
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_emoji_index = 0;
                                    state_guard.emoji_query.clear();
                                }
                                KeyCode::Esc => {
                                    log::debug!("PopupType::Emojis dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_emoji_index = 0;
                                    state_guard.emoji_query.clear();
                                }
                                KeyCode::Backspace => {
                                    if state_guard.cursor_position > 0 {
                                        let old_pos = state_guard.cursor_position;
                                        let new_pos = input_text[..old_pos]
                                            .grapheme_indices(true)
                                            .last()
                                            .map(|(i, _)| i)
                                            .unwrap_or(0);
                                        input_text.replace_range(new_pos..old_pos, "");
                                        state_guard.cursor_position = new_pos;
                                    }
                                    if let Some(last_colon_idx) = input_text.rfind(':') {
                                        state_guard.emoji_query =
                                            input_text[last_colon_idx + 1..].to_string();
                                    } else {
                                        state_guard.emoji_query.clear();
                                    }
                                    let new_filtered_emojis: Vec<String> = emojis::iter()
                                        .filter(|emoji| {
                                            emoji
                                                .shortcodes()
                                                .any(|sc| sc.contains(&state_guard.emoji_query))
                                        })
                                        .map(|emoji| emoji.to_string())
                                        .collect();
                                    let new_num_filtered_emojis = new_filtered_emojis.len();
                                    if new_num_filtered_emojis > 0 {
                                        state_guard.selected_emoji_index = state_guard
                                            .selected_emoji_index
                                            .min(new_num_filtered_emojis.saturating_sub(1));
                                    } else {
                                        state_guard.selected_emoji_index = 0;
                                    }
                                    if !should_show_emoji_popup(&input_text) {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                        state_guard.emoji_query.clear();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    input_text.insert(state_guard.cursor_position, c);
                                    state_guard.cursor_position += c.len_utf8();
                                    state_guard.emoji_query.push(c);
                                    if let Some(last_colon_idx) = input_text.rfind(':') {
                                        state_guard.emoji_query =
                                            input_text[last_colon_idx + 1..].to_string();
                                    } else {
                                        state_guard.emoji_query.clear();
                                    }
                                    if let Some(last_colon_idx) = input_text.rfind(':') {
                                        let potential_shortcode_with_colons =
                                            &input_text[last_colon_idx..];
                                        if potential_shortcode_with_colons.ends_with(':')
                                            && potential_shortcode_with_colons.len() > 1
                                        {
                                            let shortcode = &potential_shortcode_with_colons
                                                [1..potential_shortcode_with_colons.len() - 1];
                                            if !shortcode.contains(' ') {
                                                if let Some(emoji) =
                                                    emojis::get_by_shortcode(shortcode)
                                                {
                                                    input_text.replace_range(
                                                        last_colon_idx..,
                                                        emoji.as_str(),
                                                    );
                                                    state_guard.cursor_position =
                                                        last_colon_idx + emoji.as_str().len();
                                                    state_guard.popup_state.show = false;
                                                    state_guard.popup_state.popup_type =
                                                        PopupType::None;
                                                    state_guard.emoji_query.clear();
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        PopupType::DownloadProgress | PopupType::Notification => {
                            log::debug!(
                                "PopupType::{:?} branch, key: {:?}",
                                state_guard.popup_state.popup_type,
                                key.code
                            );
                            match key.code {
                                KeyCode::Esc => {
                                    log::debug!(
                                        "PopupType::{:?} dismissed with Esc",
                                        state_guard.popup_state.popup_type
                                    );
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            }
                        }
                        PopupType::DebugJson => {
                            log::debug!("PopupType::DebugJson branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Esc => {
                                    log::debug!("PopupType::DebugJson dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            }
                        }
                        PopupType::None => {
                            // No specific key handling for None popup type
                        }
                        PopupType::FileManager => {
                            log::debug!("PopupType::FileManager branch, key: {:?}", key.code);
                            match key.code {
                                KeyCode::Esc => {
                                    log::debug!("PopupType::FileManager dismissed with Esc");
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {
                                    let file_manager_event = file_manager.handle_key_event(key);
                                    match file_manager_event {
                                    popups::file_manager::FileManagerEvent::FileSelectedForUpload(path) => {
                                        if let Some(current_channel) = &state_guard.current_channel {
                                            let channel_id = current_channel.id.clone();
                                            if filecommand_tx.send(WsCommand::UploadFile {
                                                channel_id,
                                                file_path: path.clone(),
                                            }).is_err() {
                                                state_guard.set_notification(
                                                    "File Upload Error".to_string(),
                                                    "Failed to send upload command".to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        } else {
                                            state_guard.set_notification(
                                                "File Upload Warning".to_string(),
                                                "No channel selected to upload file.".to_string(),
                                                NotificationType::Warning,
                                                3,
                                            );
                                        }
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    popups::file_manager::FileManagerEvent::FileSelectedForDownload(file_id, file_name) => {
                                        if filecommand_tx.send(WsCommand::DownloadFile { file_id, file_name }).is_err() {
                                            state_guard.set_notification(
                                                "File Download Error".to_string(),
                                                "Failed to send download command"
                                                    .to_string(),
                                                NotificationType::Error,
                                                3,
                                            );
                                        }
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    popups::file_manager::FileManagerEvent::CloseFileManager => {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    popups::file_manager::FileManagerEvent::None => {}
                                }
                                }
                            }
                        }
                    }

                    // This block handles key events when NO popup is shown.
                    // It should be outside the `match current_popup_type` block
                    // or explicitly handled for `PopupType::None`.
                    if current_popup_type == PopupType::None {
                        // Global ESC and logout shortcut handling
                        if key.code == KeyCode::Esc {
                            // Show quit popup globally
                            state_guard.popup_state.show = true;
                            state_guard.popup_state.popup_type = PopupType::Quit;
                            return Ok(Some(TuiPage::Exit));
                        }
                        if key.code == KeyCode::Char('d')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            // Show deconnection/logout popup globally
                            state_guard.popup_state.show = true;
                            state_guard.popup_state.popup_type = PopupType::Deconnection;
                        }
                        log::debug!("No popup active, handling key: {:?}", key.code);
                        match key.code {
                            // Interactive download: Enter on file/image message
                            KeyCode::Enter => {
                                if !input_text.is_empty() {
                                    if input_text.starts_with("/upload ") {
                                        let parts: Vec<&str> = input_text.splitn(2, ' ').collect();
                                        if parts.len() == 2 {
                                            let file_path_str = parts[1].trim();
                                            let file_path = PathBuf::from(file_path_str);
                                            if file_path.exists() && file_path.is_file() {
                                                if let Some(current_channel) =
                                                    &state_guard.current_channel
                                                {
                                                    let channel_id = current_channel.id.clone();
                                                    if filecommand_tx
                                                        .send(WsCommand::UploadFile {
                                                            channel_id,
                                                            file_path: file_path.clone(),
                                                        })
                                                        .is_err()
                                                    {
                                                        state_guard.set_notification(
                                                            "File Upload Error".to_string(),
                                                            "Failed to send upload command"
                                                                .to_string(),
                                                            NotificationType::Error,
                                                            3,
                                                        );
                                                    }
                                                } else {
                                                    state_guard.set_notification(
                                                        "No Channel Selected".to_string(),
                                                        "No channel selected to upload file."
                                                            .to_string(),
                                                        NotificationType::Warning,
                                                        3,
                                                    );
                                                }
                                            } else {
                                                state_guard.set_notification(
                                                    "File Not Found".to_string(),
                                                    format!("File not found: {}", file_path_str),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    } else if input_text.starts_with("/download ") {
                                        let parts: Vec<&str> = input_text.splitn(2, ' ').collect();
                                        if parts.len() == 2 {
                                            let file_id_to_download = parts[1].trim().to_string();
                                            let mut file_to_download: Option<BroadcastMessage> =
                                                None;
                                            if let Some(current_channel) =
                                                &state_guard.current_channel
                                            {
                                                if let Some(messages) =
                                                    state_guard.messages.get(&current_channel.id)
                                                {
                                                    for msg in messages.iter().rev() {
                                                        if let Some(file_id) = &msg.file_id {
                                                            if file_id == &file_id_to_download {
                                                                file_to_download =
                                                                    Some(msg.clone());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(file_info) = file_to_download {
                                                if let Some(file_name) = file_info.file_name {
                                                    if filecommand_tx
                                                        .send(WsCommand::DownloadFile {
                                                            file_id: file_id_to_download,
                                                            file_name,
                                                        })
                                                        .is_err()
                                                    {
                                                        state_guard.set_notification(
                                                            "Download Error".to_string(),
                                                            "Failed to send download command"
                                                                .to_string(),
                                                            NotificationType::Error,
                                                            3,
                                                        );
                                                    }
                                                } else {
                                                    state_guard.set_notification(
                                                        "File Info Error".to_string(),
                                                        "File name not found in message"
                                                            .to_string(),
                                                        NotificationType::Error,
                                                        3,
                                                    );
                                                }
                                            } else {
                                                state_guard.set_notification(
                                                    "File Not Found".to_string(),
                                                    format!(
                                                    "File with ID '{}' not found in this channel.",
                                                    file_id_to_download
                                                ),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    } else if input_text.starts_with("/show ") {
                                        let parts: Vec<&str> = input_text.splitn(2, ' ').collect();
                                        if parts.len() == 2 {
                                            let file_path_str = parts[1].trim();
                                            let _file_path = PathBuf::from(file_path_str);
                                            state_guard.set_notification(
                                                "Image Display Error".to_string(),
                                                "Image preview is not supported anymore."
                                                    .to_string(),
                                                NotificationType::Error,
                                                3,
                                            );
                                        }
                                    } else {
                                        if let Some(current_channel) = &state_guard.current_channel
                                        {
                                            let channel_id = current_channel.id.clone();
                                            let content =
                                                replace_shortcodes_with_emojis(&input_text);
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id,
                                                    content,
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_notification(
                                                    "Message Send Error".to_string(),
                                                    "Failed to send message".to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    }
                                    input_text.clear();
                                    state_guard.cursor_position = 0;
                                } else {
                                    // Only trigger if a file/image message is selected for download
                                    if let Some(current_channel) = &state_guard.current_channel {
                                        if let Some(messages) =
                                            state_guard.messages.get(&current_channel.id)
                                        {
                                            // Calculate which message is selected based on scroll offset and visible area
                                            let message_count = messages.len();
                                            let _rendered_lines = state_guard
                                                .rendered_messages
                                                .get(&current_channel.id)
                                                .map(|v| v.len())
                                                .unwrap_or(0);
                                            // The 'view_height' here is likely incorrect as it uses terminal_width.
                                            // It should ideally be derived from the actual chat message area height.
                                            // For now, retaining the original logic but noting potential issue.
                                            let view_height = state_guard.terminal_width as usize; // fallback, ideally get from UI
                                            let scroll_offset = state_guard.message_scroll_offset;
                                            let _start_index = message_count
                                                .saturating_sub(view_height + scroll_offset);
                                            let end_index =
                                                message_count.saturating_sub(scroll_offset);
                                            // We'll use the last visible message as the 'selected' one for now
                                            if let Some(msg) =
                                                messages.get(end_index.saturating_sub(1))
                                            {
                                                if msg.message_type == "file"
                                                    && msg.file_id.is_some()
                                                    && msg.download_progress.is_none()
                                                {
                                                    if let Some(file_id) = &msg.file_id {
                                                        if let Some(file_name) = &msg.file_name {
                                                            let _ = filecommand_tx.send(
                                                                WsCommand::DownloadFile {
                                                                    file_id: file_id.clone(),
                                                                    file_name: file_name.clone(),
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('@') => {
                                input_text.insert(state_guard.cursor_position, '@');
                                state_guard.cursor_position += 1;
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::Mentions;
                                if command_tx
                                    .send(WsCommand::Message {
                                        channel_id: "home".to_string(),
                                        content: "/get_active_users".to_string(),
                                    })
                                    .is_err()
                                {
                                    state_guard.set_notification(
                                        "Active Users Request Error".to_string(),
                                        "Failed to request active users".to_string(),
                                        NotificationType::Error,
                                        3,
                                    );
                                }
                            }
                            KeyCode::Char(':') => {
                                input_text.insert(state_guard.cursor_position, ':');
                                state_guard.cursor_position += 1;
                                if should_show_emoji_popup(&input_text) {
                                    state_guard.popup_state.show = true;
                                    state_guard.popup_state.popup_type = PopupType::Emojis;
                                    state_guard.emoji_query.clear();
                                } else {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.emoji_query.clear();
                                }
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::Settings;
                            }
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::CreateChannel;
                                create_channel_form = CreateChannelForm::new();
                            }
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::FileManager;
                                file_manager = popups::file_manager::FileManager::new(
                                    popups::file_manager::FileManagerMode::LocalUpload,
                                    Vec::new(),
                                );
                            }
                            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let Some(current_channel) = &state_guard.current_channel {
                                    if let Some(messages) =
                                        state_guard.messages.get(&current_channel.id)
                                    {
                                        if let Some(last_message) = messages.back() {
                                            if let Ok(json_string) =
                                                serde_json::to_string_pretty(last_message)
                                            {
                                                state_guard.debug_json_content = json_string;
                                                state_guard.popup_state.show = true;
                                                state_guard.popup_state.popup_type =
                                                    PopupType::DebugJson;
                                            } else {
                                                state_guard.set_notification(
                                                    "JSON Error".to_string(),
                                                    "Failed to serialize message to JSON"
                                                        .to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Tab => {
                                let i = match channel_list_state.selected() {
                                    Some(i) => {
                                        if i >= state_guard.channels.len() - 1 {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                channel_list_state.select(Some(i));
                                if let Some(selected_channel) = state_guard.channels.get(i).cloned()
                                {
                                    let channel_id = selected_channel.id.clone();
                                    let messages_loaded = state_guard
                                        .messages
                                        .get(&channel_id)
                                        .map_or(false, |v| !v.is_empty());
                                    state_guard.set_current_channel(selected_channel);
                                    if !messages_loaded {
                                        if command_tx
                                            .send(WsCommand::Message {
                                                channel_id: channel_id.clone(),
                                                content: format!("/get_history {} 0", channel_id),
                                            })
                                            .is_err()
                                        {
                                            state_guard.set_notification(
                                                "Command Error".to_string(),
                                                "Failed to send command".to_string(),
                                                NotificationType::Error,
                                                3,
                                            );
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Only scroll on key press, not hold
                                    if let Some(current_channel) = &state_guard.current_channel {
                                        let channel_id = &current_channel.id;
                                        let rendered_count = state_guard
                                            .rendered_messages
                                            .get(channel_id)
                                            .map_or(0, |v| v.len());

                                        if state_guard.message_scroll_offset
                                            >= rendered_count.saturating_sub(5)
                                        {
                                            if let Some((offset, has_more)) =
                                                state_guard.channel_history_state.get(channel_id)
                                            {
                                                if *has_more {
                                                    if command_tx
                                                        .send(WsCommand::Message {
                                                            channel_id: channel_id.clone(),
                                                            content: format!(
                                                                "/get_history {} {}",
                                                                channel_id, offset
                                                            ),
                                                        })
                                                        .is_err()
                                                    {
                                                        state_guard.set_notification(
                                                            "History Request Error".to_string(),
                                                            "Failed to request history".to_string(),
                                                            NotificationType::Error,
                                                            3,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        let view_height = state_guard.last_chat_view_height.max(1);
                                        state_guard.scroll_messages_up(rendered_count, view_height);
                                    }
                                } else {
                                    let i = match channel_list_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                state_guard.channels.len() - 1
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    channel_list_state.select(Some(i));
                                    if let Some(selected_channel) =
                                        state_guard.channels.get(i).cloned()
                                    {
                                        let channel_id = selected_channel.id.clone();
                                        let messages_loaded = state_guard
                                            .messages
                                            .get(&channel_id)
                                            .map_or(false, |v| !v.is_empty());
                                        state_guard.set_current_channel(selected_channel);
                                        if !messages_loaded {
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id: channel_id.clone(),
                                                    content: format!(
                                                        "/get_history {} 0",
                                                        channel_id
                                                    ),
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_notification(
                                                    "Command Error".to_string(),
                                                    "Failed to send command".to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Only scroll on key press, not hold
                                    state_guard.scroll_messages_down();
                                } else {
                                    let i = match channel_list_state.selected() {
                                        Some(i) => {
                                            if i >= state_guard.channels.len() - 1 {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    channel_list_state.select(Some(i));
                                    if let Some(selected_channel) =
                                        state_guard.channels.get(i).cloned()
                                    {
                                        let channel_id = selected_channel.id.clone();
                                        let messages_loaded = state_guard
                                            .messages
                                            .get(&channel_id)
                                            .map_or(false, |v| !v.is_empty());
                                        state_guard.set_current_channel(selected_channel);
                                        if !messages_loaded {
                                            if command_tx
                                                .send(WsCommand::Message {
                                                    channel_id: channel_id.clone(),
                                                    content: format!(
                                                        "/get_history {} 0",
                                                        channel_id
                                                    ),
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_notification(
                                                    "Command Error".to_string(),
                                                    "Failed to send command".to_string(),
                                                    NotificationType::Error,
                                                    3,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if state_guard.cursor_position > 0 {
                                    let old_pos = state_guard.cursor_position;
                                    let new_pos = input_text[..old_pos]
                                        .grapheme_indices(true)
                                        .last()
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    input_text.replace_range(new_pos..old_pos, "");
                                    state_guard.cursor_position = new_pos;
                                }
                            }
                            KeyCode::Char(c) => {
                                input_text.insert(state_guard.cursor_position, c);
                                state_guard.cursor_position += c.len_utf8();
                            }
                            KeyCode::Left => {
                                if state_guard.cursor_position > 0 {
                                    let old_pos = state_guard.cursor_position;
                                    let new_pos = input_text[..old_pos]
                                        .grapheme_indices(true)
                                        .last()
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    state_guard.cursor_position = new_pos;
                                }
                            }
                            KeyCode::Right => {
                                let old_pos = state_guard.cursor_position;
                                if old_pos < input_text.len() {
                                    let new_pos = input_text[old_pos..]
                                        .grapheme_indices(true)
                                        .nth(1)
                                        .map(|(i, _)| old_pos + i)
                                        .unwrap_or_else(|| input_text.len());
                                    state_guard.cursor_position = new_pos;
                                }
                            }
                            KeyCode::Home => {
                                state_guard.cursor_position = 0;
                            }
                            KeyCode::End => {
                                state_guard.cursor_position = input_text.len();
                            }
                            _ => {}
                        }
                    }
                }
                KeyEventKind::Release => {
                    // Reset any stuck scroll/input state here if needed
                    // For example, if you want to reset scroll offset or input lock, do it here
                    // Currently, no persistent key state, so nothing to reset
                }
                KeyEventKind::Repeat => {
                    // Optionally handle repeated key events if needed
                }
            }
        }
    }
}
