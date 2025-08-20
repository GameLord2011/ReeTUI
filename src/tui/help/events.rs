use crate::app::{app_state::AppState, TuiPage};
use crate::tui::help::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key_events(key: KeyEvent, app_state: &mut AppState) -> Option<TuiPage> {
    let current_page = app_state.help_state.current_page;

    match key.code {
        KeyCode::Enter => app_state.help_state.next_page(),
        KeyCode::Char('s') => {
            if current_page == 1 && key.modifiers.contains(KeyModifiers::CONTROL) {
                app_state.help_state.next_page()
            } else {
                None
            }
        }
        KeyCode::Char('n') => {
            if current_page == 2 && key.modifiers.contains(KeyModifiers::CONTROL) {
                app_state.help_state.next_page()
            } else {
                None
            }
        }
        _ => None,
    }
}
