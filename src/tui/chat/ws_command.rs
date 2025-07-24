use std::path::PathBuf;

pub enum WsCommand {
    Message { channel_id: String, content: String },
    UploadFile { channel_id: String, file_path: PathBuf },
    DownloadFile { file_id: String, file_name: String }, // fields never read
    #[allow(dead_code)]
    Pong,
}
