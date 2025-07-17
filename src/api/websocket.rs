use crate::api::models::{BroadcastMessage, ChannelBroadcast, ChannelCommand, HistoryResponse};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WS_URL: &str = "wss:isock.reetui.hackclub.app";

pub async fn connect(token: &str) -> Result<(WsWriter, WsReader), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(WS_URL).await?;

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

pub enum ServerMessage {
    ChatMessage(BroadcastMessage),
    ChannelUpdate(ChannelBroadcast),
    ChannelDelete(String),
    History(Vec<BroadcastMessage>),
    ActiveUsers(Vec<String>),
    Unknown(String),
}

pub fn parse_server_message(msg_text: &str) -> ServerMessage {
    if let Ok(history_response) = serde_json::from_str::<HistoryResponse>(msg_text) {
        return ServerMessage::History(history_response.history);
    }

    if let Some(json_str) = msg_text.strip_prefix("/channel_update ") {
        if let Ok(channel) = serde_json::from_str(json_str) {
            return ServerMessage::ChannelUpdate(channel);
        }
    }

    if let Some(channel_id) = msg_text.strip_prefix("/channel_delete ") {
        return ServerMessage::ChannelDelete(channel_id.to_string());
    }

    if let Ok(active_users) = serde_json::from_str::<Vec<String>>(msg_text) {
        return ServerMessage::ActiveUsers(active_users);
    }

    if let Ok(channel_broadcast) = serde_json::from_str(msg_text) {
        return ServerMessage::ChannelUpdate(channel_broadcast);
    }

    if let Ok(chat_msg) = serde_json::from_str(msg_text) {
        return ServerMessage::ChatMessage(chat_msg);
    }

    ServerMessage::Unknown(msg_text.to_string())
}
