use crate::api::models::{BroadcastMessage, Channel, ChannelCommand};
use crate::app::NotificationType;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WS_URL: &str = "wss://isock.reetui.hackclub.app"; // i suck

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

#[derive(serde::Deserialize)]
pub enum ServerMessage {
    Broadcast(BroadcastMessage),
    ChannelList(Vec<Channel>),
    History {
        channel_id: String,
        messages: Vec<BroadcastMessage>,
        offset: usize,
        has_more: bool,
    },
    UserList(Vec<String>),
    Notification {
        title: String,
        message: String,
        notification_type: NotificationType,
    },
    Error { message: String },
    Pong,
    FileDownload { file_id: String, file_name: String },
}


