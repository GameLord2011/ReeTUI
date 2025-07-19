use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

// Helper function to get filtered users, to avoid redundancy
fn get_filtered_users<'a>(state: &'a AppState) -> Vec<&'a String> {
    let username = state.username.as_ref().map(|s| s.to_lowercase()); // Get lowercase username once
    state
        .active_users
        .iter()
        .filter(|user| {
            let user_lower = user.to_lowercase();
            user_lower.contains(&state.mention_query.to_lowercase())
                && username.as_ref().map_or(true, |u| user_lower != *u)
        })
        .collect()
}

pub fn get_mentions_popup_size(state: &AppState) -> (u16, u16) {
    let filtered_users = get_filtered_users(state); // Call the helper once

    let height = std::cmp::min(filtered_users.len() as u16, 10) + 2; // +2 for borders
    let width = filtered_users
        .iter()
        .map(|user| user.len())
        .max()
        .unwrap_or(20) as u16
        + 4; // +4 for borders and padding
    (width, height)
}

pub fn draw_mentions_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let filtered_users = get_filtered_users(state); // Call the helper once

    let users: Vec<ListItem> = filtered_users
        .iter()
        .enumerate()
        .map(|(i, user)| {
            let is_selected = i == state.selected_mention_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .bg(rgb_to_color(&current_theme.button_bg_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(rgb_to_color(&current_theme.text))
                    .bg(rgb_to_color(&current_theme.dim)) // Applied background color based on "text color of the demi circle"
            };
            ListItem::new(user.as_str()).style(style) // Use as_str() to avoid cloning String here
        })
        .collect();

    let users_list = List::new(users)
        .block(Block::default())
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(" ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_mention_index));
    f.render_stateful_widget(users_list, inner_area, &mut list_state);
}
