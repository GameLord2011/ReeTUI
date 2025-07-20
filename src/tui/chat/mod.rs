pub mod create_channel_form;
pub mod message_parsing;
pub mod popups;
pub mod theme_settings_form;
pub mod ui;
pub mod utils;
pub mod ws_command;

#[cfg(test)]
pub mod tests;

use crate::api::models::Channel;
use crate::api::websocket::{self, ServerMessage};
use crate::app::{AppState, PopupType};
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use emojis;
use futures_util::{SinkExt, StreamExt};
use notify_rust::Notification;
use ratatui::{prelude::Backend, widgets::ListState, Terminal};
use regex::Regex;
use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::tungstenite;
use unicode_segmentation::UnicodeSegmentation;

use crate::tui::chat::create_channel_form::{CreateChannelForm, CreateChannelInput};
use crate::tui::chat::message_parsing::{
    replace_shortcodes_with_emojis, should_show_emoji_popup, should_show_mention_popup,
};

use crate::api::file_api;
use crate::tui::chat::theme_settings_form::ThemeSettingsForm;
use crate::tui::chat::ui::draw_chat_ui;
use crate::tui::chat::ws_command::WsCommand;

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let mut create_channel_form = CreateChannelForm::new();
    let mut theme_settings_form = ThemeSettingsForm::new(app_state.lock().unwrap().current_theme);
    let mut file_manager = popups::file_manager::FileManager::new(popups::file_manager::FileManagerMode::LocalUpload, Vec::new());

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
    let (file_command_tx, mut file_command_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<u8>();

    let http_client = reqwest::Client::new();

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
                _ => {}
            }
        }
    });

    let app_state_clone_for_file_commands = app_state.clone();
    let http_client_clone = http_client.clone();
    let progress_tx_clone = progress_tx.clone();
    tokio::spawn(async move {
        while let Some(command) = file_command_rx.recv().await {
            match command {
                WsCommand::UploadFile {
                    channel_id,
                    file_path,
                } => {
                    let token = {
                        let state_guard = app_state_clone_for_file_commands.lock().unwrap();
                        state_guard
                            .auth_token
                            .clone()
                            .expect("Auth token not found for file upload")
                    };

                    match file_api::upload_file(
                        &http_client_clone,
                        &token,
                        &channel_id,
                        file_path.clone(),
                        progress_tx_clone.clone(),
                    )
                    .await
                    {
                        Ok(file_id) => {
                            let mut state_guard = app_state_clone_for_file_commands.lock().unwrap();
                            state_guard
                                .set_error_message(format!("File uploaded: {}", file_id), 3000);
                        }
                        Err(e) => {
                            let mut state_guard = app_state_clone_for_file_commands.lock().unwrap();
                            state_guard
                                .set_error_message(format!("File upload failed: {}", e.to_string()), 5000);
                        }
                    }
                }
                WsCommand::DownloadFile { file_id, file_name } => {
                    let _ = {
                        let state_guard = app_state_clone_for_file_commands.lock().unwrap();
                        state_guard
                            .current_channel
                            .as_ref()
                            .map(|c| c.id.clone())
                            .unwrap_or_default()
                    };

                    let download_path = PathBuf::from("./downloads").join(&file_name);
                    if !download_path.parent().unwrap().exists() {
                        tokio::fs::create_dir_all(download_path.parent().unwrap())
                            .await
                            .unwrap();
                    }

                    match file_api::download_file(
                        &http_client_clone,
                        &file_id,
                        &file_name,
                        progress_tx_clone.clone(),
                    )
                    .await
                    {
                        Ok(bytes) => {
                            let mut file = tokio::fs::File::create(&download_path).await.unwrap();
                            file.write_all(&bytes).await.unwrap();
                            let mut state_guard = app_state_clone_for_file_commands.lock().unwrap();
                            state_guard.set_error_message(
                                format!("Downloaded {} to {:?}", file_name, download_path),
                                3000,
                            );
                        }
                        Err(e) => {
                            let mut state_guard = app_state_clone_for_file_commands.lock().unwrap();
                            state_guard.set_error_message(format!("Download failed: {}", e.to_string()), 5000);
                        }
                    }
                }
                _ => {}
            }
        }
    });

    let app_state_clone_for_progress = app_state.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let mut state_guard = app_state_clone_for_progress.lock().unwrap();
            state_guard.set_error_message(format!("File transfer progress: {}%", progress), 1000);
        }
    });

    if command_tx
        .send(WsCommand::Message {
            channel_id: "home".to_string(),
            content: "/get_history home 0".to_string(),
        })
        .is_err()
    {
        app_state
            .lock()
            .unwrap()
            .set_error_message("Failed to send command".to_string(), 3000);
    }

    let mut last_rendered_width: u16 = 0;

    let mention_regex = Regex::new(r"@(\w+)").unwrap();
    let emoji_regex = Regex::new(r":([a-zA-Z0-9_+-]+):").unwrap();

    loop {
        let mut state_guard = app_state.lock().unwrap();
        state_guard.clear_expired_error();

        let filtered_users: Vec<String> = state_guard
            .active_users
            .iter()
            .filter(|user| {
                user.to_lowercase()
                    .contains(&state_guard.mention_query.to_lowercase())
                    && user != &&state_guard.username.clone().unwrap_or_default()
            })
            .cloned()
            .collect();
        let filtered_emojis: Vec<String> = emojis::iter()
            .filter(|emoji| {
                emoji
                    .name()
                    .to_lowercase()
                    .contains(&state_guard.emoji_query.to_lowercase())
            })
            .map(|emoji| emoji.as_str().to_string())
            .collect();

        terminal.draw(|f| {
            let current_width = f.area().width;
            if last_rendered_width != current_width {
                state_guard.rendered_messages.clear();
                last_rendered_width = current_width;
            }

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

        let current_popup_type = state_guard.popup_state.popup_type;
        if state_guard.popup_state.show
            && current_popup_type != PopupType::Emojis
            && current_popup_type != PopupType::Mentions
        {
            terminal.hide_cursor()?;
        } else {
            terminal.show_cursor()?;
        }

        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Resize(_, _) = event {
                state_guard.rendered_messages.clear();
            } else if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    let num_filtered_users = filtered_users.len();
                    let num_filtered_emojis = filtered_emojis.len();

                    if state_guard.popup_state.show {
                        match state_guard.popup_state.popup_type {
                            PopupType::Quit => match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    return Ok(TuiPage::Exit);
                                }
                                KeyCode::Esc => {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Settings => match key.code {
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
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.selected_setting_index = 0;
                                }
                                _ => {}
                            },
                            PopupType::CreateChannel => match key.code {
                                KeyCode::Esc => {
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
                                                state_guard.set_error_message(
                                                    format!("Failed to create channel"),
                                                    3000,
                                                );
                                            } else {
                                                state_guard.set_error_message(
                                                    format!("Channel '{}' created!", channel_name),
                                                    3000,
                                                );
                                                state_guard.popup_state.show = false;
                                                state_guard.popup_state.popup_type =
                                                    PopupType::None;
                                                create_channel_form = CreateChannelForm::new();
                                            }
                                        } else {
                                            state_guard.set_error_message(
                                                "Channel name cannot be empty!".to_string(),
                                                3000,
                                            );
                                        }
                                    }
                                },
                                _ => {}
                            },
                            PopupType::SetTheme => match key.code {
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
                                    state_guard.current_theme = theme_settings_form.original_theme;
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Deconnection => match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    state_guard.clear_user_auth();
                                    return Ok(TuiPage::Auth);
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    state_guard.popup_state.popup_type = PopupType::Settings;
                                }
                                _ => {}
                            },
                            PopupType::Help => match key.code {
                                KeyCode::Esc => {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Mentions => match key.code {
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

                                    if num_filtered_users > 0 {
                                        state_guard.selected_mention_index = state_guard
                                            .selected_mention_index
                                            .min(num_filtered_users.saturating_sub(1));
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
                            },
                            PopupType::Emojis => {
                                if num_filtered_emojis == 0 && !state_guard.emoji_query.is_empty() {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    state_guard.emoji_query.clear();
                                    // continue; // This `continue` is problematic in this context.
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

                                        if num_filtered_emojis > 0 {
                                            state_guard.selected_emoji_index = state_guard
                                                .selected_emoji_index
                                                .min(num_filtered_emojis.saturating_sub(1));
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
                            PopupType::None => {
                                // No specific key handling for None popup type
                            }
                            PopupType::FileManager => match key.code {
                                KeyCode::Esc => {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {
                                    let file_manager_event = file_manager.handle_key_event(key);
                                    match file_manager_event {
                                        popups::file_manager::FileManagerEvent::FileSelectedForUpload(path) => {
                                            if let Some(current_channel) = &state_guard.current_channel {
                                                let channel_id = current_channel.id.clone();
                                                if file_command_tx
                                                    .send(WsCommand::UploadFile {
                                                        channel_id,
                                                        file_path: path,
                                                    })
                                                    .is_err()
                                                {
                                                    state_guard.set_error_message(
                                                        "Failed to send upload command".to_string(),
                                                        3000,
                                                    );
                                                }
                                            } else {
                                                state_guard.set_error_message(
                                                    "No channel selected to upload file.".to_string(),
                                                    3000,
                                                );
                                            }
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                        },
                                        popups::file_manager::FileManagerEvent::FileSelectedForDownload(file_id, file_name) => {
                                            if file_command_tx
                                                .send(WsCommand::DownloadFile {
                                                    file_id,
                                                    file_name,
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_error_message(
                                                    "Failed to send download command".to_string(),
                                                    3000,
                                                );
                                            }
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                        },
                                        popups::file_manager::FileManagerEvent::CloseFileManager => {
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                        },
                                        popups::file_manager::FileManagerEvent::None => {},
                                    }
                                }
                            },
                        }
                    } else {
                        // This else block handles when no popup is shown
                        match key.code {
                            KeyCode::Char('@') => {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::Mentions;
                                if command_tx
                                    .send(WsCommand::Message {
                                        channel_id: "home".to_string(),
                                        content: "/get_active_users".to_string(),
                                    })
                                    .is_err()
                                {
                                    state_guard.set_error_message(
                                        "Failed to request active users".to_string(),
                                        3000,
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
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::FileManager;
                                // Mock data for downloadable files for now
                                let downloadable_files = vec![
                                    popups::file_manager::DownloadableFile {
                                        id: "file1".to_string(),
                                        name: "document.pdf".to_string(),
                                    },
                                    popups::file_manager::DownloadableFile {
                                        id: "file2".to_string(),
                                        name: "image.png".to_string(),
                                    },
                                ];
                                file_manager = popups::file_manager::FileManager::new(
                                    popups::file_manager::FileManagerMode::RemoteDownload,
                                    downloadable_files,
                                );
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
                                            state_guard.set_error_message(
                                                "Failed to send command".to_string(),
                                                3000,
                                            );
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    let channel_id =
                                        state_guard.current_channel.as_ref().unwrap().id.clone();
                                    let rendered_count = state_guard
                                        .rendered_messages
                                        .get(&channel_id)
                                        .map_or(0, |v| v.len());

                                    if state_guard.message_scroll_offset
                                        >= rendered_count.saturating_sub(5)
                                    {
                                        if let Some((offset, has_more)) =
                                            state_guard.channel_history_state.get(&channel_id)
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
                                                    state_guard.set_error_message(
                                                        "Failed to request history".to_string(),
                                                        3000,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    state_guard.scroll_messages_up();
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
                                                state_guard.set_error_message(
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
                                                state_guard.set_error_message(
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
                                                    if file_command_tx
                                                        .send(WsCommand::UploadFile {
                                                            channel_id,
                                                            file_path,
                                                        })
                                                        .is_err()
                                                    {
                                                        state_guard.set_error_message(
                                                            "Failed to send upload command"
                                                                .to_string(),
                                                            3000,
                                                        );
                                                    }
                                                } else {
                                                    state_guard.set_error_message(
                                                        "No channel selected to upload file."
                                                            .to_string(),
                                                        3000,
                                                    );
                                                }
                                            } else {
                                                state_guard.set_error_message(
                                                    format!("File not found: {}", file_path_str),
                                                    3000,
                                                );
                                            }
                                        } else {
                                            state_guard.set_error_message(
                                                "Usage: /upload <file_path>".to_string(),
                                                3000,
                                            );
                                        }
                                    } else if input_text.starts_with("/download ") {
                                        let parts: Vec<&str> = input_text.splitn(3, ' ').collect();
                                        if parts.len() == 3 {
                                            let file_id = parts[1].trim().to_string();
                                            let file_name = parts[2].trim().to_string();
                                            if file_command_tx
                                                .send(WsCommand::DownloadFile {
                                                    file_id,
                                                    file_name,
                                                })
                                                .is_err()
                                            {
                                                state_guard.set_error_message(
                                                    "Failed to send download command".to_string(),
                                                    3000,
                                                );
                                            }
                                        } else {
                                            state_guard.set_error_message(
                                                "Usage: /download <file_id> <file_name>"
                                                    .to_string(),
                                                3000,
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
                                                state_guard.set_error_message(
                                                    "Failed to send message".to_string(),
                                                    3000,
                                                );
                                            }
                                        }
                                    }
                                    input_text.clear();
                                    state_guard.cursor_position = 0;
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
            }
        }

        if let Ok(Some(msg)) =
            tokio::time::timeout(Duration::from_millis(10), ws_reader.next()).await
        {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    let server_message = websocket::parse_server_message(&text);
                    match server_message {
                        ServerMessage::ChatMessage(chat_msg) => {
                            let current_username = state_guard.username.clone();
                            let channel_name = state_guard
                                .channels
                                .iter()
                                .find(|c| c.id == chat_msg.channel_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| "Unknown Channel".to_string());
                            state_guard.add_message(chat_msg.clone());
                            if let Some(username) = current_username {
                                if chat_msg.content.contains(&format!("@{}", username))
                                    || chat_msg.content.contains("@everyone")
                                {
                                    let _ = Notification::new()
                                        .summary(&format!("New mention in {}", channel_name))
                                        .body(&format!("{}: {}", chat_msg.user, chat_msg.content))
                                        .show();
                                }
                            }
                        }
                        ServerMessage::History(history) => {
                            if !history.is_empty() {
                                state_guard
                                    .prepend_history(&history[0].channel_id, history.clone());
                            }
                        }
                        ServerMessage::ChannelUpdate(channel_broadcast) => {
                            let channel = Channel {
                                id: channel_broadcast.id,
                                name: channel_broadcast.name,
                                icon: channel_broadcast.icon,
                            };
                            state_guard.add_or_update_channel(channel);
                        }
                        ServerMessage::ChannelDelete(channel_id) => {
                            state_guard.remove_channel(&channel_id);
                        }
                        ServerMessage::ActiveUsers(active_users) => {
                            state_guard.active_users = active_users;
                        }
                        ServerMessage::Unknown(unknown_msg) => {
                            state_guard.set_error_message(
                                format!("Unknown WS message: {}", unknown_msg),
                                3000,
                            );
                            eprintln!("Received unknown WebSocket message: {}", unknown_msg);
                        }
                    }
                }
                Ok(tungstenite::Message::Ping(_)) => {
                    if command_tx.send(WsCommand::Pong).is_err() {
                        state_guard
                            .set_error_message("Failed to send pong command".to_string(), 3000);
                        eprintln!("Failed to send pong command");
                    }
                }
                Err(e) => {
                    state_guard.set_error_message(format!("WebSocket error: {:?}", e), 5000);
                    eprintln!("WebSocket error: {:?}", e);
                }
                _ => {}
            }
        }
    }
}
