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

use std::thread::JoinHandle;

#[derive(Debug)
]
pub struct GifAnimationState {
    pub frames: Vec<Arc<String>>,      // funny
    pub delays: Vec<u16>,              // Per-frame delay in ms
    pub current_frame: usize,          // Current frame index
    pub last_frame_time: Option<Instant>, // Last time frame was updated
    pub running: bool,                 // Is animation running?
    pub thread_handle: Option<JoinHandle<()>>, // Handle to animation thread
}

impl GifAnimationState {
    pub fn new(frames: Vec<Arc<String>>, delays: Vec<u16>) -> Self {
        GifAnimationState {
            frames,
            delays,
            current_frame: 0,
            last_frame_time: None,
            running: true,
            thread_handle: None,
        }
    }
}

pub fn is_gif(data: &[u8]) -> bool {
    ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map(|reader| reader.format() == Some(ImageFormat::Gif))
        .unwrap_or(false)
}

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

    let mut frames = Vec::new();
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

        let ansi = super::image_handler::run_chafa(&png_data, height).await?;
        frames.push(Arc::new(ansi));
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = if denom == 0 { numer as u16 } else { (numer / denom) as u16 }; // Get delay in ms from image::Delay
        delays.push(delay);
    }
    info!("Extracted {} frames and delays from GIF.", frames.len());
    Ok((frames, delays))
}

// Spawns a thread for GIF animation, sending frame updates via channel
pub fn spawn_gif_animation(
    animation_state: Arc<std::sync::Mutex<GifAnimationState>>,
    frame_tx: mpsc::UnboundedSender<String>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let state = animation_state.lock().unwrap();
            if !state.running || state.frames.is_empty() {
                break;
            }
            let frame = state.frames[state.current_frame].clone();
            let delay = state.delays.get(state.current_frame).copied().unwrap_or(100);
            drop(state);
            let _ = frame_tx.send((*frame).clone());
            std::thread::sleep(std::time::Duration::from_millis(delay as u64));
            let mut state = animation_state.lock().unwrap();
            state.current_frame = (state.current_frame + 1) % state.frames.len();
            state.last_frame_time = Some(Instant::now());
        }
    })
}
