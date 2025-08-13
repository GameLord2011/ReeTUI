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
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, Connector};
use tokio_tungstenite::connect_async_tls_with_config;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, RootCertStore, SignatureScheme};

#[derive(Debug)]
struct NoVerification;

impl ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use lazy_static::lazy_static;
use std::time::Duration;

lazy_static! {
    static ref THEME_KEYWORDS: Vec<&'static str> = vec!["gizzy", "zombi"];
}

pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WS_URL: &str = "wss://isock.reetui.hackclub.app";

pub async fn connect(token: &str) -> Result<(WsWriter, WsReader), Box<dyn std::error::Error>> {
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
        .expect("Failed to install default crypto provider");
    // Create a custom rustls client config that trusts any certificate
    //
    // THIS IS INSECURE AND SHOULD NOT BE USED IN PRODUCTION
    //
    let root_store = RootCertStore::empty();
    let mut client_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    client_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let mut dangerous_config = client_config.dangerous();
    dangerous_config.set_certificate_verifier(Arc::new(NoVerification));

    let connector = Connector::Rustls(Arc::new(client_config));

    let (ws_stream, _) = connect_async_tls_with_config(WS_URL, None, true, Some(connector)).await?;
    let (mut writer, reader) = ws_stream.split();
    writer.send(Message::Text(token.to_string().into())).await?;
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
    writer.send(Message::Text(payload.into())).await?;
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
pub struct ChannelUpdateWrapper {
    #[serde(rename = "ChannelUpdate")]
    pub channel: Channel,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum ServerMessage {
    History(HistoryWrapper),
    ChannelList(ChannelListWrapper),
    UserList(UserListWrapper),
    ChannelUpdate(ChannelUpdateWrapper),
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

                break;
            }
            Some(Ok(msg)) = ws_reader.next() => {
                if let Message::Text(text) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
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
                            ServerMessage::Broadcast(mut message) => {
                                message.client_id = Some(Uuid::new_v4().to_string());
                                if let Some(username) = &state.username {
                                    let mention = format!("@{}", username);
                                    if message.content.contains(&mention) {

                                    }
                                }
                                let is_image = message.is_image.unwrap_or(false);
                                if message.file_id.is_some() {
                                    // This is a file message, add it to downloadable_files
                                    let downloadable_file = crate::app::app_state::DownloadableFile {
                                        file_id: message.file_id.clone().unwrap(),
                                        file_name: message.file_name.clone().unwrap_or_default(),
                                        file_extension: message.file_extension.clone().unwrap_or_default(),
                                        file_size: (message.file_size_mb.unwrap_or(0.0) * 1024.0 * 1024.0) as u64,
                                        sender_username: message.user.clone(),
                                        sender_icon: message.icon.clone(),
                                        devicon: message.file_icon.clone().unwrap_or_default(),
                                    };
                                    state.downloadable_files.insert(downloadable_file.file_id.clone(), downloadable_file);
                                }
                                state.add_message(message.clone());
                                if THEME_KEYWORDS.iter().any(|&word| message.content.contains(word)) {
                                    let now = tokio::time::Instant::now();
                                    if now.duration_since(state.last_theme_change_time) >= Duration::from_secs(1) {
                                        let current_theme_name = &state.current_theme.name;
                                        let theme_names: Vec<&crate::themes::ThemeName> = state.themes.keys().collect();
                                        if let Some(current_index) = theme_names.iter().position(|&name| name == current_theme_name) {
                                            let next_index = (current_index + 1) % theme_names.len();
                                            let next_theme_name = theme_names[next_index];
                                            if let Some(next_theme) = state.themes.get(next_theme_name).cloned() {
                                                state.current_theme = next_theme;
                                                state.last_theme_change_time = now;
                                            }
                                        }
                                    }
                                }
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
                                    });
                                }
                            }
                            ServerMessage::History(wrapper) => {
                                let history = wrapper.history;
                                let channel_id = history.channel_id.clone();
                                let mut messages = history.messages;
                                for message in messages.iter_mut() {
                                    message.client_id = Some(Uuid::new_v4().to_string());
                                }
                                 if state.messages.get(&channel_id).map_or(true, |m| m.is_empty()) {
                                    state.prepend_history(&channel_id, messages.clone());
                                    state
                                        .channel_history_state
                                        .insert(channel_id.clone(), (history.offset as u64, history.has_more, true));
                                    state.update_last_message_count(channel_id.clone(), messages.len());
                                    state.set_initial_load_complete(true);
                                }                                for message in messages {
                                    if message.file_id.is_some() {
                                        let downloadable_file = crate::app::app_state::DownloadableFile {
                                            file_id: message.file_id.clone().unwrap(),
                                            file_name: message.file_name.clone().unwrap_or_default(),
                                            file_extension: message.file_extension.clone().unwrap_or_default(),
                                            file_size: (message.file_size_mb.unwrap_or(0.0) * 1024.0 * 1024.0) as u64,
                                            sender_username: message.user.clone(),
                                            sender_icon: message.icon.clone(),
                                            devicon: message.file_icon.clone().unwrap_or_default(),
                                        };
                                        state.downloadable_files.insert(downloadable_file.file_id.clone(), downloadable_file);
                                    }
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
                                    }
                                }
                            }
                            ServerMessage::UserList(wrapper) => {

                                state.active_users = wrapper.users;
                            }
                            ServerMessage::ChannelUpdate(wrapper) => {
                                let channel = wrapper.channel;
                                state.add_or_update_channel(channel.clone());
                                state.set_current_channel(channel.clone());
                                let _ = command_tx.send(WsCommand::Message {
                                    channel_id: channel.id.clone(),
                                    content: format!("/get_history {} 0", channel.id),
                                });
                                let _ = redraw_tx.send(String::new());
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
