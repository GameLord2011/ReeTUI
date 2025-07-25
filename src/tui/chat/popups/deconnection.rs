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

pub fn get_deconnection_popup_size() -> (u16, u16) {
    let text1 = "Abandon Ship? ";
    let text2 = "(Y)es, Beam Me Up! 󰚑 / (N)o, Stay Awhile ";
    let width = text1.len().max(text2.len()) as u16 + 4;
    let height = 1 + 1 + 1 + 2 + 2;
    (width, height)
}

pub fn draw_deconnection_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    popup_block: &Block,
) {
    let current_theme = get_theme(state.current_theme);
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "Abandon Ship? ",
            Style::default().fg(rgb_to_color(&current_theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            "(Y)es, Beam Me Up! 󰚑 / (N)o, Stay Awhile ",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}
