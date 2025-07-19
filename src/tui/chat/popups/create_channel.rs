use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text}, // Added Text for optimization
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::tui::chat::create_channel_form::{CreateChannelForm, CreateChannelInput, ICONS};
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn get_create_channel_popup_size() -> (u16, u16) {
    let hint_text = "(Enter) Confirm / (Esc) Cancel";
    let icons_row_width = (ICONS.len() * 3) as u16;
    // content: name_input(3) + icon_selector(3) + spacer(1) + create_button(3) + hint(1) = 11
    // layout: form_margin(2) + popup_border(2) = 4
    let height = 3 + 3 + 1 + 3 + 1 + 2 + 2;
    let width = hint_text.len().max(icons_row_width as usize) as u16 + 4; // +4 for borders and margin
    (width, height)
}

pub fn draw_create_channel_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    create_channel_form: &mut CreateChannelForm,
    popup_block: &Block,
) {
    let current_theme = get_theme(state.current_theme);
    let inner_area = popup_block.inner(area);

    let form_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(inner_area);

    let icons_row_width = (ICONS.len() * 3) as u16; // Calculated once

    let name_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(icons_row_width),
            Constraint::Min(0),
        ])
        .split(form_layout[0]);

    let name_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title("Channel Name")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_border_inactive))
            },
        );
    let name_paragraph = Paragraph::new(Text::from(create_channel_form.name.as_str())) // Optimized: avoid clone
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.input_text_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_text_inactive))
            },
        )
        .block(name_block);
    f.render_widget(name_paragraph, name_area_h[1]);

    let icon_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title("Channel Icon")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Icon {
                Style::default().fg(rgb_to_color(&current_theme.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.input_border_inactive))
            },
        );

    let icon_selector_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(icons_row_width),
            Constraint::Min(0),
        ])
        .split(form_layout[1]);

    let len = ICONS.len();
    let center = create_channel_form.selected_icon_index;
    let display_range = 3;

    let mut spans = Vec::with_capacity(display_range * 2 + 1);

    for i in
        (center as isize - display_range as isize)..(center as isize + display_range as isize + 1)
    {
        let actual_index = (i % len as isize + len as isize) % len as isize;
        let icon_char = ICONS[actual_index as usize];
        if actual_index == center as isize {
            spans.push(Span::styled(
                icon_char,
                Style::default()
                    .fg(rgb_to_color(&current_theme.accent))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                icon_char,
                Style::default().fg(rgb_to_color(&current_theme.dim)),
            ));
        }
        if i != center as isize + display_range as isize {
            spans.push(Span::raw("   "));
        }
    }

    let icon_paragraph = Paragraph::new(Line::from(spans))
        .alignment(ratatui::layout::Alignment::Center)
        .block(icon_block);
    f.render_widget(icon_paragraph, icon_selector_area_h[1]);

    let create_button_style =
        if create_channel_form.input_focused == CreateChannelInput::CreateButton {
            Style::default().fg(rgb_to_color(&current_theme.button_text_active))
        } else {
            Style::default().fg(rgb_to_color(&current_theme.button))
        };
    let create_button_paragraph = Paragraph::new(Line::from(Span::styled(
        "Create Channel",
        create_button_style,
    )))
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(
                if create_channel_form.input_focused == CreateChannelInput::CreateButton {
                    Style::default().fg(rgb_to_color(&current_theme.input_border_active))
                } else {
                    Style::default().fg(rgb_to_color(&current_theme.input_border_inactive))
                },
            ),
    );
    let create_button_area_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(icons_row_width),
            Constraint::Min(0),
        ])
        .split(form_layout[3]);
    f.render_widget(create_button_paragraph, create_button_area_h[1]);

    let hint_paragraph = Paragraph::new(Line::from(Span::styled(
        "(Enter) Confirm / (Esc) Cancel",
        Style::default().fg(rgb_to_color(&current_theme.accent)),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint_paragraph, form_layout[4]);
}
