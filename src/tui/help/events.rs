use crate::app::{app_state::AppState, TuiPage};
use crate::tui::help::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key_events(key: KeyEvent, app_state: &mut AppState) -> Option<TuiPage> {
    let current_page = app_state.help_state.current_page;
    let total_pages = app_state.help_state.total_pages;

    match key.code {
        KeyCode::Enter => {
            if current_page == total_pages - 1 {
                app_state.config.tutorial_seen = true;
                Some(TuiPage::Auth)
            } else {
                app_state.help_state.next_page();
                None
            }
        }
        KeyCode::Char('s') => {
            if current_page == 1 && key.modifiers.contains(KeyModifiers::CONTROL) {
                app_state.help_state.next_page();
                None
            } else {
                None
            }
        }
        KeyCode::Char('n') => {
            if current_page == 2 && key.modifiers.contains(KeyModifiers::CONTROL) {
                app_state.help_state.next_page();
                None
            } else {
                None
            }
        }
        _ => None,
    }
}
