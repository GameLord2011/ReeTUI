use futures_util::StreamExt;
use reqwest::{multipart, Client};
use std::fmt;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

const API_BASE_URL: &str = "https://back.reetui.hackclub.app";

#[derive(Debug)]
pub enum FileApiError {
    RequestFailedStatus(reqwest::StatusCode),
    RequestError(reqwest::Error),
    IoError(std::io::Error),
    Other(String),
}

impl fmt::Display for FileApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileApiError::RequestFailedStatus(status) => {
                write!(f, "Request failed with status: {}", status)
            }
            FileApiError::RequestError(e) => write!(f, "Request error: {}", e),
            FileApiError::IoError(e) => write!(f, "IO error: {}", e),
            FileApiError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for FileApiError {}

impl From<reqwest::Error> for FileApiError {
    fn from(err: reqwest::Error) -> Self {
        FileApiError::RequestError(err)
    }
}

impl From<std::io::Error> for FileApiError {
    fn from(err: std::io::Error) -> Self {
        FileApiError::IoError(err)
    }
}

pub async fn upload_file(
    client: &Client,
    token: &str,
    channel_id: &str,
    file_path: PathBuf,
    progress_sender: mpsc::UnboundedSender<(String, u8)>,
) -> Result<String, FileApiError> {
    let file_name = file_path
        .file_name()
        .ok_or(FileApiError::Other("Invalid file name".to_string()))?
        .to_str()
        .ok_or(FileApiError::Other("Invalid file name".to_string()))?
        .to_string();
    let file_extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string(); // funny
    let mut file = File::open(&file_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(buffer).file_name(file_name))
        .part("file_extension", multipart::Part::text(file_extension)); // meow

    let response = client
        .post(&format!("{}/files/upload/{}", API_BASE_URL, channel_id))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        let file_id = response.text().await?;
        // Send 100% progress on successful upload
        let _ = progress_sender.send((file_id.clone(), 100)); // funny
        Ok(file_id)
    } else {
        let status = response.status();
        let _error_text = response.text().await?; // meow
        Err(FileApiError::RequestFailedStatus(status))
    }
}

pub async fn download_file(
    client: &Client,
    file_id: &str,
    file_name: &str,
    progress_sender: mpsc::UnboundedSender<(String, u8)>,
) -> Result<PathBuf, FileApiError> {
    let response = client
        .get(&format!("{}/files/download/{}", API_BASE_URL, file_id))
        .send()
        .await?;

    if response.status().is_success() {
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded_size: u64 = 0;
        let mut stream = response.bytes_stream();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(file_name);
        let mut file = File::create(&file_path).await?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded_size += chunk.len() as u64;
            let progress = ((downloaded_size as f64 / total_size as f64) * 100.0) as u8;
            let _ = progress_sender.send((file_id.to_string(), progress));
        }
        let _ = progress_sender.send((file_id.to_string(), 100)); // funny
        Ok(file_path)
    } else {
        Err(FileApiError::RequestFailedStatus(response.status()))
    }
}
