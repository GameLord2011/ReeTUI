use reqwest::{Client, multipart};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const API_BASE_URL: &str = "https://back.reetui.hackclub.app";

pub async fn upload_file(
    client: &Client,
    token: &str,
    channel_id: &str,
    file_path: PathBuf,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
    let mut file = File::open(&file_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(buffer).file_name(file_name));

    let response = client
        .post(&format!("{}/files/upload/{}", API_BASE_URL, channel_id))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        let status = response.status();
        let error_text = response.text().await?;
        Err(format!("Failed to upload file ({}): {}", status, error_text).into())
    }
}

pub async fn download_file(
    client: &Client,
    file_id: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = client
        .get(&format!("{}/files/download/{}", API_BASE_URL, file_id))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(format!("Failed to download file: {:?}", response.status()).into())
    }
}
