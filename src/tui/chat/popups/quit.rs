use crate::app::AppState;
use crate::tui::chat::popups::helpers::{draw_dialog_popup, get_dialog_popup_size};
use crate::tui::themes::get_theme;
use ratatui::{layout::Rect, widgets::Block, Frame};

const POPUP_TITLE: &str = "Ready to Exit the Matrix? 󱔼";
const POPUP_HINT: &str = "(Q)uit and Chill 󱠢 / (Esc) Not Yet! ";

pub fn get_quit_popup_size() -> (u16, u16) {
    get_dialog_popup_size(POPUP_TITLE, POPUP_HINT)
}

pub fn draw_quit_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    draw_dialog_popup(
        f,
        &current_theme,
        area,
        popup_block,
        POPUP_TITLE,
        POPUP_HINT,
    );
}