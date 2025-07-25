use crate::app::AppState;
use crate::tui::chat::popups::helpers::{render_styled_list, render_styled_paragraph};
use crate::tui::themes::{get_theme, rgb_to_color};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::Block,
    Frame,
};
// Options defined once as a static constant
static SETTINGS_OPTIONS: &[&str] = &[" Themes", "  Deconnection", "󰞋 Help"];

pub fn get_settings_popup_size() -> (u16, u16) {
    let width = SETTINGS_OPTIONS.iter().map(|s| s.len()).max().unwrap_or(0) as u16 + 10;
    // title(1) + options list + hint(1) + margin(2) + borders(2)
    let height = 1 + SETTINGS_OPTIONS.len() as u16 + 1 + 2 + 2;
    (width, height)
}

pub fn draw_settings_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let settings_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .margin(1)
        .split(inner_area);

    render_styled_list(
        f,
        SETTINGS_OPTIONS,
        Some(state.selected_setting_index),
        &current_theme,
        settings_layout[0],
        Some(Block::default()),
        None,
        None,
        Some(" "),
    );

    render_styled_paragraph(
        f,
        vec![ratatui::text::Line::from("(Esc) Retreat to Safety ")],
        &current_theme,
        settings_layout[1],
        Alignment::Center,
        None,
        Some(rgb_to_color(&current_theme.accent)),
    );
}
