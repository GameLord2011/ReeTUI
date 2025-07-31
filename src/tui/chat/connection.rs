use crate::{
    api::{file_api, websocket, models::BroadcastMessage},
    app::{AppState, NotificationType},
    tui::chat::ws_command::WsCommand,
};
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn handle_commands(
    mut command_rx: mpsc::UnboundedReceiver<WsCommand>,
    mut ws_writer: websocket::WsWriter,
) {
    log::debug!("connection: handle_commands task started.");
    while let Some(command) = command_rx.recv().await {
        log::debug!("connection: Received command: {:?}", command);
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
}

pub async fn handle_file_commands(
    mut file_command_rx: mpsc::UnboundedReceiver<WsCommand>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
    http_client: reqwest::Client,
    progress_tx: mpsc::UnboundedSender<(String, u8)>,
) {
    log::debug!("connection: handle_file_commands task started.");
    while let Some(command) = file_command_rx.recv().await {
        log::debug!("connection: Received file command: {:?}", command);
        match command {
            WsCommand::UploadFile {
                channel_id,
                file_path,
            } => {
                let token = {
                    let state = app_state.lock().await;
                    state.auth_token.clone()
                };
                if let Some(token) = token.as_deref() {
                    match file_api::upload_file(
                        &http_client,
                        &token,
                        &channel_id,
                        file_path,
                        progress_tx.clone(),
                    )
                    .await
                    {
                        Ok(_file_id) => {
                            let mut state = app_state.lock().await;
                            state.set_notification(
                                "File Upload Success".to_string(),
                                "File uploaded successfully!".to_string(),
                                NotificationType::Success,
                                3,
                            );
                        }
                        Err(e) => {
                            let mut state = app_state.lock().await;
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
                let app_state_clone = app_state.clone();
                let progress_tx_clone = progress_tx.clone();
                let http_client_clone = http_client.clone();
                tokio::spawn(async move {
                    match file_api::download_file(
                        &http_client_clone,
                        &file_id,
                        &file_name,
                        progress_tx_clone,
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut state = app_state_clone.lock().await;
                            state.set_notification(
                                "File Download Success".to_string(),
                                format!("File '{}' downloaded successfully!", file_name),
                                NotificationType::Success,
                                3,
                            );
                        }
                        Err(e) => {
                            let mut state = app_state_clone.lock().await;
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
}

pub async fn handle_progress_updates(
    mut progress_rx: mpsc::UnboundedReceiver<(String, u8)>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) {
    log::debug!("connection: handle_progress_updates task started.");
    while let Some((file_id, progress)) = progress_rx.recv().await {
        log::debug!("connection: Received progress update for file_id: {}, progress: {}", file_id, progress);
        let mut state = app_state.lock().await;
        state.set_download_progress_popup(progress);
        if progress == 100 {
            state.popup_state.show = false;
            state.popup_state.popup_type = crate::app::PopupType::None;
        }
    }
}

pub async fn handle_websocket_messages(
    mut ws_reader: websocket::WsReader,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
    command_tx: mpsc::UnboundedSender<WsCommand>,
    http_client: reqwest::Client,
) {
    log::debug!("connection: handle_websocket_messages task started.");
    use crate::api::websocket::ServerMessage;
    use futures_util::StreamExt;

    while let Some(Ok(msg)) = ws_reader.next().await {
        log::debug!("connection: Received raw WebSocket message: {:?}", msg);
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            log::debug!("connection: Received WebSocket text: {}", text);
            if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                log::debug!("connection: Parsed ServerMessage: {:?}", server_msg);
                let mut state = app_state.lock().await;
                match server_msg {
                    ServerMessage::ChannelList(wrapper) => {
                        state.channels = wrapper.channels;
                        if let Some(first_channel) = state.channels.get(0).cloned() {
                            let channel_id = first_channel.id.clone();
                            state.set_current_channel(first_channel);
                            let _ = command_tx.send(WsCommand::Message {
                                channel_id: channel_id.clone(),
                                content: format!("/get_history {} 0", channel_id),
                            });
                        }
                    }
                    ServerMessage::Broadcast(message) => {
                        let is_image = message.is_image.unwrap_or(false);
                        state.add_message(message.clone());
                        state.needs_re_render.insert(message.channel_id.clone(), true);
                        if is_image {
                            let app_state_clone = app_state.clone();
                            let http_client_clone = http_client.clone();
                            tokio::spawn(async move {
                                crate::tui::chat::image_handler::process_image_message(
                                    app_state_clone,
                                    message,
                                    &http_client_clone,
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
                        for message in messages {
                            if message.is_image.unwrap_or(false) {
                                let app_state_clone = app_state.clone();
                                let http_client_clone = http_client.clone();
                                tokio::spawn(async move {
                                    crate::tui::chat::image_handler::process_image_message(
                                        app_state_clone,
                                        message,
                                        &http_client_clone,
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
}
