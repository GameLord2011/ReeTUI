pub mod events;
pub mod helpers;
pub mod page;
pub mod state;

use crate::app::app_state::AppState;
use crate::app::TuiPage;
use crate::tui::settings::events::handle_settings_event;
use crate::tui::settings::page::draw_settings_ui;
use crate::tui::settings::state::SettingsState;
use crate::tui::notification::ui::draw_notifications;
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::Backend;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::Block;
use ratatui::Frame;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub enum SettingsEvent {
    Key(Event),
    Tick,
}

#[derive(Debug)]
pub enum SettingsCommand {
    UpdateState(SettingsState),
    ChangePage(TuiPage),
    None,
}

pub async fn run_settings_page<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<Option<TuiPage>> {
    let (settings_tx, mut settings_rx) = mpsc::unbounded_channel::<SettingsEvent>();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<SettingsCommand>();

    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        let mut settings_state = {
            let state = app_state_clone.lock().await;
            SettingsState::new(
                state.themes.keys().cloned().collect(),
                state.current_theme.name.clone(),
                state.username.as_deref().unwrap_or(""),
                state.user_icon.as_deref().unwrap_or(""),
                state.settings_main_selection,
                state.settings_focused_pane,
            )
        };

        while let Some(event) = settings_rx.recv().await {
            let mut app_state_locked = app_state_clone.lock().await;
            let result_page =
                handle_settings_key_event(event, &mut app_state_locked, &mut settings_state);

            command_tx
                .send(SettingsCommand::UpdateState(settings_state.clone()))
                .unwrap();

            if let Some(page) = result_page {
                command_tx.send(SettingsCommand::ChangePage(page)).unwrap();
                break;
            }
        }
    });

    let mut current_settings_state = {
        let state = app_state.lock().await;
        SettingsState::new(
            state.themes.keys().cloned().collect(),
            state.current_theme.name.clone(),
            state.username.as_deref().unwrap_or(""),
            state.user_icon.as_deref().unwrap_or(""),
            state.settings_main_selection,
            state.settings_focused_pane,
        )
    };

    loop {
        let mut app_state_locked = app_state.lock().await;
        app_state_locked.notification_manager.update();
        terminal.draw(|f| {
            render_settings_popup::<B>(f, &app_state_locked, &mut current_settings_state, f.area())
                .unwrap();
        })?;
        drop(app_state_locked);

        let event_poll_result = tokio::select! {
            cmd = command_rx.recv() => {
                if let Some(command) = cmd {
                    match command {
                        SettingsCommand::UpdateState(new_state) => {
                            current_settings_state = new_state;
                        },
                        SettingsCommand::ChangePage(page) => {
                            return Ok(Some(page));
                        },
                        SettingsCommand::None => {}
                    }
                }
                None
            },
            event_result = tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(50))) => {
                match event_result {
                    Ok(Ok(true)) => Some(event::read()?),
                    _ => None,
                }
            }
        };

        if let Some(event) = event_poll_result {
            settings_tx.send(SettingsEvent::Key(event)).unwrap();
        } else {
            settings_tx.send(SettingsEvent::Tick).unwrap();
        }
    }
}

pub fn render_settings_popup<B: Backend>(
    frame: &mut Frame<'_>,
    app_state: &AppState,
    settings_state: &mut SettingsState,
    area: Rect,
) -> io::Result<()> {
    let theme = &app_state.current_theme;

    frame.render_widget(
        Block::default().bg(crate::themes::rgb_to_color(&theme.colors.background)),
        area,
    );
    draw_settings_ui::<B>(frame, settings_state, theme, app_state, area);

    draw_notifications(frame, app_state);
    Ok(())
}

pub fn handle_settings_key_event(
    event: SettingsEvent,
    app_state: &mut AppState,
    settings_state: &mut SettingsState,
) -> Option<TuiPage> {
    match event {
        SettingsEvent::Key(key_event) => {
            if let Some(page) = handle_settings_event(
                settings_state,
                app_state,
                SettingsEvent::Key(key_event.clone()),
            ) {
                app_state.settings_main_selection = settings_state.main_selection;
                app_state.settings_focused_pane = settings_state.focused_pane;
                return Some(page);
            }
            if let Event::Key(key) = key_event {
                if key.code == KeyCode::Esc {
                    return Some(TuiPage::Chat);
                }
            }
        }
        SettingsEvent::Tick => {}
    }
    None
}
