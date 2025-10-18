use crate::api::models::BroadcastMessage;
use crate::app::app_state::{AppState};
use image::ImageReader;
use image::{GenericImageView, ImageFormat};

use std::io::{self};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

/// Helper to update message and log debug info
async fn update_and_log_message(
    app_state: &Arc<Mutex<AppState>>,
    channel_id: String,
    message_id: String,
    update_fn: impl FnOnce(&mut BroadcastMessage),
    _context: &str,
) {
    let mut state = app_state.lock().await;
    if let Some(message) = state.find_message_mut(&message_id) {
        update_fn(message);
        state
            .needs_re_render
            .entry(channel_id)
            .or_default()
            .insert(message_id, true);
    }
}

/// A robust, non-blocking function to execute the chafa command.
pub async fn run_chafa(image_data: &[u8], size: &str) -> Result<String, String> {
    let size_arg = format!("--size={}", size);
    let args = [size_arg.as_str(), "-f", "symbols", "--symbols", "all"];
    let _command_str = format!("chafa {}", args.join(" "));

    let mut command = Command::new("chafa");
    command.args(&args);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn().map_err(|e| {
        format!(
            "Failed to run chafa. Is it installed and in your PATH? Details: {}",
            e
        )
    })?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let data = image_data.to_vec();
    tokio::spawn(async move {
        stdin
            .write_all(&data)
            .await
            .expect(" Failed to write to stdin");
    });

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!(" Failed to wait for chafa command: {}", e))?;

    if output.status.success() {
        let mut chafa_string = String::from_utf8_lossy(&output.stdout).to_string();

        #[cfg(windows)]
        {
            chafa_string = chafa_string.replace("\n", "\r\n");
        }
        Ok(chafa_string)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            " Chafa conversion failed. Stderr: {}",
            if stderr.is_empty() {
                r"¯\_(ツ)_/¯ No error message from chafa. Check logs for more details."
            } else {
                &stderr
            }
        ))
    }
}

pub async fn convert_image_to_chafa(image_data: &[u8], chat_width: u16) -> Result<String, String> {
    let image = image::load_from_memory(image_data).map_err(|e| e.to_string())?;
    let (original_width, original_height) = image.dimensions();
    let max_display_width = chat_width.saturating_sub(4);
    let max_display_height = 50;
    const MIN_DISPLAY_HEIGHT: u16 = 10;
    let width_scale_factor = max_display_width as f32 / original_width as f32; // it is ez
    let height_scale_factor = max_display_height as f32 / original_height as f32; // right ?
    let scale_factor = width_scale_factor.min(height_scale_factor);
    let mut final_width = (original_width as f32 * scale_factor).round() as u16;
    let mut final_height = (original_height as f32 * scale_factor).round() as u16;
    if final_height < MIN_DISPLAY_HEIGHT {
        final_height = MIN_DISPLAY_HEIGHT;
        final_width =
            (original_width as f32 * (final_height as f32 / original_height as f32)).round() as u16;
    }
    if final_width > max_display_width {
        final_width = max_display_width;
        final_height =
            (original_height as f32 * (final_width as f32 / original_width as f32)).round() as u16;
    }
    let final_width = final_width.max(1);
    let final_height = final_height.max(1);
    let size = format!("{}x{}", final_width, final_height);
    run_chafa(image_data, &size).await
}

#[allow(dead_code)]
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
    chat_width: u16,
    redraw_tx: mpsc::UnboundedSender<String>, // Add redraw_tx here
) {
    let file_id = message.file_id.clone().unwrap_or_default();
    let file_name = message.file_name.clone().unwrap_or_default();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move { while let Some(_) = progress_rx.recv().await {} });

    match crate::api::file_api::download_file(http_client, &file_id, &file_name, progress_tx, false)
        .await
    {
        Ok(file_path) => {
            let mut file = match File::open(&file_path).await {
                Ok(f) => f,
                Err(e) => {
                    let msg_id = message.client_id.clone().unwrap_or_default();
                    let ch_id = message.channel_id.clone();
                    update_and_log_message(
                        &app_state,
                        ch_id,
                        msg_id,
                        |msg| {
                            msg.content = format!("[Error opening downloaded file: {}]", e);
                        },
                        "download_open_error",
                    )
                    .await;
                    return;
                }
            };
            let mut image_data = Vec::new();
            if let Err(e) = file.read_to_end(&mut image_data).await {
                let msg_id = message.client_id.clone().unwrap_or_default();
                let ch_id = message.channel_id.clone();
                update_and_log_message(
                    &app_state,
                    ch_id,
                    msg_id,
                    |msg| {
                        msg.content = format!("[Error reading downloaded file: {}]", e);
                    },
                    "download_read_error",
                )
                .await;
                return;
            }

            if crate::tui::chat::gif_renderer::is_gif(&image_data) {
                match crate::tui::chat::gif_renderer::convert_gif_to_chafa_frames_and_delays(
                    &image_data,
                    chat_width,
                )
                .await
                {
                    Ok((frames, delays)) if frames.len() > 1 => {
                        message.content = (*frames[0]).clone();
                        let frames_with_delays: Vec<(String, std::time::Duration)> = frames
                            .iter()
                            .map(|arc| (**arc).clone())
                            .zip(
                                delays
                                    .iter()
                                    .map(|d| std::time::Duration::from_millis(*d as u64)),
                            )
                            .collect();
                        message.gif_frames = Some(frames_with_delays);

                        let animation_state =
                            crate::tui::chat::gif_renderer::GifAnimationState::new(
                                message.client_id.clone().unwrap_or_default(),
                                frames.clone(),
                                delays.clone(),
                                tokio_util::sync::CancellationToken::new(),
                            );
                        let animation_state_arc =
                            Arc::new(tokio::sync::Mutex::new(animation_state));
                        let thread_handle = crate::tui::chat::gif_renderer::spawn_gif_animation(
                            app_state.clone(),
                            animation_state_arc.clone(),
                            redraw_tx.clone(), // Pass redraw_tx here
                        )
                        .await;
                        {
                            let mut state = animation_state_arc.lock().await;
                            state.thread_handle = Some(thread_handle);
                        }
                        {
                            let mut state = app_state.lock().await;
                            state
                                .active_animations
                                .insert(file_id.clone(), animation_state_arc.clone());
                        }
                        // Optionally: handle frame_rx to update UI with new frames
                        let msg_id = message.client_id.clone().unwrap_or_default();
                        let ch_id = message.channel_id.clone();
                        update_and_log_message(
                            &app_state,
                            ch_id,
                            msg_id,
                            |msg| {
                                msg.content = message.content.clone(); // Preserve the content set earlier
                                msg.gif_frames = message.gif_frames.clone(); // Preserve gif_frames
                                msg.image_preview = message.image_preview.clone();
                                // Preserve image_preview
                            },
                            "gif_ok",
                        )
                        .await;
                    }
                    Ok((frames, _delays)) if !frames.is_empty() => {
                        let msg_id = message.client_id.clone().unwrap_or_default();
                        let ch_id = message.channel_id.clone();
                        update_and_log_message(
                            &app_state,
                            ch_id,
                            msg_id,
                            |msg| {
                                msg.content = (*frames[0]).clone();
                                msg.gif_frames = None;
                            },
                            "gif_static",
                        )
                        .await;
                    }
                    Err(_e) => {
                        // If there's an error converting the GIF, do nothing.
                        // The original message content will be displayed.
                    }
                    Ok((frames, _delays)) if frames.is_empty() => {
                        // :3
                        // If no frames, do nothing to message.content or image_preview.
                        // The original message content will be displayed.
                        // We do NOT call update_and_log_message here, as it would
                        // potentially clear a previously set image_preview or
                        // mark the message for re-render without a valid frame.
                        // The message will remain as it was before processing the GIF.
                        // This ensures that if a GIF fails to load, the previous
                        // display state for that message is preserved.
                    }
                    _ => {
                        // If there's an unknown issue, do nothing.
                        // The original message content will be displayed.
                    }
                }
            } else {
                // Handle static images
                match convert_image_to_chafa(&image_data, chat_width).await {
                    Ok(chafa_string) => {
                        let msg_id = message.client_id.clone().unwrap_or_default();
                        let ch_id = message.channel_id.clone();
                        update_and_log_message(
                            &app_state,
                            ch_id,
                            msg_id,
                            |msg| {
                                msg.content = chafa_string.clone();
                                msg.image_preview = Some(chafa_string);
                            },
                            "static_ok",
                        )
                        .await;
                    }
                    Err(_e) => {
                        // If there's an error converting the static image, do nothing.
                        // The original message content will be displayed.
                    }
                }
            }
        }
        Err(e) => {
            // Handle error in downloading file
            let msg_id = message.client_id.clone().unwrap_or_default();
            let ch_id = message.channel_id.clone();
            update_and_log_message(
                &app_state,
                ch_id,
                msg_id,
                |msg| {
                    msg.content = format!("[Error downloading file: {}]", e);
                },
                "download_error",
            )
            .await;
        }
    }
}
