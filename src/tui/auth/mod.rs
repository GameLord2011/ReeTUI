pub mod events;
pub mod page;
pub mod state;

use crate::api::auth_api;
use crate::app::app_state::AppState;
use crate::app::TuiPage;
use crate::tui::auth::events::handle_auth_event;
use crate::tui::auth::page::{draw_auth_ui, get_validation_error, ICONS};
use crate::tui::auth::state::{AuthMode, AuthState, SelectedField};
use crate::tui::notification::notification::NotificationType;
use crossterm::event;
use ratatui::Terminal;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::{io, time::Duration};

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
            state.quit_confirmation_state,
            state.quit_selection,
        )
    };
    let client = reqwest::Client::new();
    terminal.show_cursor()?;

    loop {
        let mut app_state_guard = app_state.lock().await;
        app_state_guard.notification_manager.update();

        let theme = app_state_guard.current_theme.clone();
        terminal.draw(|f| {
            draw_auth_ui::<B>(
                f,
                &auth_state.username_input,
                &auth_state.password_input,
                auth_state.selected_icon_index,
                &auth_state.current_mode,
                &auth_state.selected_field,
                &theme,
                &mut app_state_guard,
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
                if let Some(target_page) = crate::tui::settings::handle_settings_key_event(
                    crate::tui::settings::SettingsEvent::Key(event.clone()),
                    &mut app_state_guard,
                    &mut settings_state,
                ) {
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
                    if key.code == event::KeyCode::Char('s')
                        && key.modifiers.contains(event::KeyModifiers::CONTROL)
                    {
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
                    app_state.clone(),
                )?;

                auth_state.selected_icon_index = event_result.selected_icon_index;
                auth_state.current_mode = event_result.current_mode;
                auth_state.selected_field = event_result.selected_field;
                auth_state.update_focus();

                                if let Some(page) = event_result.next_page {
                    app_state_guard.next_page = Some(page);
                }

                if event_result.should_submit {
                    match auth_state.selected_field {
                        SelectedField::RegisterButton => {
                            if auth_state.current_mode == AuthMode::Register {
                                let validation_error = get_validation_error(
                                    &auth_state.username_input,
                                    &auth_state.password_input,
                                    &auth_state.current_mode,
                                    &mut app_state_guard.notification_manager,
                                    app_state.clone(),
                                )
                                .await;
                                if validation_error.is_some() {
                                    continue;
                                }

                                let hashed_password = format!(
                                    "{:x}",
                                    Sha256::digest(auth_state.password_input.text.as_bytes())
                                );
                                let loading_notification = app_state_guard
                                    .notification_manager
                                    .add(
                                        "Registering...".to_string(),
                                        "Please wait...".to_string(),
                                        NotificationType::Loading,
                                        None,
                                        app_state.clone(),
                                    )
                                    .await;

                                terminal.draw(|f| {
                                    draw_auth_ui::<B>(
                                        f,
                                        &auth_state.username_input,
                                        &auth_state.password_input,
                                        auth_state.selected_icon_index,
                                        &auth_state.current_mode,
                                        &auth_state.selected_field,
                                        &theme,
                                        &mut app_state_guard,
                                        &mut settings_state,
                                    );
                                })?;

                                drop(app_state_guard); // Release the lock before async calls that might re-acquire it

                                let register_result = auth_api::register(
                                    &client,
                                    &auth_state.username_input.text,
                                    &hashed_password,
                                    ICONS[auth_state.selected_icon_index],
                                )
                                .await;

                                let mut app_state_guard = app_state.lock().await; // Re-acquire the lock once after the API call

                                match register_result
                                {
                                    Ok(token_response) => {
                                        if let Some(loading) = loading_notification {
                                            tokio::spawn(async move { loading.remove().await; });
                                        }
                                        app_state_guard
                                            .notification_manager
                                            .add(
                                                "Registration Success".to_string(),
                                                "You have been successfully registered."
                                                    .to_string(),
                                                NotificationType::Success,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            )
                                            .await;
                                        app_state_guard.set_user_auth(
                                            token_response.token,
                                            auth_state.username_input.text.clone(),
                                            token_response.icon,
                                        );
                                        terminal.draw(|f| {
                                            draw_auth_ui::<B>(
                                                f,
                                                &auth_state.username_input,
                                                &auth_state.password_input,
                                                auth_state.selected_icon_index,
                                                &auth_state.current_mode,
                                                &auth_state.selected_field,
                                                &theme,
                                                &mut app_state_guard,
                                                &mut settings_state,
                                            );
                                        })?;
                                        return Ok(TuiPage::Chat);
                                    }
                                    Err(e) => {
                                        if let Some(loading) = loading_notification {
                                            tokio::spawn(async move { loading.remove().await; });
                                        }
                                        app_state_guard
                                            .notification_manager
                                            .add(
                                                "Registration Error".to_string(),
                                                e.to_string(),
                                                NotificationType::Error,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            )
                                            .await;
                                        terminal.draw(|f| {
                                            draw_auth_ui::<B>(
                                                f,
                                                &auth_state.username_input,
                                                &auth_state.password_input,
                                                auth_state.selected_icon_index,
                                                &auth_state.current_mode,
                                                &auth_state.selected_field,
                                                &theme,
                                                &mut app_state_guard,
                                                &mut settings_state,
                                            );
                                        })?;
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
                                    &mut app_state_guard.notification_manager,
                                    app_state.clone(),
                                )
                                .await;
                                if validation_error.is_some() {
                                    continue;
                                }

                                let loading_notification = app_state_guard
                                    .notification_manager
                                    .add(
                                        "Logging in...".to_string(),
                                        "Please wait...".to_string(),
                                        NotificationType::Loading,
                                        None,
                                        app_state.clone(),
                                    )
                                    .await;

                                terminal.draw(|f| {
                                    draw_auth_ui::<B>(
                                        f,
                                        &auth_state.username_input,
                                        &auth_state.password_input,
                                        auth_state.selected_icon_index,
                                        &auth_state.current_mode,
                                        &auth_state.selected_field,
                                        &theme,
                                        &mut app_state_guard,
                                        &mut settings_state,
                                    );
                                })?;

                                drop(app_state_guard); // Release the lock before async calls that might re-acquire it

                                let hashed_password = format!(
                                    "{:x}",
                                    Sha256::digest(auth_state.password_input.text.as_bytes())
                                );
                                let login_result = auth_api::login(
                                    &client,
                                    &auth_state.username_input.text,
                                    &hashed_password,
                                )
                                .await;

                                let mut app_state_guard = app_state.lock().await; // Re-acquire the lock once after the API call

                                match login_result
                                {
                                    Ok(token_response) => {
                                        if let Some(loading) = loading_notification {
                                            tokio::spawn(async move { loading.remove().await; });
                                        }
                                        app_state_guard
                                            .notification_manager
                                            .add(
                                                "Login Success".to_string(),
                                                "You have been successfully logged in.".to_string(),
                                                NotificationType::Success,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            )
                                            .await;
                                        app_state_guard.set_user_auth(
                                            token_response.token,
                                            auth_state.username_input.text.clone(),
                                            token_response.icon,
                                        );
                                        terminal.draw(|f| {
                                            draw_auth_ui::<B>(
                                                f,
                                                &auth_state.username_input,
                                                &auth_state.password_input,
                                                auth_state.selected_icon_index,
                                                &auth_state.current_mode,
                                                &auth_state.selected_field,
                                                &theme,
                                                &mut app_state_guard,
                                                &mut settings_state,
                                            );
                                        })?;
                                        return Ok(TuiPage::Chat);
                                    }
                                    Err(e) => {
                                        if let Some(loading) = loading_notification {
                                            tokio::spawn(async move { loading.remove().await; });
                                        }
                                        app_state_guard
                                            .notification_manager
                                            .add(
                                                "Login Error".to_string(),
                                                e.to_string(),
                                                NotificationType::Error,
                                                Some(Duration::from_secs(3)),
                                                app_state.clone(),
                                            )
                                            .await;
                                        terminal.draw(|f| {
                                            draw_auth_ui::<B>(
                                                f,
                                                &auth_state.username_input,
                                                &auth_state.password_input,
                                                auth_state.selected_icon_index,
                                                &auth_state.current_mode,
                                                &auth_state.selected_field,
                                                &theme,
                                                &mut app_state_guard,
                                                &mut settings_state,
                                            );
                                        })?;
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
