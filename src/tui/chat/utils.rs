use ratatui::style::Color;
use std::hash::{Hash, Hasher};
use ratatui::layout::{Rect, Layout, Direction, Constraint};

pub fn get_color_for_user(username: &str) -> Color {
    let colors = [
        Color::Rgb(255, 0, 255),
        Color::Rgb(139, 0, 255),
        Color::Rgb(0, 191, 255),
        Color::Rgb(0, 255, 127),
        Color::Rgb(255, 215, 0),
        Color::Rgb(255, 105, 180),
        Color::Rgb(255, 69, 0),
        Color::Rgb(50, 205, 50),
    ];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    username.hash(&mut hasher);
    let hash = hasher.finish();
    colors[(hash % colors.len() as u64) as usize]
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
