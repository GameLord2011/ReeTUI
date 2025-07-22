use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_debug_json_popup(f: &mut Frame, area: Rect, content: &str) {
    let block = Block::default()
        .title("JSON Jungle: Debugging Delights ")
        .borders(Borders::ALL);
    f.render_widget(block, area);

    let paragraph = Paragraph::new(content).wrap(ratatui::widgets::Wrap { trim: false });

    let inner_area = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);

    f.render_widget(paragraph, inner_area);
}

pub fn get_debug_json_popup_size() -> (u16, u16) {
    (80, 20) // Width, Height
}
