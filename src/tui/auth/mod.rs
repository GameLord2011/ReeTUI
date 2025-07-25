// funny
mod events;
pub mod page;
mod state;

use crate::api::auth_api;
use crate::app::AppState;
use crate::tui::auth::events::handle_auth_event;
use crate::tui::auth::page::{draw_auth_ui, get_validation_error, AuthMode, SelectedField, ICONS};
use crate::tui::auth::state::AuthState;
use crate::tui::themes::ThemeName;
use crate::tui::TuiPage;
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
    let current_theme = ThemeName::CatppuccinMocha;
    let client = reqwest::Client::new();
    loop {
        if auth_state.selected_icon_index >= ICONS.len() {
            auth_state.selected_icon_index = ICONS.len() - 1;
        }

        let msg_to_draw_guard = auth_state.message_state.lock().await;
        let msg_to_draw = msg_to_draw_guard.clone();
        drop(msg_to_draw_guard);

        terminal.draw(|f| {
            draw_auth_ui(
                f,
                &auth_state.username_input,
                &auth_state.password_input,
                auth_state.selected_icon_index,
                &auth_state.current_mode,
                &auth_state.selected_field,
                &msg_to_draw,
                current_theme,
            );

            match auth_state.selected_field {
                SelectedField::Username => {
                    let cursor_x = f.area().x
                        + (f.area().width.saturating_sub(35)) / 2
                        + 1
                        + auth_state.username_input.len() as u16;
                    let cursor_y = f.area().y + 1 + 1 + 1 + 1;
                    f.set_cursor_position((cursor_x, cursor_y));
                }
                SelectedField::Password => {
                    let cursor_x = f.area().x
                        + (f.area().width.saturating_sub(35)) / 2
                        + 1
                        + auth_state.password_input.len() as u16;
                    let cursor_y = f.area().y + 1 + 1 + 1 + 1 + 3;
                    f.set_cursor_position((cursor_x, cursor_y));
                }
                _ => {}
            }
        })?;

        match auth_state.selected_field {
            SelectedField::Username | SelectedField::Password => terminal.show_cursor()?,
            _ => terminal.hide_cursor()?,
        }

        let event_result = handle_auth_event(
            Duration::from_millis(50),
            auth_state.username_input.clone(),
            auth_state.password_input.clone(),
            auth_state.selected_icon_index,
            auth_state.current_mode,
            auth_state.selected_field,
            if msg_to_draw.is_empty() {
                None
            } else {
                Some(msg_to_draw.clone())
            },
        )?;

        auth_state.username_input = event_result.username_input;
        auth_state.password_input = event_result.password_input;
        auth_state.selected_icon_index = event_result.selected_icon_index;
        auth_state.current_mode = event_result.current_mode;
        auth_state.selected_field = event_result.selected_field;
        if let Some(msg) = event_result.message {
            *auth_state.message_state.lock().await = msg;
        }

        if let Some(page) = event_result.next_page {
            return Ok(page);
        }

        // Handle API calls and validation for Register/Login buttons
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

                    let hashed_password =
                        format!("{:x}", Sha256::digest(auth_state.password_input.as_bytes()));
                    *auth_state.message_state.lock().await = "Registering...".to_string();
                    terminal.draw(|f| {
                        draw_auth_ui(
                            f,
                            &auth_state.username_input,
                            &auth_state.password_input,
                            auth_state.selected_icon_index,
                            &auth_state.current_mode,
                            &auth_state.selected_field,
                            &msg_to_draw,
                            current_theme,
                        );
                    })?;

                    match auth_api::register(
                        &client,
                        &auth_state.username_input,
                        &hashed_password,
                        ICONS[auth_state.selected_icon_index],
                    )
                    .await
                    {
                        Ok(token_response) => {
                            let mut state = app_state.lock().await;
                            state.set_user_auth(
                                token_response.token,
                                auth_state.username_input.clone(),
                                token_response.icon,
                            );
                            return Ok(TuiPage::Home);
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
                    terminal.draw(|f| {
                        draw_auth_ui(
                            f,
                            &auth_state.username_input,
                            &auth_state.password_input,
                            auth_state.selected_icon_index,
                            &auth_state.current_mode,
                            &auth_state.selected_field,
                            &msg_to_draw,
                            current_theme,
                        );
                    })?;

                    let hashed_password =
                        format!("{:x}", Sha256::digest(auth_state.password_input.as_bytes()));
                    match auth_api::login(&client, &auth_state.username_input, &hashed_password)
                        .await
                    {
                        Ok(token_response) => {
                            let mut state = app_state.lock().await;
                            state.set_user_auth(
                                token_response.token,
                                auth_state.username_input.clone(),
                                token_response.icon,
                            );
                            return Ok(TuiPage::Home);
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
