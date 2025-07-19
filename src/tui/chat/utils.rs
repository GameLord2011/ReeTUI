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

pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Length(r.height.saturating_sub(height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Length(r.width.saturating_sub(width) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn centered_rect_with_size(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Length(r.height.saturating_sub(height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Length(r.width.saturating_sub(width) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn centered_rect_with_size_and_padding(width: u16, height: u16, padding_x: u16, padding_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height + padding_y * 2) / 2),
            Constraint::Length(height + padding_y * 2),
            Constraint::Length(r.height.saturating_sub(height + padding_y * 2) / 2),
        ])
        .split(r);

    let popup_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width + padding_x * 2) / 2),
            Constraint::Length(width + padding_x * 2),
            Constraint::Length(r.width.saturating_sub(width + padding_x * 2) / 2),
        ])
        .split(popup_layout[1])[1];

    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(padding_y),
            Constraint::Length(height),
            Constraint::Length(padding_y),
        ])
        .split(popup_layout)[1]
}
