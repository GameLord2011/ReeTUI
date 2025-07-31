use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::app_state::AppState;

pub fn draw_downloads_popup(f: &mut Frame, _app_state: &mut AppState) {
    let block = Block::default().title("Downloads").borders(Borders::ALL);
    let area = centered_rect(60, 20, f.area());
    f.render_widget(block.clone(), area);

    let no_downloads_message = "No downloads available.";
    let paragraph = Paragraph::new(no_downloads_message)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
