use crate::themes::Theme;
use ratatui::{
    layout::Rect,
    style::{Style, Color},
    widgets::{List, ListItem, ListState},
    Frame,
};

pub fn render_styled_list(
    f: &mut Frame,
    items: &[&str],
    selected_index: Option<usize>,
    theme: &Theme,
    area: Rect,
    block: Option<ratatui::widgets::Block>,
    highlight_symbol: Option<String>,
) {
    let list_items: Vec<ListItem> = items
        .iter()
        .map(|&item| {
            ListItem::new(item).style(Style::default().fg(Color::Rgb(theme.colors.text.0, theme.colors.text.1, theme.colors.text.2)))
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

    let highlight_style = Style::default()
        .add_modifier(ratatui::style::Modifier::REVERSED)
        .fg(Color::Rgb(theme.colors.selected_icon.0, theme.colors.selected_icon.1, theme.colors.selected_icon.2));

    let symbol_ref = highlight_symbol.as_ref().map(|s| s.as_str());

    if let Some(symbol) = symbol_ref {
        list_widget = list_widget.highlight_symbol(symbol);
    }

    list_widget = list_widget.highlight_style(highlight_style);

    f.render_stateful_widget(list_widget, area, &mut list_state);
}
