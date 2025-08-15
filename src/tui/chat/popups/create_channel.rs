use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::app_state::AppState;
use crate::themes::rgb_to_color;
use crate::tui::chat::create_channel_form::{CreateChannelForm, CreateChannelInput, ICONS};

pub fn get_create_channel_popup_size() -> (u16, u16) {
    let width = 40;
    let height = 3 + 3 + 3 + 2 + 2;
    (width, height)
}

pub fn draw_create_channel_popup(
    f: &mut Frame,
    state: &mut AppState,
    area: Rect,
    create_channel_form: &mut CreateChannelForm,
    popup_block: &Block,
) {
    let current_theme = &state.current_theme;
    let inner_area = popup_block.inner(area);
    let fixed_width = 35;

    let form_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name input
            Constraint::Length(3), // Icon selector
            Constraint::Length(3), // Create button
            Constraint::Min(0),    // Spacer
            Constraint::Length(1), // Hint
        ])
        .margin(1)
        .split(inner_area);

    // Center the form elements horizontally
    let name_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(fixed_width),
            Constraint::Min(0),
        ])
        .split(form_layout[0])[1];

    let icon_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(fixed_width),
            Constraint::Min(0),
        ])
        .split(form_layout[1])[1];

    let button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(fixed_width),
            Constraint::Min(0),
        ])
        .split(form_layout[2])[1];

    // Name Input
    let name_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Channel Name")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_border_inactive))
            },
        );
    let name_paragraph = Paragraph::new(Text::from(create_channel_form.name.as_str()))
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Name {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_text_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_text_inactive))
            },
        )
        .block(name_block);
    f.render_widget(name_paragraph, name_area);

    // Icon Selector
    let icon_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title("󰓺 icon")
        .style(
            if create_channel_form.input_focused == CreateChannelInput::Icon {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_border_active))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.input_border_inactive))
            },
        );

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
                    .fg(rgb_to_color(&current_theme.colors.accent))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                icon_char,
                Style::default().fg(rgb_to_color(&current_theme.colors.dim)),
            ));
        }
        if i != center as isize + display_range as isize {
            spans.push(Span::raw("   "));
        }
    }

    let icon_paragraph = Paragraph::new(Line::from(spans))
        .alignment(ratatui::layout::Alignment::Center)
        .block(icon_block);
    f.render_widget(icon_paragraph, icon_area);

    // Create Button
    let border_style = if create_channel_form.input_focused == CreateChannelInput::CreateButton {
        Style::default().fg(rgb_to_color(&current_theme.colors.accent))
    } else {
        Style::default().fg(rgb_to_color(&current_theme.colors.border))
    };
    let create_button_style = border_style;

    let create_button_paragraph = Paragraph::new(Line::from(Span::styled(
        "Forge Channel! 󰓥(—w—) ",
        create_button_style,
    )))
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style),
    );
    f.render_widget(create_button_paragraph, button_area);

    // Hint
    let hint_paragraph = Paragraph::new(Line::from(Span::styled(
        "(Enter) Seal the Deal  / (Esc) Abort Mission ",
        Style::default().fg(rgb_to_color(&current_theme.colors.accent)),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint_paragraph, form_layout[4]);
}
