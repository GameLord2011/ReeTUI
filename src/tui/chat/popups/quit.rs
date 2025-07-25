use ratatui::{
    layout::Alignment,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn get_quit_popup_size() -> (u16, u16) {
    let text1 = "Ready to Exit the Matrix? 󱔼";
    let text2 = "(Q)uit and Chill 󱠢 / (Esc) Not Yet! ";
    let width = text1.len().max(text2.len()) as u16 + 4;
    // content: text1(1) + empty_line(1) + text2(1) = 3
    // layout: popup_border(2) + paragraph_margin(2) = 4
    let height = 1 + 1 + 1 + 2 + 2;
    (width, height)
}

pub fn draw_quit_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "Ready to Exit the Matrix? 󱔼",
            Style::default().fg(rgb_to_color(&current_theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            "(Q)uit and Chill 󱠢 / (Esc) Not Yet! ",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}
