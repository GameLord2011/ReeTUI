use crate::api::models::BroadcastMessage;
use crate::app::{AppState, GifAnimationState};
use image::ImageFormat;
use image::ImageReader;
use log::{error, info};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

/// Helper to update message and log debug info
async fn update_and_log_message(
    app_state: &Arc<Mutex<AppState>>,
    message: BroadcastMessage,
    context: &str,
) {
    let mut state = app_state.lock().await;
    log::debug!(
        "image_handler: [{}] Updating message: file_id={:?} timestamp={:?} file_name={:?} is_gif={} gif_frames={} image_preview={}",
        context,
        message.file_id,
        message.timestamp,
        message.file_name,
        message.gif_frames.is_some(),
        message.gif_frames.as_ref().map(|f| f.len()).unwrap_or(0),
        message.image_preview.is_some()
    );
    state.update_message(message);
}

/// Asynchronously writes data to a new temporary file.
async fn create_temp_file_with_data(data: &[u8]) -> Result<NamedTempFile, String> {
    let data_vec = data.to_vec();
    tokio::task::spawn_blocking(move || {
        let mut temp_file =
            NamedTempFile::new().map_err(|e| format!("Failed to create temporary file: {}", e))?;
        temp_file
            .write_all(&data_vec)
            .map_err(|e| format!("Failed to write data to temporary file: {}", e))?;
        Ok(temp_file)
    })
    .await
    .map_err(|e| format!("Task join error during temp file creation: {}", e))?
}

/// A robust, non-blocking function to execute the chafa command.
async fn run_chafa(input_path: &Path, height: u16) -> Result<String, String> {
    let size_arg = format!("--size=x{}", height);
    let args = [size_arg.as_str(), "-f", "symbols"];
    let command_str = format!("chafa {} {}", args.join(" "), input_path.to_string_lossy());
    info!("Executing command: {}", &command_str);

    let output = Command::new("chafa")
        .args(&args)
        .arg(input_path)
        .output()
        .await
        .map_err(|e| {
            error!(
                "Failed to spawn chafa command. Is 'chafa' installed and in your system's PATH? Error: {}",
                e
            );
            format!(
                "Failed to run chafa. Is it installed and in your PATH? Details: {}",
                e
            )
        })?;

    if output.status.success() {
        info!("Chafa command executed successfully.");
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!(
            "Chafa command failed!\n\
            - Status: {}\n\
            - Command: {}\n\
            - Stderr: {}\n\
            - Stdout: {}",
            output.status, command_str, stderr, stdout
        );
        Err(format!(
            "Chafa conversion failed. Stderr: {}",
            if stderr.is_empty() {
                "No error message from chafa. Check logs for more details."
            } else {
                &stderr
            }
        ))
    }
}

pub async fn convert_gif_to_chafa_frames_and_delays(
    gif_data: &[u8],
    height: u16,
) -> Result<(Vec<String>, Vec<u16>), String> {
    use gif::{Decoder, Frame}; // unused, kept for clarity
    use std::io::Cursor;
    info!("Handling GIF to Chafa frames and delays conversion.");
    let mut decoder =
        gif::Decoder::new(Cursor::new(gif_data)).map_err(|e| format!("GIF decode error: {}", e))?;
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    let mut frame_idx = 0;
    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|e| format!("GIF frame error: {}", e))?
    {
        // Save frame as PNG temp file
        let mut temp_file =
            tempfile::NamedTempFile::new().map_err(|e| format!("Temp file error: {}", e))?;
        let width = frame.width as u32;
        let height_px = frame.height as u32;
        let buffer = &frame.buffer;
        image::save_buffer(
            &mut temp_file,
            buffer,
            width,
            height_px,
            image::ColorType::Rgba8,
        )
        .map_err(|e| format!("Image save error: {}", e))?;
        // Run chafa on this frame with correct height
        let ansi = run_chafa(temp_file.path(), height).await?;
        frames.push(ansi);
        // Delay is in 10ms units, convert to ms, default to 200ms if 0
        let delay = if frame.delay == 0 {
            200
        } else {
            frame.delay as u16 * 10
        };
        delays.push(delay);
        frame_idx += 1;
    }
    info!("Extracted {} frames and delays from GIF.", frames.len());
    Ok((frames, delays))
}

pub async fn convert_image_to_chafa(image_data: &[u8], height: u16) -> Result<String, String> {
    info!("Handling static image to Chafa conversion.");
    let temp_file = create_temp_file_with_data(image_data).await?;
    run_chafa(temp_file.path(), height).await
}

pub fn is_gif(data: &[u8]) -> bool {
    ImageReader::new(io::Cursor::new(data))
        .with_guessed_format()
        .map(|reader| reader.format() == Some(ImageFormat::Gif))
        .unwrap_or(false)
}

/// Processes an image message, converting it and registering for animation if it's a GIF.
pub async fn process_image_message(
    app_state: Arc<Mutex<AppState>>,
    mut message: BroadcastMessage,
    http_client: &reqwest::Client,
    height: u16,
) {
    log::debug!(
        "process_image_message: file_id={:?} timestamp={}",
        message.file_id,
        message.timestamp
    );
    let file_id = message.file_id.clone().unwrap_or_default();
    let file_name = message.file_name.clone().unwrap_or_default();

    match crate::api::file_api::download_file(
        http_client,
        &file_id,
        &file_name,
        mpsc::unbounded_channel().0,
    )
    .await
    {
        Ok(image_data) => {
            if is_gif(&image_data) {
                match convert_gif_to_chafa_frames_and_delays(&image_data, height).await {
                    Ok((frames, delays)) if frames.len() > 1 => {
                        message.content = frames[0].clone();
                        let frames_with_delays: Vec<(String, std::time::Duration)> = frames
                            .iter()
                            .cloned()
                            .zip(
                                delays
                                    .iter()
                                    .map(|d| std::time::Duration::from_millis(*d as u64)),
                            )
                            .collect();
                        message.gif_frames = Some(frames_with_delays);
                        let animation_state = GifAnimationState {
                            frames,
                            delays,
                            current_frame: 0,
                            last_frame_time: None,
                        };
                        {
                            let mut state = app_state.lock().await;
                            state.active_animations.insert(file_id.clone(), animation_state);
                        }
                        update_and_log_message(&app_state, message, "gif_ok").await;
                    }
                    Ok((frames, delays)) if !frames.is_empty() => {
                        message.content = frames[0].clone();
                        let frames_with_delays: Vec<(String, std::time::Duration)> = frames
                            .iter()
                            .cloned()
                            .zip(
                                delays
                                    .iter()
                                    .map(|d| std::time::Duration::from_millis(*d as u64)),
                            )
                            .collect();
                        message.gif_frames = Some(frames_with_delays);
                        update_and_log_message(&app_state, message, "gif_static").await;
                    }
                    Err(e) => {
                        message.content = format!("[Error converting GIF: {}]", e);
                        update_and_log_message(&app_state, message, "gif_error").await;
                    }
                    _ => {
                        message.content = "[Could not display GIF]".to_string();
                        update_and_log_message(&app_state, message, "gif_unknown").await;
                    }
                }
            } else {
                // Handle static images
                match convert_image_to_chafa(&image_data, height).await {
                    Ok(chafa_string) => {
                        message.content = chafa_string.clone();
                        message.image_preview = Some(chafa_string);
                        update_and_log_message(&app_state, message, "static_ok").await;
                    }
                    Err(e) => {
                        message.content = format!("[Error converting image: {}]", e);
                        update_and_log_message(&app_state, message, "static_error").await;
                    }
                }
            }
        }
        Err(e) => {
            // Handle error in downloading file
            message.content = format!("[Error downloading file: {}]", e);
            update_and_log_message(&app_state, message, "download_error").await;
        }
    }
}
