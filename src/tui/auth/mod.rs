pub mod events;
pub mod page;
pub mod state;

use crate::api::auth_api;
use crate::app::app_state::AppState;
use crate::tui::auth::events::handle_auth_event;
use crate::tui::auth::page::{draw_auth_ui, get_validation_error, ICONS};
use crate::tui::auth::state::{AuthMode, AuthState, SelectedField};
use crate::TuiPage;
use crossterm::event;

use ratatui::Terminal;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::{io, time::Duration};
use tokio::time::sleep;

pub async fn run_auth_page<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut auth_state = AuthState::new();
    let mut settings_state = {
        let state = app_state.lock().await;
        crate::tui::settings::state::SettingsState::new(
            state.themes.keys().cloned().collect(),
            state.current_theme.name.clone(),
            state.username.as_deref().unwrap_or(""),
            state.user_icon.as_deref().unwrap_or(""),
            state.settings_main_selection,
            state.settings_focused_pane,
        )
    };
    let client = reqwest::Client::new();
    terminal.show_cursor()?;

    loop {
        let mut app_state_guard = app_state.lock().await;

        terminal.draw(|f| {
            let msg_to_draw = auth_state.message_state.try_lock().map(|g| g.clone()).unwrap_or_default();
            draw_auth_ui::<B>(
                f,
                &auth_state.username_input,
                &auth_state.password_input,
                auth_state.selected_icon_index,
                &auth_state.current_mode,
                &auth_state.selected_field,
                &msg_to_draw,
                &app_state_guard.current_theme,
                &app_state_guard,
                &mut settings_state,
            );
        })?;

        let event = tokio::select! {
            event_result = tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(16))) => {
                match event_result {
                    Ok(Ok(true)) => Some(tokio::task::spawn_blocking(event::read).await.unwrap().unwrap()),
                    _ => None,
                }
            }
        };

        if let Some(event) = event {
            if app_state_guard.show_settings {
                if let event::Event::Key(key) = &event {
                    if key.code == event::KeyCode::Esc {
                        app_state_guard.show_settings = false;
                        continue;
                    }
                }
                if let Some(target_page) = crate::tui::settings::handle_settings_key_event(crate::tui::settings::SettingsEvent::Key(event.clone()), &mut app_state_guard, &mut settings_state) {
                    if target_page == TuiPage::Auth {
                        app_state_guard.show_settings = false;
                    } else if target_page == TuiPage::Exit {
                        return Ok(TuiPage::Exit);
                    } else {
                        return Ok(target_page);
                    }
                }
            } else {
                if let event::Event::Key(key) = event {
                    if key.code == event::KeyCode::Char('s') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                        app_state_guard.show_settings = true;
                        continue;
                    }
                }

                let event_result = handle_auth_event(
                    event,
                    &mut auth_state.username_input,
                    &mut auth_state.password_input,
                    auth_state.selected_icon_index,
                    auth_state.current_mode,
                    auth_state.selected_field,
                    None,
                    app_state.clone(),
                )?;

                auth_state.selected_icon_index = event_result.selected_icon_index;
                auth_state.current_mode = event_result.current_mode;
                auth_state.selected_field = event_result.selected_field;
                auth_state.update_focus();
                if let Some(msg) = event_result.message {
                    *auth_state.message_state.lock().await = msg;
                }

                if let Some(page) = event_result.next_page {
                    return Ok(page);
                }

                if event_result.should_submit {
                    match auth_state.selected_field {
                        SelectedField::RegisterButton => {
                            if auth_state.current_mode == AuthMode::Register {
                                let validation_error = get_validation_error(
                                    &auth_state.username_input,
                                    &auth_state.password_input,
                                    &auth_state.current_mode,
                                );
                                if let Some(err_msg) = validation_error {
                                    *auth_state.message_state.lock().await = err_msg;
                                    continue;
                                }

                                let hashed_password = format!(
                                    "{:x}",
                                    Sha256::digest(auth_state.password_input.text.as_bytes())
                                );
                                *auth_state.message_state.lock().await = "Registering...".to_string();

                                match auth_api::register(
                                    &client,
                                    &auth_state.username_input.text,
                                    &hashed_password,
                                    ICONS[auth_state.selected_icon_index],
                                )
                                .await
                                {
                                    Ok(token_response) => {
                                        app_state_guard.set_user_auth(
                                            token_response.token,
                                            auth_state.username_input.text.clone(),
                                            token_response.icon,
                                        );
                                        return Ok(TuiPage::Chat);
                                    }
                                    Err(e) => {
                                        *auth_state.message_state.lock().await = e.to_string();
                                        let msg_clone = auth_state.message_state.clone();
                                        tokio::spawn(async move {
                                            sleep(Duration::from_secs(3)).await;
                                            *msg_clone.lock().await = String::new();
                                        });
                                    }
                                }
                            }
                        }
                        SelectedField::LoginButton => {
                            if auth_state.current_mode == AuthMode::Login {
                                let validation_error = get_validation_error(
                                    &auth_state.username_input,
                                    &auth_state.password_input,
                                    &auth_state.current_mode,
                                );
                                if let Some(err_msg) = validation_error {
                                    *auth_state.message_state.lock().await = err_msg;
                                    continue;
                                }

                                *auth_state.message_state.lock().await = "Logging in...".to_string();

                                let hashed_password = format!(
                                    "{:x}",
                                    Sha256::digest(auth_state.password_input.text.as_bytes())
                                );
                                match auth_api::login(
                                    &client,
                                    &auth_state.username_input.text,
                                    &hashed_password,
                                )
                                .await
                                {
                                    Ok(token_response) => {
                                        app_state_guard.set_user_auth(
                                            token_response.token,
                                            auth_state.username_input.text.clone(),
                                            token_response.icon,
                                        );
                                        return Ok(TuiPage::Chat);
                                    }
                                    Err(e) => {
                                        *auth_state.message_state.lock().await = e.to_string();
                                        let msg_clone = auth_state.message_state.clone();
                                        tokio::spawn(async move {
                                            sleep(Duration::from_secs(3)).await;
                                            *msg_clone.lock().await = String::new();
                                        });
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}