use crate::app::app_state::AppState;
use crate::app::TuiPage;
use std::fs;
use crate::config;

use crate::tui::settings::state::{
    DisconnectConfirmationState, FocusedPane, QuitConfirmationState, SettingsScreen, SettingsState,
};
use crate::tui::settings::SettingsEvent;
use crossterm::event::{Event, KeyCode, KeyEventKind};

pub async fn handle_settings_event(
    settings_state: &mut SettingsState,
    app_state: &mut AppState,
    event: SettingsEvent,
) -> Option<TuiPage> {
    match event {
        SettingsEvent::Key(key_event) => {
            if let Event::Key(key) = key_event {
                if key.kind == KeyEventKind::Press {
                    match settings_state.focused_pane {
                        FocusedPane::Left => {
                            return handle_left_pane_events(settings_state, key.code, app_state)
                        }
                        FocusedPane::Right => {
                            return handle_right_pane_events(settings_state, key.code, app_state)
                                .await
                        }
                    }
                }
            }
        }
        SettingsEvent::Tick => {}
    }
    None
}

fn handle_left_pane_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Up => settings_state.previous_main_setting(),
        KeyCode::Down => settings_state.next_main_setting(),
        KeyCode::Enter => {
            if settings_state.main_selection == 3 {
                // 3 is Quit
                app_state.quit_confirmation_state = QuitConfirmationState::Active; // Directly update app_state
                settings_state.focused_pane = FocusedPane::Right;
                return Some(TuiPage::Settings); // Force redraw of settings page
            } else if settings_state.main_selection == 2 {
                // 2 is Disconnect
                app_state.disconnect_confirmation_state = DisconnectConfirmationState::Active;
                settings_state.focused_pane = FocusedPane::Right;
                return Some(TuiPage::Settings);
            } else {
                settings_state.focused_pane = FocusedPane::Right;
            }
        }
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

async fn handle_right_pane_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {
            match settings_state.screen {
                SettingsScreen::Themes => {
                    handle_themes_events(settings_state, key_code, app_state);
                }
                SettingsScreen::Help => {
                    handle_help_events(settings_state, key_code);
                }
                SettingsScreen::Disconnect => {
                    if app_state.disconnect_confirmation_state
                        == DisconnectConfirmationState::Active
                    {
                        handle_disconnect_confirmation_events(settings_state, key_code, app_state)
                            .await;
                    } else {
                        handle_disconnect_events(settings_state, key_code, app_state).await;
                    }
                }
                SettingsScreen::Quit => {
                    if app_state.quit_confirmation_state == QuitConfirmationState::Active {
                        handle_quit_confirmation_events(settings_state, key_code, app_state);
                    } else {
                        handle_quit_events(settings_state, key_code, app_state);
                    }
                }
            }
            None
        }
    }
}

fn handle_themes_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Up => settings_state.previous_theme(),
        KeyCode::Down => settings_state.next_theme(),
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left, // Re-added
        KeyCode::Enter => {
            if let Some(selected_index) = settings_state.theme_list_state.selected() {
                let selected_theme_name = settings_state.themes[selected_index];
                app_state.current_theme = app_state
                    .themes
                    .get(&selected_theme_name)
                    .unwrap()
                    .clone();
                app_state.config.current_theme_name = selected_theme_name;
            }
        }
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {} 
    }
    None
}

fn handle_help_events(settings_state: &mut SettingsState, key_code: KeyCode) -> Option<TuiPage> {
    // Removed underscore
    match key_code {
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left, // Re-added
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

async fn handle_disconnect_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Enter => {
            app_state.clear_user_auth().await;
            let config_path = config::get_config_path();
            if config_path.exists() {
                let _ = fs::remove_file(config_path);
            }
            return Some(TuiPage::Auth);
        }
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left,
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

fn handle_quit_confirmation_events(
    _settings_state: &mut SettingsState, // No longer directly modifying settings_state
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Left => {
            if app_state.quit_selection == 1 {
                // If "Hell no" is selected
                app_state.quit_selection = 0; // Select "Ye"
            }
        }
        KeyCode::Right => {
            if app_state.quit_selection == 0 {
                // If "Ye" is selected
                app_state.quit_selection = 1; // Select "Hell no"
            }
        }
        KeyCode::Enter => {
            if app_state.quit_selection == 0 {
                app_state.should_exit_app = true;
                app_state.next_page = Some(TuiPage::Exit);
                return None; // Return None, let the main loop handle next_page
            } else {
                app_state.quit_confirmation_state = QuitConfirmationState::Inactive;
            }
        }
        KeyCode::Esc => {
            app_state.quit_confirmation_state = QuitConfirmationState::Inactive;
        }
        _ => {}
    }
    None
}

fn handle_quit_events(
    settings_state: &mut SettingsState, // Removed underscore
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Enter => {
            app_state.should_exit_app = true;
            return Some(TuiPage::Exit);
        }
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left, // Re-added
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

async fn handle_disconnect_confirmation_events(
    _settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Left => {
            if app_state.disconnect_selection == 1 {
                app_state.disconnect_selection = 0;
            }
        }
        KeyCode::Right => {
            if app_state.disconnect_selection == 0 {
                app_state.disconnect_selection = 1;
            }
        }
        KeyCode::Enter => {
            if app_state.disconnect_selection == 0 {
                app_state.clear_user_auth().await;
                let config_path = config::get_config_path();
                if config_path.exists() {
                    let _ = fs::remove_file(config_path);
                }
                return Some(TuiPage::Auth);
            } else {
                app_state.disconnect_confirmation_state = DisconnectConfirmationState::Inactive;
            }
        }
        KeyCode::Esc => {
            app_state.disconnect_confirmation_state = DisconnectConfirmationState::Inactive;
        }
        _ => {}
    }
    None
}
