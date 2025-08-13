use crate::app::app_state::AppState;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use image::ImageFormat;
use image::ImageReader;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use image::EncodableLayout;

use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct GifAnimationState {
    pub message_id: String,
    pub frames: Vec<Arc<String>>,              // funny
    pub delays: Vec<u16>,                      // Per-frame delay in ms
    pub current_frame: usize,                  // Current frame index
    pub last_frame_time: Option<Instant>,      // Last time frame was updated
    pub running: bool,                         // Is animation running?
    pub thread_handle: Option<JoinHandle<()>>, // Handle to animation thread
    pub last_redraw_sent_time: Option<Instant>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl GifAnimationState {
    pub fn new(
        message_id: String,
        frames: Vec<Arc<String>>,
        delays: Vec<u16>,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        GifAnimationState {
            message_id,
            frames,
            delays,
            current_frame: 0,
            last_frame_time: None,
            running: true,
            thread_handle: None,
            last_redraw_sent_time: None,
            cancellation_token,
        }
    }
}

pub fn is_gif(data: &[u8]) -> bool {
    let is_gif_format = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map(|reader| reader.format() == Some(ImageFormat::Gif))
        .unwrap_or(false);

    is_gif_format
}

pub async fn convert_gif_to_chafa_frames_and_delays(
    gif_data: &[u8],
    chat_width: u16,
) -> Result<(Vec<Arc<String>>, Vec<u16>), String> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;

    let decoder =
        GifDecoder::new(Cursor::new(gif_data)).map_err(|e| format!("GIF decode error: {}", e))?;
    let frames_iter = decoder.into_frames();
    let all_frames: Vec<image::Frame> = frames_iter
        .map(|f| f.map_err(|e| format!("GIF frame error: {}", e)))
        .collect::<Result<Vec<image::Frame>, String>>()?;

    let mut frames = Vec::new();
    let mut delays = Vec::new();

    let (width, height) = if let Some(frame) = all_frames.first() {
        (frame.buffer().width(), frame.buffer().height())
    } else {
        return Ok((Vec::new(), Vec::new()));
    };

    let max_display_width = chat_width.saturating_sub(4); // Usable width, similar to static images
    let max_display_height = 50; // Max lines for GIF preview

    // Calculate scaling factors for both width and height constraints
    let width_scale_factor = max_display_width as f32 / width as f32;
    let height_scale_factor = max_display_height as f32 / height as f32;

    // Choose the smaller scale factor to ensure the image fits within both dimensions
    let scale_factor = width_scale_factor.min(height_scale_factor);

    let final_width = (width as f32 * scale_factor).round() as u16;
    let mut final_height = (height as f32 * scale_factor).round() as u16;

    // Adjust height for terminal character aspect ratio (approx. 2:1 height:width)
    // This means for every 1 unit of width, we need 2 units of height to maintain visual aspect ratio.
    // So, if we have a target width, the actual height in characters should be roughly half of the image's aspect-corrected height.
    final_height = (final_height as f32 * 0.5).round() as u16;

    // Ensure minimum dimensions if image is too small, or if scaling results in 0
    let final_width = final_width.max(1);
    let final_height = final_height.max(1);

    let size = format!("{}x{}", final_width, final_height);

    for frame in all_frames {
        let mut png_data = Vec::new();
        let png_encoder = PngEncoder::new(&mut png_data);
        png_encoder
            .write_image(
                frame.buffer().as_bytes(),
                frame.buffer().width(),
                frame.buffer().height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("PNG encode error: {}", e))?;

        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = if denom == 0 {
            numer as u16
        } else {
            (numer / denom) as u16
        };
        delays.push(delay);

        let ansi = super::image_handler::run_chafa(&png_data, &size).await?;
        if ansi.is_empty() {
            // If chafa produces empty output, treat it as an error for this frame.
            // This frame will be skipped, and the previous frame will persist.
            // This might cause a slight jump in animation, but prevents flickering to blank.
            continue;
        }
        frames.push(Arc::new(ansi));
    }

    Ok((frames, delays))
}

#[allow(unused_mut)]
pub async fn spawn_gif_animation(
    app_state: Arc<tokio::sync::Mutex<AppState>>,
    animation_state: Arc<tokio::sync::Mutex<GifAnimationState>>,
    redraw_tx: mpsc::UnboundedSender<String>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (frame_content, delay, message_id, next_frame_index) = {
                let mut state = animation_state.lock().await;
                if !state.running || state.frames.is_empty() {
                    break;
                }
                let frame_content = state.frames[state.current_frame].clone();
                let delay = state
                    .delays
                    .get(state.current_frame)
                    .copied()
                    .unwrap_or(100);
                
                let message_id = state.message_id.clone();
                let next_frame_index = (state.current_frame + 1) % state.frames.len();
                state.current_frame = next_frame_index;
                state.last_frame_time = Some(Instant::now());
                
                (frame_content, delay, message_id, next_frame_index)
            };

            // Update the message in AppState with the new frame
            let mut app = app_state.lock().await;
            let mut channel_id_to_redraw: Option<String> = None;
            if let Some(msg) = app.find_message_mut(&message_id) {
                msg.image_preview = Some((*frame_content).clone());
                channel_id_to_redraw = Some(msg.channel_id.clone());
            }
            drop(app);

            if let Some(channel_id) = channel_id_to_redraw {
                let mut app = app_state.lock().await;
                app.needs_re_render
                    .entry(channel_id.clone())
                    .or_default()
                    .insert(message_id.clone(), true);
                let _ = redraw_tx.send(channel_id.clone()); // Signal redraw for specific channel
                drop(app);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(delay as u64)).await;
        }
    })
}
