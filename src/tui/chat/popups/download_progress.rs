use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge},
};

pub fn draw_download_progress_popup(f: &mut Frame, area: Rect, progress: u8) {
    let block = Block::default()
        .title("Downloading Awesomeness ")
        .borders(Borders::ALL);
    f.render_widget(block, area);

    let gauge_area = Rect::new(area.x + 2, area.y + 2, area.width - 4, area.height - 4);

    let label = format!("{}%", progress);
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
        .percent(progress as u16)
        .label(label);

    f.render_widget(gauge, gauge_area);
}

pub fn get_download_progress_popup_size() -> (u16, u16) {
    (50, 5) // Width, Height
}
