use crate::app::app_state::AppState;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Instant;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use image::ImageFormat;
use image::ImageReader;
use tokio::sync::mpsc;
use log::{info};
use image::EncodableLayout;

use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct GifAnimationState {
    pub message_id: String,
    pub frames: Vec<Arc<String>>,      // funny
    pub delays: Vec<u16>,              // Per-frame delay in ms
    pub current_frame: usize,          // Current frame index
    pub last_frame_time: Option<Instant>, // Last time frame was updated
    pub running: bool,                 // Is animation running?
    pub thread_handle: Option<JoinHandle<()>>, // Handle to animation thread
    pub last_redraw_sent_time: Option<Instant>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl GifAnimationState {
    pub fn new(message_id: String, frames: Vec<Arc<String>>, delays: Vec<u16>, cancellation_token: tokio_util::sync::CancellationToken) -> Self {
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
    log::debug!("gif_renderer: is_gif called. Result: {}", is_gif_format);
    is_gif_format
}

use futures::future::join_all;

pub async fn convert_gif_to_chafa_frames_and_delays(
    gif_data: &[u8],
    height: u16,
) -> Result<(Vec<Arc<String>>, Vec<u16>), String> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    info!("Handling GIF to Chafa frames and delays conversion.");
    let decoder = GifDecoder::new(Cursor::new(gif_data))
        .map_err(|e| format!("GIF decode error: {}", e))?;
    let frames_iter = decoder.into_frames();
    let all_frames: Vec<image::Frame> = frames_iter
        .map(|f| f.map_err(|e| format!("GIF frame error: {}", e)))
        .collect::<Result<Vec<image::Frame>, String>>()?;

    let mut chafa_futures = Vec::new();
    let mut delays = Vec::new();

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
        let delay = if denom == 0 { numer as u16 } else { (numer / denom) as u16 };
        delays.push(delay);

        // Spawn a tokio task for each chafa conversion
        chafa_futures.push(tokio::spawn(async move {
            super::image_handler::run_chafa(&png_data, height).await
        }));
    }

    let results = join_all(chafa_futures).await;

    let mut frames = Vec::new();
    for res in results {
        match res {
            Ok(Ok(ansi)) => frames.push(Arc::new(ansi)),
            Ok(Err(e)) => return Err(format!("Chafa conversion error: {}", e)),
            Err(e) => return Err(format!("Tokio task join error: {}", e)),
        }
    }

    info!("Extracted {} frames and delays from GIF.", frames.len());
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
            let mut state = animation_state.lock().await;
            if !state.running || state.frames.is_empty() {
                break;
            }
            let frame = state.frames[state.current_frame].clone();
            let delay = state.delays.get(state.current_frame).copied().unwrap_or(100);
            let current_frame_index = state.current_frame;
            let delay_value = delay;
            let message_id = state.message_id.clone();

            // Debounce redraw signals
            const REDRAW_DEBOUNCE_MS: u128 = 100; // Redraw at most every 100ms
            let now = Instant::now();
            let should_redraw = state.last_redraw_sent_time.is_none()
                || now.duration_since(state.last_redraw_sent_time.unwrap()).as_millis()
                    >= REDRAW_DEBOUNCE_MS;

            if should_redraw {
                // Update the message in AppState with the new frame
                let mut app = app_state.lock().await;
                let mut channel_id_to_redraw: Option<String> = None;
                if let Some(msg) = app.find_message_mut(&message_id) {
                    msg.image_preview = Some((*frame).clone());
                    channel_id_to_redraw = Some(msg.channel_id.clone());
                }
                drop(app);

                if let Some(channel_id) = channel_id_to_redraw {
                    let mut app = app_state.lock().await;
                    app.needs_re_render.entry(channel_id.clone()).or_default().insert(message_id.clone(), true);
                    let _ = redraw_tx.send(channel_id.clone()); // Signal redraw for specific channel
                    drop(app);
                }
                state.last_redraw_sent_time = Some(now);
            }

            state.current_frame = (state.current_frame + 1) % state.frames.len();
            state.last_frame_time = Some(Instant::now());
            drop(state);

            log::debug!("gif_renderer: Sent frame {} to UI. Delay: {}ms", current_frame_index, delay_value);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay as u64)).await;
        }
    })
}
