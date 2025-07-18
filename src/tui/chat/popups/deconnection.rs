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

pub fn draw_deconnection_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "  Are you sure you want to disconnect?",
            Style::default().fg(rgb_to_color(&current_theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            "  (Y)es / (N)o / (Esc) Cancel",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}
