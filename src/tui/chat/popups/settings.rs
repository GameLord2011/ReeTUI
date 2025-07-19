use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

// Options defined once as a static constant
static SETTINGS_OPTIONS: &[&str] = &[" Themes", "  Deconnection", "󰞋 Help"];

pub fn get_settings_popup_size() -> (u16, u16) {
    let width = SETTINGS_OPTIONS.iter().map(|s| s.len()).max().unwrap_or(0) as u16 + 10;
    // title(1) + options list + hint(1) + margin(2) + borders(2)
    let height = 1 + SETTINGS_OPTIONS.len() as u16 + 1 + 2 + 2;
    (width, height)
}

pub fn draw_settings_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let settings_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(inner_area);

    let option_items: Vec<ListItem> = SETTINGS_OPTIONS // Use the static constant
        .iter()
        .enumerate()
        .map(|(i, &option)| {
            let is_selected = i == state.selected_setting_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .bg(rgb_to_color(&current_theme.button_bg_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(option).style(style)
        })
        .collect();

    let options_list = List::new(option_items)
        .block(Block::default())
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(" ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_setting_index));
    f.render_stateful_widget(options_list, settings_layout[0], &mut list_state);

    let hint_paragraph_ig = Paragraph::new(Text::styled(
        "",
        Style::default().fg(rgb_to_color(&current_theme.accent)),
    ))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint_paragraph_ig, settings_layout[1]);

    let hint_paragraph = Paragraph::new(Text::styled(
        "(Esc) Cancel",
        Style::default().fg(rgb_to_color(&current_theme.accent)),
    ))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint_paragraph, settings_layout[2]);
}
