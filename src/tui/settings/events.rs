use crate::app::app_state::AppState;
use crate::app::TuiPage;
use crate::tui::auth::page::ICONS;
use crate::tui::settings::state::{FocusedPane, SettingsScreen, SettingsState};
use crate::tui::settings::SettingsEvent;
use crossterm::event::{Event, KeyCode, KeyEventKind};

pub fn handle_settings_event(
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
        KeyCode::Right => settings_state.focused_pane = FocusedPane::Right,
        KeyCode::Enter => {
            if settings_state.main_selection == 3 {
                app_state.should_exit_app = true;
                return Some(TuiPage::Exit);
            }
            settings_state.focused_pane = FocusedPane::Right
        }
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

fn handle_right_pane_events(
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
                    ()
                }
                SettingsScreen::Help => {
                    handle_help_events(settings_state, key_code);
                    ()
                }
                SettingsScreen::UserSettings => {
                    handle_user_settings_events(settings_state, key_code, app_state);
                    ()
                }
                SettingsScreen::Quit => {
                    handle_quit_events(settings_state, key_code, app_state);
                    ()
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
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left,
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

fn handle_help_events(settings_state: &mut SettingsState, key_code: KeyCode) -> Option<TuiPage> {
    match key_code {
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left,
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}

fn handle_user_settings_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    if !settings_state.is_user_logged_in() {
        if key_code == KeyCode::Left {
            settings_state.focused_pane = FocusedPane::Left;
        }
        return None;
    }

    match key_code {
        KeyCode::Left => {
            if settings_state.new_username.is_empty() {
                let mut current_icon_index = ICONS
                    .iter()
                    .position(|&i| i == settings_state.new_icon)
                    .unwrap_or(0);
                current_icon_index = if current_icon_index == 0 {
                    ICONS.len() - 1
                } else {
                    current_icon_index - 1
                };
                settings_state.new_icon = ICONS[current_icon_index].to_string();
            } else {
                settings_state.focused_pane = FocusedPane::Left;
            }
        }
        KeyCode::Right => {
            if settings_state.new_username.is_empty() {
                let mut current_icon_index = ICONS
                    .iter()
                    .position(|&i| i == settings_state.new_icon)
                    .unwrap_or(0);
                current_icon_index = (current_icon_index + 1) % ICONS.len();
                settings_state.new_icon = ICONS[current_icon_index].to_string();
            }
        }
        KeyCode::Char(c) => {
            settings_state.new_username.push(c);
        }
        KeyCode::Backspace => {
            settings_state.new_username.pop();
        }
        KeyCode::Enter => {
            app_state.username = Some(settings_state.new_username.clone());
            app_state.user_icon = Some(settings_state.new_icon.clone());
        }
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }

    None
}

fn handle_quit_events(
    settings_state: &mut SettingsState,
    key_code: KeyCode,
    app_state: &mut AppState,
) -> Option<TuiPage> {
    match key_code {
        KeyCode::Enter => {
            app_state.should_exit_app = true;
            return Some(TuiPage::Exit);
        }
        KeyCode::Left => settings_state.focused_pane = FocusedPane::Left,
        KeyCode::Esc => return Some(TuiPage::Chat),
        _ => {}
    }
    None
}
