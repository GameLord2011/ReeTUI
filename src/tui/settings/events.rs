use crate::app::app_state::AppState;
use crate::app::TuiPage;

use crate::tui::settings::state::{FocusedPane, SettingsScreen, SettingsState, QuitConfirmationState};
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
                            return handle_right_pane_events(settings_state, key.code, app_state).await
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
            if settings_state.main_selection == 3 { // 3 is Quit
                app_state.quit_confirmation_state = QuitConfirmationState::Active; // Directly update app_state
                settings_state.focused_pane = FocusedPane::Right;
                return Some(TuiPage::Settings); // Force redraw of settings page
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
                    handle_disconnect_events(settings_state, key_code, app_state).await;
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
                app_state.current_theme = app_state
                    .themes
                    .get(&settings_state.themes[selected_index])
                    .unwrap()
                    .clone();
            }
        }
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

fn handle_help_events(settings_state: &mut SettingsState, key_code: KeyCode) -> Option<TuiPage> { // Removed underscore
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
            log::debug!("handle_disconnect_events: Enter pressed, clearing user auth and returning TuiPage::Auth");
            app_state.clear_user_auth().await;
            return Some(TuiPage::Auth);
        }
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left,
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

use log::debug; // Add this import

fn handle_quit_confirmation_events(
    _settings_state: &mut SettingsState, // No longer directly modifying settings_state
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    debug!("handle_quit_confirmation_events: KeyCode: {:?}, quit_selection: {}", key_code, app_state.quit_selection); // Use app_state.quit_selection
    match key_code {
        KeyCode::Left => {
            if app_state.quit_selection == 1 { // If "Hell no" is selected
                app_state.quit_selection = 0; // Select "Ye"
                debug!("handle_quit_confirmation_events: Selected Ye");
            }
        }
        KeyCode::Right => {
            if app_state.quit_selection == 0 { // If "Ye" is selected
                app_state.quit_selection = 1; // Select "Hell no"
                debug!("handle_quit_confirmation_events: Selected Hell no");
            }
        }
        KeyCode::Enter => {
            debug!("handle_quit_confirmation_events: Enter pressed");
            if app_state.quit_selection == 0 {
                // "Ye" selected
                debug!("handle_quit_confirmation_events: Ye selected, setting should_exit_app to true");
                app_state.should_exit_app = true;
                app_state.next_page = Some(TuiPage::Exit);
                return None; // Return None, let the main loop handle next_page
            } else {
                // "Hell no" selected
                debug!("handle_quit_confirmation_events: Hell no selected, going back");
                app_state.quit_confirmation_state = QuitConfirmationState::Inactive;
                // app_state.focused_pane = FocusedPane::Left; // This should be handled by the main settings state
            }
        }
        KeyCode::Esc => {
            debug!("handle_quit_confirmation_events: Esc pressed, going back");
            app_state.quit_confirmation_state = QuitConfirmationState::Inactive;
            // app_state.focused_pane = FocusedPane::Left; // This should be handled by the main settings state
        }
        _ => {
            debug!("handle_quit_confirmation_events: Other key pressed: {:?}", key_code);
        }
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
