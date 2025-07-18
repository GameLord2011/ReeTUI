pub enum WsCommand {
    Message { channel_id: String, content: String },
    Pong,
}
