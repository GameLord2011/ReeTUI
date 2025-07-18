use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn draw_mentions_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let filtered_users: Vec<String> = state
        .active_users
        .iter()
        .filter(|user| {
            user.to_lowercase()
                .contains(&state.mention_query.to_lowercase())
                && user != &&state.username.clone().unwrap_or_default()
        })
        .cloned()
        .collect();

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
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(user.clone()).style(style)
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
        .highlight_symbol(">> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_mention_index));
    f.render_stateful_widget(users_list, inner_area, &mut list_state);
}
