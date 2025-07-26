use crate::tui::themes::{rgb_to_color, Theme};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

/// funny
pub fn render_styled_list(
    f: &mut Frame,
    items: &[&str],
    selected_index: Option<usize>,
    theme: &Theme,
    area: Rect,
    block: Option<Block>,
    highlight_fg: Option<ratatui::style::Color>,
    highlight_bg: Option<ratatui::style::Color>,
    highlight_symbol: Option<&str>,
) {
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let is_selected = selected_index.map_or(false, |idx| idx == i);
            let style = if is_selected {
                Style::default()
                    .fg(highlight_fg.unwrap_or(rgb_to_color(&theme.button_text_active)))
                    .bg(highlight_bg.unwrap_or(rgb_to_color(&theme.button_bg_active)))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&theme.text))
            };
            ListItem::new(item).style(style)
        })
        .collect();
    let mut list_state = ListState::default();
    if let Some(idx) = selected_index {
        list_state.select(Some(idx));
    }
    let mut list_widget = List::new(list_items);
    if let Some(b) = block {
        list_widget = list_widget.block(b);
    }
    list_widget = list_widget.highlight_style(
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(highlight_fg.unwrap_or(rgb_to_color(&theme.button_text_active)))
            .bg(highlight_bg.unwrap_or(rgb_to_color(&theme.button_bg_active))),
    );
    if let Some(symbol) = highlight_symbol {
        list_widget = list_widget.highlight_symbol(symbol);
    }
    f.render_stateful_widget(list_widget, area, &mut list_state);
}

/// Helper to render a styled paragraph
pub fn render_styled_paragraph(
    f: &mut Frame,
    lines: Vec<Line>,
    theme: &Theme,
    area: Rect,
    alignment: Alignment,
    block: Option<Block>,
    fg: Option<ratatui::style::Color>,
) {
    let mut paragraph = Paragraph::new(lines).alignment(alignment);
    if let Some(b) = block {
        paragraph = paragraph.block(b);
    }
    if let Some(fg_color) = fg {
        paragraph = paragraph.style(Style::default().fg(fg_color));
    } else {
        paragraph = paragraph.style(Style::default().fg(rgb_to_color(&theme.text)));
    }
    f.render_widget(paragraph, area);
}

pub fn get_dialog_popup_size(title: &str, hint: &str) -> (u16, u16) {
    let width = title.len().max(hint.len()) as u16 + 4;
    let height = 7;
    (width, height)
}

pub fn draw_dialog_popup(
    f: &mut Frame,
    theme: &Theme,
    area: Rect,
    popup_block: &Block,
    title: &str,
    hint: &str,
) {
    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Line::styled(
            title,
            Style::default().fg(rgb_to_color(&theme.popup_text)),
        )),
        Line::from(""),
        Line::from(Line::styled(
            hint,
            Style::default().fg(rgb_to_color(&theme.accent)),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(popup_text, popup_block.inner(area));
}
