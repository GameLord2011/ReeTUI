pub mod events;
pub mod page;
pub mod state;

use crate::app::app_state::AppState;
use crate::app::TuiPage;
use crate::tui::home::events::handle_home_event;
use crate::tui::home::page::{draw_home_ui, ANIMATION_FRAMES, FRAME_DURATION_MS};
use crate::tui::home::state::AnimationState;
use crate::tui::notification::notification::NotificationType;
use crate::tui::notification::ui::draw_notifications;
use ratatui::style::Stylize;
use ratatui::widgets::Block;
use ratatui::{backend::Backend, Terminal};
use std::io;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

pub async fn run_home_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<Option<TuiPage>> {
    let mut animation_state = AnimationState::new();
    let _frame_count = ANIMATION_FRAMES.len();
    let frame_duration = Duration::from_millis(FRAME_DURATION_MS);

    app_state
        .lock()
        .await
        .notification_manager
        .add(
            "Welcome to ReeTUI 󱠡".to_string(),
            "Press any key to continue. 󰌏".to_string(),
            NotificationType::Info,
            Some(Duration::from_secs(5)),
            app_state.clone(),
        )
        .await;

    loop {
        app_state.lock().await.notification_manager.update();
        let current_frame_index = animation_state.frame_index;
        let app_state_locked = app_state.lock().await;
        let theme = &app_state_locked.current_theme;
        terminal.draw(|f| {
            f.render_widget(
                Block::default().bg(crate::themes::rgb_to_color(&theme.colors.background)),
                f.area(),
            );
            draw_home_ui::<B>(f, current_frame_index, theme);
            draw_notifications(f, &app_state_locked);
        })?;

        if let Some(page) = handle_home_event(Duration::from_millis(100))? {
            if page == TuiPage::Exit {
                app_state
                    .lock()
                    .await
                    .notification_manager
                    .add(
                        "Exiting Application, REALLY  ?!".to_string(),
                        "Goodbye  ! 󱠡".to_string(),
                        NotificationType::Info,
                        Some(Duration::from_secs(2)),
                        app_state.clone(),
                    )
                    .await;
            }
            sleep(Duration::from_secs(2));
            return Ok(Some(page));
        }

        animation_state.update(ANIMATION_FRAMES.len(), frame_duration);
        tokio::time::sleep(frame_duration).await;
    }
}
