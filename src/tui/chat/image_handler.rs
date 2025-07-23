use std::{env, fs, io, path::{Path, PathBuf}, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use chrono::Utc;
use sha2::Digest;
use ratatui::text::{Line, Span, Text};
use ratatui::style::{Modifier, Style};
use ansi_to_tui::IntoText as _;

use crate::api::models::BroadcastMessage;
use crate::app::{AppState, NotificationType, PopupType};
use crate::tui::themes::{get_theme, rgb_to_color};

pub async fn handle_file_message(
    app_state_arc: Arc<tokio::sync::Mutex<AppState>>,
    msg: &mut BroadcastMessage,
    client: &reqwest::Client,
) {
    log::debug!("handle_file_message called for message: {:?}", msg);
    if msg.is_image.unwrap_or(false) {
        log::debug!("Message identified as image. Proceeding with image handling.");
        if let Some(download_url) = &msg.download_url {
            log::info!("Processing image with download URL: {}", download_url);
            let cache_dir = env::temp_dir().join("ReeTUI_cache");
            log::debug!("Using cache directory: {:?}", cache_dir);
            if !cache_dir.exists() {
                log::info!("Cache directory {:?} does not exist. Creating it.", cache_dir);
                fs::create_dir_all(&cache_dir).unwrap_or_default();
            }
            let file_name = msg.file_name.clone().unwrap_or_default();
            let file_extension = Path::new(&file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("tmp");
            let cached_image_path = cache_dir.join(format!(
                "{}.{}",
                &msg.file_id.clone().unwrap(),
                file_extension
            ));
            log::info!("Cached image path determined to be: {:?}", cached_image_path);

            if !cached_image_path.exists() {
                log::info!("Image not found in cache. Downloading from URL: {}", download_url);
                match client.get(download_url).send().await {
                    Ok(response) => {
                        log::debug!("Received response for image download. Status: {}", response.status());
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => {
                                    log::debug!("Successfully read {} bytes from response.", bytes.len());
                                    match tokio::fs::File::create(&cached_image_path).await {
                                        Ok(mut file) => {
                                            log::debug!("File created at cache path: {:?}", cached_image_path);
                                            match file.write_all(&bytes).await {
                                                Ok(_) => log::debug!("Successfully wrote image to cache: {:?}", cached_image_path),
                                                Err(e) => log::error!("Failed to write image to cache {:?}: {}", cached_image_path, e),
                                            }
                                        },
                                        Err(e) => log::error!("Failed to create file for caching {:?}: {}", cached_image_path, e),
                                    }
                                },
                                Err(e) => log::error!("Failed to get bytes from response for download URL {}: {}", download_url, e),
                            }
                        } else {
                            log::error!("Download failed with status {}: {}", response.status(), response.text().await.unwrap_or_default());
                        }
                    },
                    Err(e) => log::error!("Failed to send GET request for download URL {}: {}", download_url, e),
                }
            } else {
                log::info!("Image found in cache at: {:?}", cached_image_path);
            }
            if cached_image_path.exists() {
                log::debug!("Cached image path exists: {:?}", cached_image_path);
                if let Ok(metadata) = fs::metadata(&cached_image_path) {
                    log::debug!("Cached image file size: {} bytes", metadata.len());
                } else {
                    log::warn!("Could not get metadata for cached image: {:?}", cached_image_path);
                }
                let msg_clone_for_spawn = msg.clone(); // Clone msg for the spawned task
                let app_state_arc_clone = app_state_arc.clone();
                tokio::spawn(async move {
                    log::info!("Spawning task to generate image preview for {:?}", cached_image_path);
                    log::debug!("Chafa command: chafa --size=x7 -f symbols {:?}", cached_image_path);
                    let chafa_command = tokio::process::Command::new("chafa")
                        .arg("--size=x7")
                        .arg("-f")
                        .arg("symbols")
                        .arg(&cached_image_path)
                        .output()
                        .await;

                    match chafa_command {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            log::debug!("Chafa command stdout: {}
Chafa command stderr: {}", stdout, stderr);

                            if output.status.success() {
                                log::info!("Successfully generated image preview.");
                                let preview = stdout.to_string();
                                log::debug!("Chafa generated preview (length: {}):
{}", preview.len(), preview);
                                let mut msg_to_update = msg_clone_for_spawn;
                                msg_to_update.image_preview = Some(preview);
                                let mut state_guard = app_state_arc_clone.lock().await; // Acquire lock here
                                log::debug!("Acquired app_state lock to update message with image preview.");
                                state_guard.update_message(msg_to_update); // Pass the modified message
                                log::debug!("Message with image_preview updated in state.");
                            } else {
                                log::error!("Chafa command failed with status: {}. Stderr: {}", output.status, stderr);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to execute chafa command: {}", e);
                        }
                    }
                });
            } else {
                log::warn!("Cached image path does not exist after download attempt: {:?}. Cannot generate Chafa preview.", cached_image_path);
            }
        }
    } else {
        log::debug!("Message is not an image, skipping chafa processing.");
    }
}

pub async fn handle_show_image_command(
    app_state_arc: Arc<tokio::sync::Mutex<AppState>>,
    file_path: PathBuf,
) {
    log::debug!("handle_show_image_command called for file: {:?}", file_path);

    if !file_path.exists() || !file_path.is_file() {
        log::error!("Image file not found or is not a file: {:?}", file_path);
        let mut state_guard = app_state_arc.lock().await;
        state_guard.set_notification(
            "Image Display Error".to_string(),
            format!("File not found or is not a file: {:?}", file_path),
            NotificationType::Error,
        );
        return;
    }

    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let file_extension = file_path.extension().unwrap_or_default().to_string_lossy().to_string();
    log::info!("Processing local image: {}, extension: {}", file_name, file_extension);

    let is_image = match file_extension.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => true,
        _ => {
            log::warn!("Unsupported image format: {}", file_extension);
            let mut state_guard = app_state_arc.lock().await;
            state_guard.set_notification(
                "Image Display Error".to_string(),
                format!("Unsupported image format: {}", file_extension),
                NotificationType::Error,
            );
            return;
        }
    };

    if is_image {
        log::debug!("Local file identified as image. Proceeding with chafa.");
        let app_state_arc_clone = app_state_arc.clone();
        tokio::spawn(async move {
            log::info!("Spawning task to generate preview for local image: {:?}", file_path);
            log::debug!("Attempting to generate chafa preview for local image: {:?}", file_path);
            log::debug!("Chafa command: chafa --size=x7 -f symbols {:?}", file_path);
            let chafa_command = tokio::process::Command::new("chafa")
                .arg("--size=x7")
                .arg("-f")
                .arg("symbols")
                .arg(&file_path)
                .output()
                .await;

            match chafa_command {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::debug!("Chafa stdout: {}
Chafa stderr: {}", stdout, stderr);

                    if output.status.success() {
                        log::debug!("Chafa command succeeded for local image.");
                        let preview = stdout.to_string();
                        log::debug!("Chafa preview generated for local image. Length: {}
{}", preview.len(), preview);

                        let mut state_guard = app_state_arc_clone.lock().await;
                        log::debug!("Acquired app_state lock to add local image message.");
                        if let Some(current_channel) = state_guard.current_channel.clone() {
                            let current_timestamp = Utc::now().timestamp();
                            let local_image_message = BroadcastMessage {
                                user: state_guard.username.clone().unwrap_or_default(),
                                icon: state_guard.user_icon.clone().unwrap_or_default(),
                                content: format!("Local image: {}", file_name),
                                timestamp: current_timestamp,
                                channel_id: current_channel.id.clone(),
                                message_type: "file".to_string(),
                                file_name: Some(file_name.clone()),
                                file_extension: Some(file_extension.clone()),
                                file_icon: Some("🖼️".to_string()), // Generic image icon
                                file_size_mb: None,
                                is_image: Some(true),
                                image_preview: Some(preview),
                                file_id: None, // No file_id for local images
                                download_url: None, // No download_url for local images
                                download_progress: Some(100),
                            };
                            log::info!("Adding local image as a message to channel '{}'", current_channel.name);
                            state_guard.add_message(local_image_message);
                        } else {
                            log::error!("No channel selected to display image.");
                            state_guard.set_notification(
                                "Image Display Error".to_string(),
                                "No channel selected to display image.".to_string(),
                                NotificationType::Error,
                            );
                        }
                    } else {
                        log::error!("Failed to generate image preview for local file. Stderr: {}", stderr);
                        let mut state_guard = app_state_arc_clone.lock().await;
                        state_guard.set_notification(
                            "Image Display Error".to_string(),
                            format!("Failed to generate image preview: {}", stderr),
                            NotificationType::Error,
                        );
                    }
                }
                Err(e) => {
                    log::error!("Failed to execute chafa command for local image: {}", e);
                    let mut state_guard = app_state_arc_clone.lock().await;
                    state_guard.set_notification(
                        "Image Display Error".to_string(),
                        format!("Failed to execute chafa command: {}", e),
                        NotificationType::Error,
                    );
                }
            }
        });
    }
}