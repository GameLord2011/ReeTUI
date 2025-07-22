pub mod create_channel_form;
pub mod message_parsing;
pub mod popups;
pub mod theme_settings_form;
pub mod ui;
pub mod utils;
pub mod ws_command;

#[cfg(test)]
pub mod tests;



use crate::api::models::BroadcastMessage;
use crate::api::websocket::{self, ServerMessage};
use crate::app::{AppState, NotificationType, PopupType};
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use emojis;
use futures_util::{SinkExt, StreamExt};


use ratatui::{prelude::Backend, widgets::ListState, Terminal};
use regex::Regex;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
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
use serde_json;

pub async fn run_chat_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut input_text = String::new();
    let mut channel_list_state = ListState::default();
    channel_list_state.select(Some(0));

    let mut create_channel_form = CreateChannelForm::new();
    let mut theme_settings_form = ThemeSettingsForm::new(app_state.lock().await.current_theme);
    let mut file_manager = popups::file_manager::FileManager::new(
        popups::file_manager::FileManagerMode::LocalUpload,
        Vec::new(),
    );

    let (mut ws_writer, mut ws_reader) = {
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
                        log::error!("Failed to send message via websocket");
                        break;
                    }
                }
                WsCommand::Pong => {
                    if ws_writer
                        .send(tungstenite::Message::Pong(vec![].into()))
                        .await
                        .is_err()
                    {
                        log::error!("Failed to send pong");
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
                        let state_guard = app_state_clone_for_file_commands.lock().await;
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
                            let mut state_guard = app_state_clone_for_file_commands.lock().await;
                            state_guard.set_notification(
                                "File Upload Success".to_string(),
                                format!("File uploaded: {}", file_id),
                                NotificationType::Success,
                            );
                        }
                        Err(e) => {
                            let mut state_guard = app_state_clone_for_file_commands.lock().await;
                            state_guard.set_notification(
                                "File Upload Failed".to_string(),
                                format!("File upload failed: {}", e.to_string()),
                                NotificationType::Error,
                            );
                        }
                    }
                }
                WsCommand::DownloadFile { file_id, file_name } => {
                    let _ = {
                        let state_guard = app_state_clone_for_file_commands.lock().await;
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
                            let mut state_guard = app_state_clone_for_file_commands.lock().await;
                            state_guard.set_notification(
                                "File Download Success".to_string(),
                                format!("Downloaded {} to {:?}", file_name, download_path),
                                NotificationType::Success,
                            );
                        }
                        Err(e) => {
                            let mut state_guard = app_state_clone_for_file_commands.lock().await;
                            state_guard.set_notification(
                                "File Download Failed".to_string(),
                                format!("Download failed: {}", e.to_string()),
                                NotificationType::Error,
                            );
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
            let mut state_guard = app_state_clone_for_progress.lock().await;
            state_guard.download_progress = progress;
            if progress < 100 {
                state_guard.popup_state.show = true;
                state_guard.popup_state.popup_type = PopupType::DownloadProgress;
            } else {
                state_guard.popup_state.show = false;
                state_guard.popup_state.popup_type = PopupType::None;
            }
            state_guard.set_notification(
                "File Transfer Progress".to_string(),
                format!("File transfer progress: {}%", progress),
                NotificationType::Info,
            );
        }
    });

    if command_tx
        .send(WsCommand::Message {
            channel_id: "home".to_string(),
            content: "/get_history home 0".to_string(),
        })
        .is_err()
    {
        app_state.lock().await.set_notification(
            "Command Error".to_string(),
            "Failed to send command".to_string(),
            NotificationType::Error,
        );
    }

    let mut last_rendered_width: u16 = 0;

    let mention_regex = Regex::new(r"@(\w+)").unwrap();
    let emoji_regex = Regex::new(r":([a-zA-Z0-9_+-]+):").unwrap();

    loop {
        let mut state_guard = app_state.lock().await;
        state_guard.clear_expired_notification();

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
                if let Some(tui_page) = handle_key_event::<B>(
                    key,
                    &mut state_guard,
                    &mut input_text,
                    &mut channel_list_state,
                    &mut create_channel_form,
                    &mut theme_settings_form,
                    &mut file_manager,
                    &filtered_users,
                    &filtered_emojis,
                    &command_tx,
                    &file_command_tx,
                )
                .await?
                {
                    return Ok(tui_page);
                }
            }
        }

        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(10), ws_reader.next()).await
        {
            handle_websocket_message(msg, app_state.clone(), &http_client, &command_tx, &file_command_tx).await?;
        }
    }
}



async fn handle_file_message(
    app_state_arc: Arc<tokio::sync::Mutex<AppState>>,
    msg: &mut BroadcastMessage,
    client: &reqwest::Client,
) {
    log::debug!("handle_file_message called for msg: {:?}", msg);
    if msg.is_image.unwrap_or(false) {
        log::debug!("Message is an image.");
        if let Some(download_url) = &msg.download_url {
            log::debug!("Download URL found: {}", download_url);
            let cache_dir = env::temp_dir().join("ReeTUI_cache");
            if !cache_dir.exists() {
                log::debug!("Cache directory does not exist, creating: {:?}", cache_dir);
                fs::create_dir_all(&cache_dir).unwrap_or_default();
            }
            let file_name = msg.file_name.clone().unwrap_or_default();
            let file_extension = Path::new(&file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("tmp");
            let cached_image_path = cache_dir.join(format!(
                "{}.{}",
                &msg.file_id.clone().unwrap(),
                file_extension
            ));

            if !cached_image_path.exists() {
                log::debug!(
                    "Cached image does not exist, downloading to: {:?}",
                    cached_image_path
                );
                if let Ok(response) = client.get(download_url).send().await {
                    log::debug!("Download response status: {}", response.status());
                    if let Ok(bytes) = response.bytes().await {
                        log::debug!("Image bytes downloaded. Size: {}", bytes.len());
                        if let Ok(mut file) = tokio::fs::File::create(&cached_image_path).await {
                            let _ = file.write_all(&bytes).await;
                            log::debug!("Image saved to cache: {:?}", cached_image_path);
                        } else {
                            log::error!(
                                "Failed to create file for caching: {:?}",
                                cached_image_path
                            );
                        }
                    } else {
                        log::error!(
                            "Failed to get bytes from response for download URL: {}",
                            download_url
                        );
                    }
                } else {
                    log::error!(
                        "Failed to send GET request for download URL: {}",
                        download_url
                    );
                }
            } else {
                log::debug!("Cached image already exists: {:?}", cached_image_path);
            }

            if cached_image_path.exists() {
                let msg_clone_for_spawn = msg.clone(); // Clone msg for the spawned task
                tokio::spawn(async move {
                    log::debug!(
                        "Attempting to generate chafa preview for: {:?}, for message: {:?}",
                        cached_image_path,
                        msg_clone_for_spawn
                    );
                    let chafa_command = tokio::process::Command::new("chafa")
                        .arg("--size=x7")
                        .arg(&cached_image_path)
                        .output()
                        .await;

                    match chafa_command {
                        Ok(output) => {
                            if output.status.success() {
                                let preview = String::from_utf8_lossy(&output.stdout).to_string();
                                log::debug!("Chafa preview generated. Length: {}", preview.len());
                                let mut msg_to_update = msg_clone_for_spawn;
                                msg_to_update.image_preview = Some(preview);
                                let mut state_guard = app_state_arc.lock().await; // Acquire lock here
                                state_guard.add_message(msg_to_update.clone()); // Clone for add_message
                                state_guard
                                    .rendered_messages
                                    .remove(&msg_to_update.channel_id);
                                log::debug!("Message with image_preview added to state.");
                            } else {
                                log::error!(
                                    "Chafa command failed. Stderr: {}",
                                    String::from_utf8_lossy(&output.stderr)
                                );
                                log::error!(
                                    "Chafa command failed. Stdout: {}",
                                    String::from_utf8_lossy(&output.stdout)
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to execute chafa command: {}", e);
                        }
                    }
                });
            } else {
                log::warn!("Cached image path does not exist after download attempt: {:?}, for message: {:?}", cached_image_path, msg);
            }
        } else {
            log::warn!("Image message has no download_url: {:?}", msg);
        }
    } else {
        log::debug!("Message is not an image, skipping chafa processing.");
    }
}

async fn handle_websocket_message(
    msg: tungstenite::Message,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    command_tx: &mpsc::UnboundedSender<WsCommand>,
    file_command_tx: &mpsc::UnboundedSender<WsCommand>,
) -> io::Result<()> {
    match msg {
        tungstenite::Message::Text(text) => {
            log::debug!("Received text message: {}", text);
            if let Ok(server_message) = serde_json::from_str::<ServerMessage>(&text) {
                let mut state_guard = app_state.lock().await;
                match server_message {
                    ServerMessage::Broadcast(mut broadcast_msg) => {
                        log::debug!("Received broadcast message: {:?}", broadcast_msg);
                        if broadcast_msg.is_image.unwrap_or(false) {
                            handle_file_message(app_state.clone(), &mut broadcast_msg, http_client).await;
                        } else {
                            state_guard.add_message(broadcast_msg.clone());
                            state_guard.rendered_messages.remove(&broadcast_msg.channel_id);
                        }
                    }
                    ServerMessage::ChannelList(channels) => {
                        log::debug!("Received channel list: {:?}", channels);
                        let new_channels = channels;
                        let mut channel_to_set: Option<crate::api::models::Channel> = None;

                        if state_guard.current_channel.is_none() && !new_channels.is_empty() {
                            channel_to_set = Some(new_channels[0].clone());
                        }

                        state_guard.channels = new_channels;

                        if let Some(channel) = channel_to_set {
                            state_guard.set_current_channel(channel);
                        }
                    }
                    ServerMessage::History { channel_id, messages, offset, has_more } => {
                        log::debug!(
                            "Received history for channel {}: {} messages, offset {}, has_more {}",
                            channel_id,
                            messages.len(),
                            offset,
                            has_more
                        );
                        let mut new_messages = messages;
                        new_messages.extend(state_guard.messages.get(&channel_id).cloned().unwrap_or_default());
                        state_guard.messages.insert(channel_id.clone(), new_messages.into());
                        state_guard.channel_history_state.insert(channel_id.clone(), (offset, has_more));
                        state_guard.rendered_messages.remove(&channel_id);
                    }
                    ServerMessage::UserList(users) => {
                        log::debug!("Received user list: {:?}", users);
                        state_guard.active_users = users;
                    }
                    ServerMessage::Notification {
                        title,
                        message,
                        notification_type,
                    } => {
                        log::debug!(
                            "Received notification: {} - {} ({:?})",
                            title,
                            message,
                            notification_type
                        );
                        state_guard.set_notification(title, message, notification_type);
                    }
                    ServerMessage::Error { message } => {
                        log::error!("Received error from server: {}", message);
                        state_guard.set_notification(
                            "Server Error".to_string(),
                            message,
                            NotificationType::Error,
                        );
                    }
                    ServerMessage::Pong => {
                        log::debug!("Received Pong from server");
                    }
                    ServerMessage::FileDownload { file_id, file_name } => {
                        log::debug!("Received file download request: {} ({})", file_name, file_id);
                        if file_command_tx
                            .send(WsCommand::DownloadFile { file_id, file_name })
                            .is_err()
                        {
                            state_guard.set_notification(
                                "Download Error".to_string(),
                                "Failed to send download command".to_string(),
                                NotificationType::Error,
                            );
                        }
                    }
                }
            } else {
                log::warn!("Failed to parse server message: {}", text);
            }
        }
        tungstenite::Message::Ping(_) => {
            log::debug!("Received Ping from server");
            if command_tx.send(WsCommand::Pong).is_err() {
                log::error!("Failed to send pong command");
            }
        }
        tungstenite::Message::Close(close_frame) => {
            log::info!("WebSocket connection closed: {:?}", close_frame);
            app_state.lock().await.set_notification(
                "Disconnected".to_string(),
                "Disconnected from server.".to_string(),
                NotificationType::Error,
            );
            return Ok(());
        }
        _ => {
            log::warn!("Received unhandled WebSocket message type");
        }
    }
    Ok(())
}

async fn handle_key_event<B: Backend>(
    key: KeyEvent,
    state_guard: &mut tokio::sync::MutexGuard<'_, AppState>,
    input_text: &mut String,
    channel_list_state: &mut ListState,
    create_channel_form: &mut CreateChannelForm,
    theme_settings_form: &mut ThemeSettingsForm,
    file_manager: &mut popups::file_manager::FileManager,
    filtered_users: &Vec<String>,
    filtered_emojis: &Vec<String>,
    command_tx: &mpsc::UnboundedSender<WsCommand>,
    file_command_tx: &mpsc::UnboundedSender<WsCommand>,
) -> io::Result<Option<TuiPage>> {
    if key.kind == KeyEventKind::Press {
        let num_filtered_users = filtered_users.len();
        let num_filtered_emojis = filtered_emojis.len();

        if state_guard.popup_state.show {
            match state_guard.popup_state.popup_type {
                PopupType::Quit => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        return Ok(Some(TuiPage::Exit));
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
                            *theme_settings_form =
                                ThemeSettingsForm::new(state_guard.current_theme);
                        }
                        1 => {
                            state_guard.popup_state.popup_type = PopupType::Deconnection;
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
                        *create_channel_form = CreateChannelForm::new();
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
                        if create_channel_form.input_focused == CreateChannelInput::Icon {
                            create_channel_form.previous_icon();
                        }
                    }
                    KeyCode::Right => {
                        if create_channel_form.input_focused == CreateChannelInput::Icon {
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
                                let channel_icon = create_channel_form.get_selected_icon();

                                let command =
                                    format!("/propose_channel {} {}", channel_name, channel_icon);
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
                                    );
                                } else {
                                    state_guard.set_notification(
                                        "Channel Creation Success".to_string(),
                                        format!("Channel '{}' created!", channel_name),
                                        NotificationType::Success,
                                    );
                                    state_guard.popup_state.show = false;
                                    state_guard.popup_state.popup_type = PopupType::None;
                                    *create_channel_form = CreateChannelForm::new();
                                }
                            } else {
                                state_guard.set_notification(
                                    "Channel Creation Warning".to_string(),
                                    "Channel name cannot be empty!".to_string(),
                                    NotificationType::Warning,
                                );
                            }
                        }
                    },
                    _ => {}
                },
                PopupType::SetTheme => match key.code {
                    KeyCode::Up => {
                        theme_settings_form.previous_theme();
                        state_guard.current_theme = theme_settings_form.get_selected_theme();
                    }
                    KeyCode::Down => {
                        theme_settings_form.next_theme();
                        state_guard.current_theme = theme_settings_form.get_selected_theme();
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
                        return Ok(Some(TuiPage::Auth));
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
                            state_guard.selected_mention_index =
                                (state_guard.selected_mention_index + num_filtered_users - 1)
                                    % num_filtered_users;
                        } else {
                            state_guard.selected_mention_index = 0;
                        }
                    }
                    KeyCode::Down => {
                        if num_filtered_users > 0 {
                            state_guard.selected_mention_index =
                                (state_guard.selected_mention_index + 1) % num_filtered_users;
                        } else {
                            state_guard.selected_mention_index = 0;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(user) = filtered_users.get(state_guard.selected_mention_index) {
                            let query_start = input_text.rfind('@').unwrap_or(input_text.len());
                            input_text.replace_range(query_start.., &format!("@{} ", user));
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
                            state_guard.mention_query = input_text[last_at_idx + 1..].to_string();
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
                                state_guard.selected_emoji_index =
                                    (state_guard.selected_emoji_index + num_filtered_emojis - 1)
                                        % num_filtered_emojis;
                            } else {
                                state_guard.selected_emoji_index = 0;
                            }
                        }
                        KeyCode::Down => {
                            if num_filtered_emojis > 0 {
                                state_guard.selected_emoji_index =
                                    (state_guard.selected_emoji_index + 1) % num_filtered_emojis;
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
                                    state_guard.cursor_position = query_start + emoji_str.len();
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
                                let potential_shortcode_with_colons = &input_text[last_colon_idx..];
                                if potential_shortcode_with_colons.ends_with(':')
                                    && potential_shortcode_with_colons.len() > 1
                                {
                                    let shortcode = &potential_shortcode_with_colons
                                        [1..potential_shortcode_with_colons.len() - 1];

                                    if !shortcode.contains(' ') {
                                        if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                                            input_text
                                                .replace_range(last_colon_idx.., emoji.as_str());
                                            state_guard.cursor_position =
                                                last_colon_idx + emoji.as_str().len();
                                            state_guard.popup_state.show = false;
                                            state_guard.popup_state.popup_type = PopupType::None;
                                            state_guard.emoji_query.clear();
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } // Added this closing brace
                PopupType::DownloadProgress => {
                    // No specific key handling for download progress popup
                }
                PopupType::Notification => {
                    // No specific key handling for notification popup
                }
                PopupType::DebugJson => match key.code {
                    KeyCode::Esc => {
                        state_guard.popup_state.show = false;
                        state_guard.popup_state.popup_type = PopupType::None;
                    }
                    _ => {}
                },
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
                                            file_path: path.clone(),
                                        })
                                        .is_err()
                                    {
                                        state_guard.set_notification(
                                            "File Upload Error".to_string(),
                                            "Failed to send upload command".to_string(),
                                            NotificationType::Error,
                                        );
                                    }
                                } else {
                                    state_guard.set_notification(
                                        "File Upload Warning".to_string(),
                                        "No channel selected to upload file.".to_string(),
                                        NotificationType::Warning,
                                    );
                                }
                                state_guard.popup_state.show = false;
                                state_guard.popup_state.popup_type = PopupType::None;
                            }
                            popups::file_manager::FileManagerEvent::FileSelectedForDownload(
                                file_id,
                                file_name,
                            ) => {
                                if file_command_tx
                                    .send(WsCommand::DownloadFile { file_id, file_name })
                                    .is_err()
                                {
                                    state_guard.set_notification(
                                        "Download Error".to_string(),
                                        "Failed to send download command".to_string(),
                                        NotificationType::Error,
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
                        state_guard.set_notification(
                            "Active Users Request Error".to_string(),
                            "Failed to request active users".to_string(),
                            NotificationType::Error,
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
                    *create_channel_form = CreateChannelForm::new();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state_guard.popup_state.show = true;
                    state_guard.popup_state.popup_type = PopupType::FileManager;
                    *file_manager = popups::file_manager::FileManager::new(
                        popups::file_manager::FileManagerMode::LocalUpload,
                        Vec::new(),
                    );
                }
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(current_channel) = &state_guard.current_channel {
                        if let Some(messages) = state_guard.messages.get(&current_channel.id) {
                            if let Some(last_message) = messages.back() {
                                if let Ok(json_string) = serde_json::to_string_pretty(last_message)
                                {
                                    state_guard.debug_json_content = json_string;
                                    state_guard.popup_state.show = true;
                                    state_guard.popup_state.popup_type = PopupType::DebugJson;
                                } else {
                                    state_guard.set_notification(
                                        "JSON Error".to_string(),
                                        "Failed to serialize message to JSON".to_string(),
                                        NotificationType::Error,
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
                    if let Some(selected_channel) = state_guard.channels.get(i).cloned() {
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
                                );
                            }
                        }
                    }
                }
                KeyCode::Up => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        let channel_id = state_guard.current_channel.as_ref().unwrap().id.clone();
                        let rendered_count =
                            state_guard.messages.get(&channel_id).map_or(0, |v| v.len());
                        if state_guard.message_scroll_offset >= rendered_count.saturating_sub(5) {
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
                                        state_guard.set_notification(
                                            "History Request Error".to_string(),
                                            "Failed to request history".to_string(),
                                            NotificationType::Error,
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
                        if let Some(selected_channel) = state_guard.channels.get(i).cloned() {
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
                        if let Some(selected_channel) = state_guard.channels.get(i).cloned() {
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
                                    if let Some(current_channel) = &state_guard.current_channel {
                                        let channel_id = current_channel.id.clone();
                                        if file_command_tx
                                            .send(WsCommand::UploadFile {
                                                channel_id,
                                                file_path: file_path.clone(),
                                            })
                                            .is_err()
                                        {
                                            state_guard.set_notification(
                                                "File Upload Error".to_string(),
                                                "Failed to send upload command".to_string(),
                                                NotificationType::Error,
                                            );
                                        }
                                    } else {
                                        state_guard.set_notification(
                                            "No Channel Selected".to_string(),
                                            "No channel selected to upload file.".to_string(),
                                            NotificationType::Warning,
                                        );
                                    }
                                } else {
                                    state_guard.set_notification(
                                        "File Not Found".to_string(),
                                        format!("File not found: {}", file_path_str),
                                        NotificationType::Error,
                                    );
                                }
                            }
                        } else if input_text.starts_with("/download ") {
                            let parts: Vec<&str> = input_text.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                let file_id_to_download = parts[1].trim().to_string();
                                let mut file_to_download: Option<BroadcastMessage> = None;
                                if let Some(current_channel) = &state_guard.current_channel {
                                    if let Some(messages) =
                                        state_guard.messages.get(&current_channel.id)
                                    {
                                        for msg in messages.iter().rev() {
                                            if let Some(file_id) = &msg.file_id {
                                                if file_id == &file_id_to_download {
                                                    file_to_download = Some(msg.clone());
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(file_info) = file_to_download {
                                    if let Some(file_name) = file_info.file_name {
                                        if file_command_tx
                                            .send(WsCommand::DownloadFile {
                                                file_id: file_id_to_download,
                                                file_name,
                                            })
                                            .is_err()
                                        {
                                            state_guard.set_notification(
                                                "Download Error".to_string(),
                                                "Failed to send download command".to_string(),
                                                NotificationType::Error,
                                            );
                                        }
                                    } else {
                                        state_guard.set_notification(
                                            "File Info Error".to_string(),
                                            "File name not found in message".to_string(),
                                            NotificationType::Error,
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
                                    );
                                }
                            }
                        } else {
                            if let Some(current_channel) = &state_guard.current_channel {
                                let channel_id = current_channel.id.clone();
                                let content = replace_shortcodes_with_emojis(&input_text);
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
    Ok(None)
}

                                    
