use crate::app::AppState;
use crate::tui::chat::popups::helpers::{draw_dialog_popup, get_dialog_popup_size};
use crate::tui::themes::get_theme;
use ratatui::{layout::Rect, widgets::Block, Frame};

const POPUP_TITLE: &str = "Abandon Ship? ";
const POPUP_HINT: &str = "(Y)es, Beam Me Up! 󰚑 / (N)o, Stay Awhile ";

pub fn get_deconnection_popup_size() -> (u16, u16) {
    get_dialog_popup_size(POPUP_TITLE, POPUP_HINT)
}

pub fn draw_deconnection_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    popup_block: &Block,
) {
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