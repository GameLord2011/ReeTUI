use crate::api::models::{BroadcastMessage, Channel, ChannelCommand};
use crate::app::app_state::AppState;
use crate::tui::chat::ws_command::WsCommand;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;

use std::time::Duration;

pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WS_URL: &str = "wss://isock.reetui.hackclub.app";

pub async fn connect(token: &str) -> Result<(WsWriter, WsReader), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(WS_URL).await?;
    let (mut writer, reader) = ws_stream.split();
    writer.send(Message::Text(token.to_string())).await?;
    Ok((writer, reader))
}

pub async fn send_message(
    writer: &mut WsWriter,
    channel_id: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let command = ChannelCommand {
        channel_id,
        content,
    };
    let payload = serde_json::to_string(&command)?;
    writer.send(Message::Text(payload)).await?;
    Ok(())
}

#[derive(serde::Deserialize, Debug)]
pub struct HistoryData {
    pub channel_id: String,
    pub messages: Vec<BroadcastMessage>,
    pub offset: usize,
    pub has_more: bool,
}

#[derive(serde::Deserialize, Debug)]
pub struct HistoryWrapper {
    #[serde(rename = "History")]
    pub history: HistoryData,
}

#[derive(serde::Deserialize, Debug)]
pub struct ChannelListWrapper {
    #[serde(rename = "ChannelList")]
    pub channels: Vec<Channel>,
}

#[derive(serde::Deserialize, Debug)]
pub struct UserListWrapper {
    #[serde(rename = "UserList")]
    pub users: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum ServerMessage {
    History(HistoryWrapper),
    ChannelList(ChannelListWrapper),
    UserList(UserListWrapper),
    ChannelUpdate(Channel),
    Broadcast(BroadcastMessage),
    Error {
        #[allow(dead_code)]
        message: String,
    },
    FileDownload {
        #[allow(dead_code)]
        file_id: String,
        #[allow(dead_code)]
        file_name: String,
    },
    Notification {
        #[allow(dead_code)]
        title: String,
        #[allow(dead_code)]
        message: String,
        #[allow(dead_code)]
        notification_type: crate::tui::notification::notification::NotificationType,
    },
}

pub async fn handle_websocket_communication(
    mut ws_reader: WsReader,
    app_state: Arc<Mutex<AppState>>,
    command_tx: mpsc::UnboundedSender<WsCommand>,
    http_client: Client,
    redraw_tx: mpsc::UnboundedSender<String>,
    cancellation_token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                log::info!("WebSocket communication cancelled.");
                break;
            }
            Some(Ok(msg)) = ws_reader.next() => {
                if let Message::Text(text) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        let mut state = app_state.lock().await;
                        log::debug!("Received WebSocket message: {:?}", server_msg);
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
                                if is_image {
                                    let app_state_clone = app_state.clone();
                                    let http_client_clone = http_client.clone();
                                    let redraw_tx_clone = redraw_tx.clone();
                                         let chat_width = state.chat_width;
                                         tokio::spawn(async move {
                                             crate::tui::chat::image_handler::process_image_message(
                                                 app_state_clone,
                                                 message,
                                                 &http_client_clone,
                                                 chat_width,
                                                 redraw_tx_clone,
                                             )
                                             .await;
                                         });                                }
                            }
                            ServerMessage::History(wrapper) => {
                                let history = wrapper.history;
                                let channel_id = history.channel_id.clone();
                                let messages = history.messages.clone();
                                state.prepend_history(&channel_id, messages.clone());
                                state
                                    .channel_history_state
                                    .insert(channel_id, (history.offset as u64, history.has_more, true));
                                for message in messages {
                                     if message.is_image.unwrap_or(false) {
                                         let app_state_clone = app_state.clone();
                                         let http_client_clone = http_client.clone();
                                         let redraw_tx_clone = redraw_tx.clone();
                                         let chat_width = state.chat_width;
                                         tokio::spawn(async move {
                                             crate::tui::chat::image_handler::process_image_message(
                                                 app_state_clone,
                                                 message,
                                                 &http_client_clone,
                                                 chat_width,
                                                 redraw_tx_clone,
                                             )
                                             .await;
                                         });
                                     }                                }
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
                                state.notification_manager.add(title, message, notification_type, Some(Duration::from_secs(3)), app_state.clone()).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    Ok(())
}