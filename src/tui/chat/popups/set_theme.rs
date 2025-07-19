use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::tui::chat::create_channel_form::ICONS;
use crate::tui::chat::theme_settings_form::ThemeSettingsForm;
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn get_set_theme_popup_size(theme_settings_form: &ThemeSettingsForm) -> (u16, u16) {
    // content: list + hint = (themes_len + 2) + 2 = themes_len + 4
    // layout: content_margin(2) + popup_border(2) = 4
    let height = (theme_settings_form.themes.len() as u16 + 2) + 2;
    let width = theme_settings_form
        .themes
        .iter()
        .map(|theme| format!("{} {:?}", theme.icon(), theme).len())
        .max()
        .unwrap_or(20) as u16
        + 10; // +10 for padding and highlight symbol
    (width, height)
}

pub fn draw_set_theme_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    theme_settings_form: &mut ThemeSettingsForm,
    popup_block: &Block,
) {
    let current_theme = get_theme(state.current_theme);
    let theme_items: Vec<ListItem> = theme_settings_form
        .themes
        .iter()
        .enumerate()
        .map(|(i, &theme_name)| {
            let is_selected = i == theme_settings_form.selected_theme_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.button_text_active))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(format!("{} {:?}", theme_name.icon(), theme_name)).style(style)
        })
        .collect();

    let inner_area = popup_block.inner(area);

    let num_themes = theme_settings_form.themes.len() as u16;
    let required_list_height = num_themes;

    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(required_list_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(inner_area);

    let theme_list_width = (ICONS.len() * 3) as u16;
    let list_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(theme_list_width),
            Constraint::Min(0),
        ])
        .split(content_layout[1]);

    let theme_list_block = Block::default()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.popup_border))
                .bg(rgb_to_color(&current_theme.background)),
        );

    let theme_list = List::new(theme_items)
        .block(theme_list_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(" ");

    f.render_stateful_widget(
        theme_list,
        list_area_h[1],
        &mut theme_settings_form.list_state,
    );

    let hint_paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            "(Up/Down) Navigate",
            Style::default().fg(rgb_to_color(&current_theme.accent)),
        )),
    ])
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint_paragraph, content_layout[3]);
}
