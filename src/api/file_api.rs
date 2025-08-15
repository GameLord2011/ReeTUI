use futures_util::StreamExt;
use reqwest::{multipart, Client};
use std::fmt;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
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
                write!(f, "Request failed with status: {}\nTell to the owner (Youssef 󰊤 :'YoussefDevPro')\nIn the repo 󰌷 https://github.com/YoussefDevPro/ReeTUI", status)
            }
            FileApiError::RequestError(e) => write!(f, "Request error: {}\nTell to the owner (Youssef 󰊤 :'YoussefDevPro')\nIn the repo 󰌷 https://github.com/YoussefDevPro/ReeTUI", e),
            FileApiError::IoError(e) => write!(f, "IO error: {}\nTell to the owner (Youssef 󰊤 :'YoussefDevPro')\nIn the repo 󰌷 https://github.com/YoussefDevPro/ReeTUI", e),
            FileApiError::Other(e) => write!(f, "Error: {}\nTell to the owner (Youssef 󰊤 :'YoussefDevPro')\nIn the repo 󰌷 https://github.com/YoussefDevPro/ReeTUI", e),
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
        .ok_or_else(|| FileApiError::Other("Invalid file name ".to_string()))?
        .to_str()
        .ok_or_else(|| FileApiError::Other("Invalid file name ".to_string()))?
        .to_string();
    let file_extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string();
    let file = tokio::fs::read(&file_path).await?;

    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(file).file_name(file_name))
        .part("file_extension", multipart::Part::text(file_extension));

    let response = client
        .post(&format!("{}/files/upload/{}", API_BASE_URL, channel_id))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        let file_id = response.text().await?;
        let _ = progress_sender.send((file_id.clone(), 100));
        Ok(file_id)
    } else {
        let status = response.status();
        let _error_text = response.text().await?;
        Err(FileApiError::RequestFailedStatus(status))
    }
}

pub async fn download_file(
    client: &Client,
    file_id: &str,
    file_name: &str,
    progress_sender: mpsc::UnboundedSender<(String, u8)>,
    save_to_downloads: bool,
) -> Result<PathBuf, FileApiError> {
    let response = client
        .get(&format!("{}/files/download/{}", API_BASE_URL, file_id))
        .send()
        .await?;

    if response.status().is_success() {
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded_size: u64 = 0;
        let mut stream = response.bytes_stream();

        let file_path = if save_to_downloads {
            let current_dir = std::env::current_dir()?;
            let downloads_dir = current_dir.join("downloads");
            tokio::fs::create_dir_all(&downloads_dir).await?;

            let mut unique_file_path = downloads_dir.join(file_name);
            let mut counter = 0;
            while tokio::fs::metadata(&unique_file_path).await.is_ok() {
                counter += 1;
                let stem = PathBuf::from(file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let extension = PathBuf::from(file_name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| format!(".{}", s))
                    .unwrap_or_default();
                unique_file_path =
                    downloads_dir.join(format!("{}({}){}", stem, counter, extension));
            }
            unique_file_path
        } else {
            let temp_dir = std::env::temp_dir();
            temp_dir.join(file_name)
        };
        let mut file = File::create(&file_path).await?;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            downloaded_size += chunk.len() as u64;
            let progress = (((downloaded_size as f64 / total_size as f64) * 100.0) as u8).min(100);
            if progress_sender
                .send((file_id.to_string(), progress))
                .is_err()
            {
                // The receiver has been dropped, so we can stop sending progress updates.
                break;
            }
        }
        drop(progress_sender);
        Ok(file_path)
    } else {
        Err(FileApiError::RequestFailedStatus(response.status()))
    }
}
