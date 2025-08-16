use crate::themes::Rgb;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use std::hash::{Hash, Hasher};
// E
pub fn get_color_for_user(username: &str, colors: &Vec<Rgb>) -> Color {
    if colors.is_empty() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        username.hash(&mut hasher);
        let hash = hasher.finish();
        let r = (hash & 0xFF) as u8;
        let g = ((hash >> 8) & 0xFF) as u8;
        let b = ((hash >> 16) & 0xFF) as u8;
        return Color::Rgb(r, g, b);
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    username.hash(&mut hasher);
    let hash = hasher.finish();
    let index = (hash as usize) % colors.len();
    let rgb = colors[index];
    Color::Rgb(rgb.0, rgb.1, rgb.2)
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

pub fn centered_rect_with_size_and_padding(
    width: u16,
    height: u16,
    padding_x: u16,
    padding_y: u16,
    r: Rect,
) -> Rect {
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
