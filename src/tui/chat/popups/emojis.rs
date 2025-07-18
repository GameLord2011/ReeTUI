use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn get_emojis_popup_height(state: &AppState) -> u16 {
    let filtered_emojis_count = emojis::iter()
        .filter(|emoji| {
            emoji
                .name()
                .to_lowercase()
                .contains(&state.emoji_query.to_lowercase())
        })
        .count();
    // content (min(filtered_emojis_count, 10)) + borders (2) + extra padding (2)
    std::cmp::min(filtered_emojis_count as u16, 10) + 2 + 2
}

pub fn draw_emojis_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let filtered_emojis: Vec<_> = emojis::iter()
        .filter(|emoji| {
            emoji
                .name()
                .to_lowercase()
                .contains(&state.emoji_query.to_lowercase())
        })
        .collect();

    let emoji_list: Vec<ListItem> = filtered_emojis
        .iter()
        .enumerate()
        .map(|(i, emoji)| {
            let is_selected = i == state.selected_emoji_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .bg(rgb_to_color(&current_theme.button_bg_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(format!("{} | {}", emoji.as_str(), emoji.name())).style(style)
        })
        .collect();

    let emojis_list = List::new(emoji_list)
        .block(Block::default())
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(">> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_emoji_index));
    f.render_stateful_widget(emojis_list, inner_area, &mut list_state);
}
