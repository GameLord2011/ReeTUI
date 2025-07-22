use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

fn get_filtered_emojis(state: &AppState) -> Vec<&'static emojis::Emoji> {
    emojis::iter()
        .filter(|emoji| {
            emoji
                .name()
                .to_lowercase()
                .contains(&state.emoji_query.to_lowercase())
        })
        .collect()
}

pub fn get_emojis_popup_size(state: &AppState) -> (u16, u16) {
    let filtered_emojis = get_filtered_emojis(state);

    let height = std::cmp::min(filtered_emojis.len() as u16, 10) + 2; // +2 for borders
    let width = filtered_emojis
        .iter()
        .map(|emoji| format!("{}  {}", emoji.as_str(), emoji.name()).len())
        .max()
        .unwrap_or(20) as u16
        + 4; // +4 for borders and padding
    (width, height)
}

pub fn draw_emojis_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let filtered_emojis = get_filtered_emojis(state);

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
            ListItem::new(format!("{}  {}", emoji.as_str(), emoji.name())).style(style)
        })
        .collect();

    let emojis_list = List::new(emoji_list)
        .block(Block::default().title("Emoji Extravaganza "))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(" ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_emoji_index));
    f.render_stateful_widget(emojis_list, inner_area, &mut list_state);
}
