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
use crate::api::websocket;
use crate::app::{AppState, PopupType};

use crate::tui::chat::create_channel_form::{CreateChannelForm, CreateChannelInput};
use crate::tui::chat::message_parsing::{
    replace_shortcodes_with_emojis, should_show_emoji_popup, should_show_mention_popup, get_emoji_query,
};
use crate::tui::chat::ui::draw_chat_ui;
use crate::tui::chat::ws_command::WsCommand;
use crate::tui::notification::notification::NotificationType;
use crate::tui::settings::{self, state::SettingsState};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use lazy_static::lazy_static;
use ratatui::{prelude::Backend, widgets::ListState, Terminal};
use regex::Regex;
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use unicode_segmentation::UnicodeSegmentation;

use crate::tui::file_manager_module::file_manager::{FileManager, FileManagerEvent};

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(r"@[a-zA-Z0-9_]+").unwrap();
    static ref EMOJI_REGEX: Regex = Regex::new(r":[a-zA-Z0-9_]+:").unwrap();
}

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<Option<crate::app::TuiPage>> {
    log::debug!("chat_mod: run_chat_page started.");
    log::debug!("Initial app state: {:?}", app_state.lock().await);

    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let (redraw_tx, mut redraw_rx) = mpsc::unbounded_channel::<String>();
    let mut create_channel_form = CreateChannelForm::new();
    let mut file_manager = FileManager::new(redraw_tx.clone(), app_state.clone());

    let mut settings_state = {
        let state = app_state.lock().await;
        SettingsState::new(
            state.themes.keys().cloned().collect(),
            state.current_theme.name.clone(),
            state.username.as_deref().unwrap_or(""),
            state.user_icon.as_deref().unwrap_or(""),
            state.settings_main_selection,
            state.settings_focused_pane,
            state.quit_confirmation_state,
            state.quit_selection,
        )
    };

    let cancellation_token = CancellationToken::new();
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

    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (filecommand_tx, mut file_command_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<(String, u8)>();
    let http_client = reqwest::Client::new();

    let command_tx_clone = command_tx.clone();
    let ws_task = tokio::spawn(websocket::handle_websocket_communication(
        ws_reader,
        app_state.clone(),
        command_tx_clone,
        http_client.clone(),
        redraw_tx.clone(),
        cancellation_token.clone(),
    ));

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
                _ => {}
            }
        }
    });

    let app_state_for_file_commands = app_state.clone();
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
                    let app_state_for_upload_clone = app_state_for_file_commands.clone();
                    let token = {
                        let state = app_state_for_upload_clone.lock().await;
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
                                let mut state = app_state_for_upload_clone.lock().await;
                                state.notification_manager.add(
                                    "File Upload Success".to_string(),
                                    "File uploaded successfully!".to_string(),
                                    NotificationType::Success,
                                    Some(Duration::from_secs(3)),
                                    app_state_for_upload_clone.clone(),
                                ).await;
                            }
                            Err(e) => {
                                let mut state = app_state_for_upload_clone.lock().await;
                                state.notification_manager.add(
                                    "File Upload Error".to_string(),
                                    format!("Failed to upload file: {}", e),
                                    NotificationType::Error,
                                    Some(Duration::from_secs(3)),
                                    app_state_for_upload_clone.clone(),
                                ).await;
                            }
                        }
                    }
                }
                WsCommand::DownloadFile { file_id, file_name } => {
                    let app_state_for_download = app_state_for_file_commands.clone();
                    let progress_tx3 = progress_tx2.clone();
                    let http_client_clone = http_client_for_file_commands.clone();
                    tokio::spawn(async move {
                        match file_api::download_file(
                            &http_client_clone,
                            &file_id,
                            &file_name,
                            progress_tx3.clone(),
                            true,
                        )
                        .await
                        {
                            Ok(_) => {
                                let mut state = app_state_for_download.lock().await;
                                state.notification_manager.add(
                                    "File Download Success".to_string(),
                                    format!("File '{}' downloaded successfully!", file_name),
                                    NotificationType::Success,
                                    Some(Duration::from_secs(3)),
                                    app_state_for_download.clone(),
                                ).await;
                            }
                            Err(e) => {
                                let mut state = app_state_for_download.lock().await;
                                state.notification_manager.add(
                                    "File Download Error".to_string(),
                                    format!("Failed to download file: {}", e),
                                    NotificationType::Error,
                                    Some(Duration::from_secs(3)),
                                    app_state_for_download.clone(),
                                ).await;
                            }
                        }
                    });
                }

                _ => {}
            }
        }
    });

    let app_state_clone_for_progress = app_state.clone();
    tokio::spawn(async move {
        while let Some((_file_id, progress)) = progress_rx.recv().await {
            let mut state = app_state_clone_for_progress.lock().await;
            state.notification_manager.add(
                "Download Progress".to_string(),
                format!("Downloading: {}%", progress),
                NotificationType::Info,
                None,
                app_state_clone_for_progress.clone(),
            ).await;
            if progress == 100 {
                state.popup_state.show = false;
                state.popup_state.popup_type = PopupType::None;
            }
        }
    });

    loop {
        let mut state_guard = app_state.lock().await;
        state_guard.notification_manager.update();

        let _filtered_users: Vec<String> = state_guard.active_users.clone();
        let _filtered_emojis: Vec<String> = emojis::iter().map(|e| e.to_string()).collect();
        let _mention_regex = &MENTION_REGEX;
        let _emoji_regex = &EMOJI_REGEX;

        terminal.draw(|f| {
            let mention_regex = &MENTION_REGEX;
            let emoji_regex = &EMOJI_REGEX;

            draw_chat_ui::<B>(
                f,
                &mut state_guard,
                &input_text,
                &mut channel_list_state,
                &mut create_channel_form,
                &mut file_manager,
                mention_regex,
                emoji_regex,
                &mut settings_state,
            );
        })?;

        let event = tokio::select! {
            Some(_) = redraw_rx.recv() => None,
            event_result = tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(16))) => {
                match event_result {
                    Ok(Ok(true)) => Some(tokio::task::spawn_blocking(event::read).await.unwrap().unwrap()),
                    _ => None,
                }
            }
        };

        if let Some(event) = event {
            if state_guard.show_settings {
                if let Some(target_page) = settings::handle_settings_key_event(
                    settings::SettingsEvent::Key(event.clone()),
                    &mut state_guard,
                    &mut settings_state,
                ) {
                    if target_page == crate::app::TuiPage::Chat {
                        state_guard.show_settings = false;
                    } else {
                        cancellation_token.cancel();
                        ws_task.await.unwrap().unwrap();
                        return Ok(Some(target_page));
                    }
                }
            } else {
                if let Event::Key(key) = event {
                    if key.code == KeyCode::Char('d')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        state_guard.popup_state.show = true;
                        state_guard.popup_state.popup_type = PopupType::Downloads;
                        continue;
                    }
                    if key.code == KeyCode::Char('s')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        state_guard.show_settings = true;
                        continue;
                    }
                }

                if let Event::Mouse(mouse_event) = event {
                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => {
                            state_guard.scroll_messages_up(1);
                        }
                        MouseEventKind::ScrollDown => {
                            state_guard.scroll_messages_down(1);
                        }
                        _ => {}
                    }
                    continue;
                }

                if let Event::Key(key) = event {
                    log::debug!("Raw Key Event: {:?}", key);
                    if key.kind == KeyEventKind::Press {
                        log::debug!("Key pressed: {:?}, Modifiers: {:?}", key.code, key.modifiers);
                        log::debug!("Current chat_focused_pane: {:?}", state_guard.chat_focused_pane);
                        let current_popup_type = state_guard.popup_state.popup_type;

                        match current_popup_type {
                            PopupType::CreateChannel => match key.code {
                                KeyCode::Esc => {
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    create_channel_form = CreateChannelForm::new();
                                }
                                KeyCode::Tab | KeyCode::Down => {
                                    create_channel_form.next_input();
                                }
                                KeyCode::Up => {
                                    create_channel_form.previous_input();
                                }
                                KeyCode::Backspace => {
                                    if let CreateChannelInput::Name =
                                        create_channel_form.input_focused
                                    {
                                        create_channel_form.name.pop();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if let CreateChannelInput::Name =
                                        create_channel_form.input_focused
                                    {
                                        create_channel_form.name.push(c);
                                    }
                                }
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
                                                state_guard.notification_manager.add(
                                                    "Channel Creation Error".to_string(),
                                                    "Failed to create channel".to_string(),
                                                    NotificationType::Error,
                                                    Some(Duration::from_secs(3)),
                                                    app_state.clone(),
                                                ).await;
                                            } else {
                                                state_guard.notification_manager.add(
                                                    "Channel Creation Success".to_string(),
                                                    format!("Channel '{}' created!", channel_name),
                                                    NotificationType::Success,
                                                    Some(Duration::from_secs(3)),
                                                    app_state.clone(),
                                                ).await;
                                                state_guard.popup_state.show = false;
                                                state_guard.popup_state.popup_type =
                                                    PopupType::None;
                                                create_channel_form = CreateChannelForm::new();
                                            }
                                        } else {
                                            state_guard.notification_manager.add(
                                                "Channel Creation Warning".to_string(),
                                                "Channel name cannot be empty!".to_string(),
                                                NotificationType::Warning,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            ).await;
                                        }
                                    }
                                },
                                _ => {}
                            },
                            PopupType::Deconnection => match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    state_guard.clear_user_auth().await;
                                    return Ok(Some(crate::app::TuiPage::Auth));
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    state_guard.popup_state.popup_type = PopupType::None;
                                }
                                _ => {}
                            },
                            PopupType::Mentions => {
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
                                        }
                                    }
                                    KeyCode::Down => {
                                        if num_filtered_users > 0 {
                                            state_guard.selected_mention_index =
                                                (state_guard.selected_mention_index + 1)
                                                    % num_filtered_users;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        if let Some(user) =
                                            filtered_users.get(state_guard.selected_mention_index)
                                        {
                                            let query_start =
                                                input_text.rfind('@').unwrap_or(input_text.len());
                                            input_text.replace_range(
                                                query_start..,
                                                &format!("@{} ", user),
                                            );
                                            state_guard.cursor_position = input_text.len();
                                        }
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    KeyCode::Esc => {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
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
                                        if !should_show_mention_popup(&input_text) {
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        input_text.insert(state_guard.cursor_position, c);
                                        state_guard.cursor_position += c.len_utf8();
                                        if !should_show_mention_popup(&input_text) {
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            PopupType::Emojis => {
                                let filtered_emojis: Vec<String> = emojis::iter()
                                    .filter(|emoji| {
                                        emoji
                                            .shortcodes()
                                            .any(|sc| sc.contains(&state_guard.emoji_query))
                                    })
                                    .map(|emoji| emoji.to_string())
                                    .collect();
                                let num_filtered_emojis = filtered_emojis.len();

                                match key.code {
                                    KeyCode::Up => {
                                        if num_filtered_emojis > 0 {
                                            state_guard.selected_emoji_index = (state_guard
                                                .selected_emoji_index
                                                + num_filtered_emojis
                                                - 1)
                                                % num_filtered_emojis;
                                        }
                                    }
                                    KeyCode::Down => {
                                        if num_filtered_emojis > 0 {
                                            state_guard.selected_emoji_index =
                                                (state_guard.selected_emoji_index + 1)
                                                    % num_filtered_emojis;
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
                                    }
                                    KeyCode::Esc => {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
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
                                         if !should_show_emoji_popup(&input_text) {
                                             state_guard.popup_state.show = false;
                                             state_guard.popup_state.popup_type = PopupType::None;
                                         } else {
                                             state_guard.emoji_query = get_emoji_query(&input_text);
                                         }                                    }
                                     KeyCode::Char(c) => {
                                         input_text.insert(state_guard.cursor_position, c);
                                         state_guard.cursor_position += c.len_utf8();
                                         state_guard.emoji_query = get_emoji_query(&input_text);
                                         if !should_show_emoji_popup(&input_text) {
                                             state_guard.popup_state.show = false;
                                             state_guard.popup_state.popup_type = PopupType::None;
                                         } else {
                                             // Check for completed shortcode only if popup is still active
                                             if let Some(last_colon_idx) = input_text.rfind(':') {
                                                 let potential_shortcode = &input_text[last_colon_idx..];
                                                 if potential_shortcode.ends_with(':')
                                                     && potential_shortcode.len() > 1
                                                 {
                                                     let shortcode = &potential_shortcode
                                                         [1..potential_shortcode.len() - 1];
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
                                                     }
                                                 }
                                             }
                                         }
                                     }                                    _ => {}
                                }
                            }
                            PopupType::FileManager => {
                                let file_manager_event = file_manager.handle_key_event(key);
                                match file_manager_event {
                                    FileManagerEvent::FileSelectedForUpload(path) => {
                                        if let Some(current_channel) = &state_guard.current_channel
                                        {
                                            let channel_id = current_channel.id.clone();
                                            if filecommand_tx
                                                .send(WsCommand::UploadFile {
                                                    channel_id,
                                                    file_path: path.clone(),
                                                })
                                                .is_err()
                                            {
                                                state_guard.notification_manager.add(
                                                    "File Upload Error".to_string(),
                                                    "Failed to send upload command".to_string(),
                                                    NotificationType::Error,
                                                    Some(Duration::from_secs(3)),
                                                    app_state.clone(),
                                                ).await;
                                            }
                                        }
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    FileManagerEvent::CloseFileManager => {
                                        state_guard.popup_state.show = false;
                                        state_guard.popup_state.popup_type = PopupType::None;
                                    }
                                    FileManagerEvent::None => {}
                                }
                            }
                            
                            _ => {}
                        }

                        if current_popup_type == PopupType::None {
                            if key.code == KeyCode::Char('d')
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                state_guard.popup_state.show = true;
                                state_guard.popup_state.popup_type = PopupType::Deconnection;
                            } else {
                                match key.code {
                                    KeyCode::Enter => {
                                        if !input_text.is_empty() {
                                            if input_text.starts_with("/download ") {
                                                let parts: Vec<&str> =
                                                    input_text.splitn(2, ' ').collect();
                                                if parts.len() == 2 {
                                                    let file_id = parts[1].to_string();
                                                    let file_name_to_download = {
                                                        let mut found_file_name = None;
                                                        if let Some(current_channel) =
                                                            &state_guard.current_channel
                                                        {
                                                            if let Some(messages_deque) =
                                                                state_guard
                                                                    .messages
                                                                    .get(&current_channel.id)
                                                            {
                                                                for msg in messages_deque.iter() {
                                                                    if msg.file_id.as_deref()
                                                                        == Some(&file_id)
                                                                    {
                                                                        found_file_name =
                                                                            msg.file_name.clone();
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        found_file_name.unwrap_or_else(|| {
                                                            "downloaded_file".to_string()
                                                        })
                                                    };

                                                    if filecommand_tx
                                                        .send(WsCommand::DownloadFile {
                                                            file_id,
                                                            file_name: file_name_to_download,
                                                        })
                                                        .is_err()
                                                    {
                                                        state_guard.notification_manager.add(
                                                            "Download Error".to_string(),
                                                            "Failed to send download command"
                                                                .to_string(),
                                                            NotificationType::Error,
                                                            Some(Duration::from_secs(3)),
                                                            app_state.clone(),
                                                        ).await;
                                                    }
                                                } else {
                                                    state_guard.notification_manager.add(
                                                         "Download Error".to_string(),
                                                         "Invalid /download command format. Usage: /download <file_id>".to_string(),
                                                         NotificationType::Error,
                                                         Some(Duration::from_secs(3)),
                                                         app_state.clone(),
                                                     ).await;
                                                }
                                            } else {
                                                if let Some(current_channel) =
                                                    &state_guard.current_channel
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
                                                        state_guard.notification_manager.add(
                                                            "Message Send Error".to_string(),
                                                            "Failed to send message".to_string(),
                                                            NotificationType::Error,
                                                            Some(Duration::from_secs(3)),
                                                            app_state.clone(),
                                                        ).await;
                                                    }
                                                }
                                            }
                                            input_text.clear();
                                            state_guard.cursor_position = 0;
                                        }
                                    }
                                    KeyCode::Char('@') => {
                                        input_text.insert(state_guard.cursor_position, '@');
                                        state_guard.cursor_position += 1;
                                        state_guard.popup_state.show = true;
                                        state_guard.popup_state.popup_type = PopupType::Mentions;
                                        state_guard.notification_manager.add(
                                            "Mention".to_string(),
                                            "Mentioning a user".to_string(),
                                            NotificationType::Info,
                                            Some(Duration::from_secs(3)),
                                            app_state.clone(),
                                        ).await;
                                        if command_tx
                                            .send(WsCommand::Message {
                                                channel_id: "home".to_string(),
                                                content: "/get_active_users".to_string(),
                                            })
                                            .is_err()
                                        {
                                            state_guard.notification_manager.add(
                                                "Active Users Request Error".to_string(),
                                                "Failed to request active users".to_string(),
                                                NotificationType::Error,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            ).await;
                                        }
                                    }
                                    KeyCode::Char(':') => {
                                        input_text.insert(state_guard.cursor_position, ':');
                                        state_guard.cursor_position += 1;
                                          if should_show_emoji_popup(&input_text) {
                                              state_guard.popup_state.show = true;
                                              state_guard.popup_state.popup_type = PopupType::Emojis;
                                              state_guard.emoji_query = String::new(); // Initialize emoji_query when popup is shown
                                            state_guard.notification_manager.add(
                                                "Emoji".to_string(),
                                                "Showing emoji suggestions".to_string(),
                                                NotificationType::Info,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            ).await;
                                        }
                                    }
                                    KeyCode::Char('n')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        state_guard.popup_state.show = true;
                                        state_guard.popup_state.popup_type =
                                            PopupType::CreateChannel;
                                        create_channel_form = CreateChannelForm::new();
                                    }
                                    KeyCode::Char('u')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        state_guard.popup_state.show = true;
                                        state_guard.popup_state.popup_type = PopupType::FileManager;
                                        file_manager =
                                            FileManager::new(redraw_tx.clone(), app_state.clone());
                                    }
                                    
                                    KeyCode::Tab => {
                                        match state_guard.chat_focused_pane {
                                            crate::app::app_state::ChatFocusedPane::ChannelList => {
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
                                                    state_guard.set_current_channel(selected_channel);
                                                    if state_guard.messages.get(&channel_id).map_or(true, |m| m.is_empty()) && state_guard.channel_history_state.get(&channel_id).map_or(true, |&(_, _, initial_fetched)| !initial_fetched) {
                                                        if command_tx
                                                            .send(WsCommand::Message {
                                                                channel_id: channel_id.clone(),
                                                                content: format!("/get_history {}", channel_id),
                                                            })
                                                            .is_err()
                                                        {
                                                            log::error!("Failed to send /get_history command");
                                                        } else {
                                                            if let Some(state) = state_guard.channel_history_state.get_mut(&channel_id) {
                                                                state.2 = true;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {
                                                state_guard.chat_focused_pane = match state_guard.chat_focused_pane {
                                                    crate::app::app_state::ChatFocusedPane::Input => crate::app::app_state::ChatFocusedPane::ChannelList,
                                                    crate::app::app_state::ChatFocusedPane::ChannelList => crate::app::app_state::ChatFocusedPane::Messages,
                                                    crate::app::app_state::ChatFocusedPane::Messages => crate::app::app_state::ChatFocusedPane::Input,
                                                };
                                            }
                                        }
                                    }
                                    KeyCode::Up => {
                                        state_guard.scroll_messages_up(1);
                                    }
                                    KeyCode::Down => {
                                        log::debug!("Attempting to scroll DOWN by 1.");
                                        state_guard.scroll_messages_down(1);
                                    }
                                    KeyCode::PageUp => {
                                        log::debug!("PageUp key pressed. Chat focused pane: {:?}", state_guard.chat_focused_pane);
                                        state_guard.scroll_messages_page_up();
                                    }
                                    KeyCode::PageDown => {
                                        log::debug!("PageDown key pressed. Chat focused pane: {:?}", state_guard.chat_focused_pane);
                                        state_guard.scroll_messages_page_down();
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
                                        if state_guard.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Input {
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
                                    }
                                    KeyCode::Right => {
                                        if state_guard.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Input {
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
                                    }
                                    KeyCode::Home => {
                                        if state_guard.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Input {
                                            state_guard.cursor_position = 0;
                                        }
                                    }
                                    KeyCode::End => {
                                        if state_guard.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Input {
                                            state_guard.cursor_position = input_text.len();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
