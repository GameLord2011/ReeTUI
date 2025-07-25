// funny
pub mod page;
pub mod events;
pub mod state;

use crate::app::AppState;
use crate::tui::TuiPage;
use crate::tui::home::page::{draw_home_ui, ANIMATION_FRAMES, FRAME_DURATION_MS};
use crate::tui::home::events::handle_home_event;
use crate::tui::home::state::AnimationState;
use crate::tui::themes::{get_theme, ThemeName};
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub async fn run_home_page<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let current_theme = get_theme(ThemeName::CatppuccinMocha);
    let mut animation_state = AnimationState::new();
    let frame_count = ANIMATION_FRAMES.len();
    let frame_duration = Duration::from_millis(FRAME_DURATION_MS);

    loop {
        let _state_guard = app_state.lock().await;

        terminal.draw(|f| {
            draw_home_ui::<B>(f, animation_state.frame_index, &current_theme);
        })?;

        animation_state.update(frame_count, frame_duration);
        let wait_time = Duration::from_millis(0);

        tokio::time::sleep(frame_duration).await;

        if let Some(page) = handle_home_event(wait_time)? {
            return Ok(page);
        }
    }
}
