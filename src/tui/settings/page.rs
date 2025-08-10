use crate::app::app_state::AppState;
use crate::themes::Theme;

use crate::tui::settings::state::{FocusedPane, SettingsScreen, SettingsState, QuitConfirmationState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

const SETTINGS_OPTIONS: &[&str] = &[" Themes", "󰞋 Help", "  Disconnect", "  Quit"];

const HELP_CONTENT: &[&str] = &[
    "Settings Navigation:",
    "  Left/Right Arrow: Switch between panes",
    "  Up/Down Arrow:    Navigate items in the current pane",
    "  Enter:            Select / Confirm an option",
    "  Esc:              Go back / Exit settings",
    "",
    "User Settings:",
    "  (in right pane, when User Settings is selected)",
    "  Username:         Type to change your username",
    "  Icon:             Use Left/Right arrows to change icon",
    "  Save:             Press Enter to save changes (feature in development)",
];

pub fn draw_settings_ui<B: ratatui::backend::Backend>(
    f: &mut Frame,
    settings_state: &mut SettingsState,
    theme: &Theme,
    app_state: &AppState,
    area: Rect, // This is now the popup_area
) {
    f.render_widget(Clear, area);
    let main_block = Block::default()
        .border_style(Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border)))
        .bg(crate::themes::rgb_to_color(&theme.colors.background));

    f.render_widget(main_block.clone(), area);

    let main_area = centered_rect(60, 60, area); // Use the passed area as the base for centering
    f.render_widget(main_block, main_area);

    let max_menu_item_width = SETTINGS_OPTIONS.iter().map(|s| s.len()).max().unwrap_or(0);
    // +2 for padding, +2 for borders (1 on each side)
    let left_pane_width = max_menu_item_width as u16 + 4;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Length(left_pane_width), Constraint::Min(0)])
        .split(main_area);

    draw_left_pane(f, settings_state, theme, chunks[0]);
    draw_right_pane::<B>(f, settings_state, theme, chunks[1], app_state);
}

fn draw_left_pane(f: &mut Frame, settings_state: &mut SettingsState, theme: &Theme, area: Rect) {
    let is_focused = settings_state.focused_pane == FocusedPane::Left;
    let border_style = if is_focused {
        Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border_focus))
    } else {
        Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border))
    };

    let menu_block = Block::default()
        .title("Menu")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(border_style)
        .bg(crate::themes::rgb_to_color(&theme.colors.background));
    f.render_widget(&menu_block, area);

    let inner_area = menu_block.inner(area);

    let item_height = 3; // 1 for content, 2 for borders
    let constraints: Vec<Constraint> = SETTINGS_OPTIONS
        .iter()
        .map(|_| Constraint::Length(item_height))
        .collect();

    let item_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (i, &name) in SETTINGS_OPTIONS.iter().enumerate() {
        let is_selected = i == settings_state.main_selection;
        let is_disabled = false; // Disconnect button is always enabled

        let item_style = if is_disabled {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.dim))
        } else if is_selected && is_focused {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.accent)).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border_focus))
        } else {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.text))
        };

        let border_style = if is_selected && is_focused {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.accent))
        } else if is_selected {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border_focus))
        } else {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border))
        };

        let item_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let paragraph = Paragraph::new(name)
            .style(item_style.bg(crate::themes::rgb_to_color(&theme.colors.background)))
            .alignment(Alignment::Center)
            .block(item_block.clone()); // Clone the block to apply background to it as well

        f.render_widget(paragraph, item_chunks[i]);
    }
}

fn draw_right_pane<B: ratatui::backend::Backend>(
    f: &mut Frame,
    settings_state: &mut SettingsState,
    theme: &Theme,
    area: Rect,
    app_state: &AppState,
) {
    let is_focused = settings_state.focused_pane == FocusedPane::Right;
    let border_style = if is_focused {
        Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border_focus))
    } else {
        Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border))
    };

    let block = Block::default()
        .title(SETTINGS_OPTIONS[settings_state.main_selection])
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(border_style)
        .bg(crate::themes::rgb_to_color(&theme.colors.background));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    match settings_state.screen {
        SettingsScreen::Themes => {
            draw_themes_pane::<B>(f, settings_state, theme, inner_area, app_state)
        }
        SettingsScreen::Help => draw_help_pane(f, theme, inner_area),
        SettingsScreen::Disconnect => draw_disconnect_pane(f, theme, inner_area),
        SettingsScreen::Quit => {
            if settings_state.quit_confirmation_state == QuitConfirmationState::Active {
                draw_quit_confirmation_pane(f, settings_state, theme, inner_area, app_state);
            } else {
                draw_quit_message_pane(f, theme, inner_area);
            }
        }
    }
}

fn draw_themes_pane<B: ratatui::backend::Backend>(
    f: &mut Frame,
    settings_state: &mut SettingsState,
    theme: &Theme,
    area: Rect,
    app_state: &AppState,
) {
    let theme_items: Vec<ListItem> = settings_state
        .themes
        .iter()
        .enumerate()
        .map(|(i, &theme_name)| {
            let theme_preview = app_state.themes.get(&theme_name).unwrap().clone();

            // Determine if this item is selected
            let is_selected = settings_state.theme_list_state.selected() == Some(i);

            // Conditional text colors for icon and name
            let icon_fg_color = if is_selected {
                crate::themes::rgb_to_color(&theme.colors.background) // Text color when selected
            } else {
                crate::themes::rgb_to_color(&theme.colors.text) // Normal text color
            };
            let name_fg_color = icon_fg_color; // Same color for name

            let icon_span = ratatui::text::Span::styled(
                theme_preview.icon.clone(),
                Style::default().fg(icon_fg_color),
            );
            let name_span = ratatui::text::Span::styled(
                format!(" {:?}", theme_name),
                Style::default().fg(name_fg_color),
            );

            // Calculate width of icon and name part
            let icon_name_width =
                theme_preview.icon.len() as u16 + format!(" {:?}", theme_name).len() as u16;

            let mut color_squares_spans: Vec<ratatui::text::Span> = Vec::new();
            let color_squares_rgb = vec![
                theme_preview.colors.background,
                theme_preview.colors.text,
                theme_preview.colors.accent,
                theme_preview.colors.border_focus,
                theme_preview.colors.error,
            ];

            for color_rgb in color_squares_rgb {
                color_squares_spans.push(ratatui::text::Span::styled(
                    "󱓻",
                    Style::default().fg(crate::themes::rgb_to_color(&color_rgb)),
                ));
                color_squares_spans.push(ratatui::text::Span::raw(" ")); // Small space between squares
            }

            let color_squares_total_width: u16 =
                color_squares_spans.iter().map(|s| s.width() as u16).sum();

            // Calculate available width for content (area.width - highlight_symbol_width)
            // Assuming highlight_symbol is " " which is 2 chars wide.
            let highlight_symbol_width = 2;
            let available_content_width = area.width.saturating_sub(highlight_symbol_width);

            // Calculate spacer width
            let fixed_content_width = icon_name_width + color_squares_total_width;
            let spacer_width = available_content_width.saturating_sub(fixed_content_width);
            let spacer_span = ratatui::text::Span::raw(" ".repeat(spacer_width as usize));

            let mut final_spans = vec![icon_span, name_span, spacer_span];
            final_spans.extend(color_squares_spans);

            ListItem::new(ratatui::text::Line::from(final_spans))
                .style(Style::default().bg(crate::themes::rgb_to_color(&theme.colors.background)))
        })
        .collect();

    let list = List::new(theme_items)
        .highlight_style(
            Style::default()
                .bg(crate::themes::rgb_to_color(&theme.colors.accent))
                .fg(crate::themes::rgb_to_color(&theme.colors.background))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ");

    f.render_stateful_widget(list, area, &mut settings_state.theme_list_state);
}

fn draw_help_pane(f: &mut Frame, theme: &Theme, area: Rect) {
    let help_text: Vec<ratatui::text::Line> = HELP_CONTENT.iter().map(|&s| s.into()).collect();
    let paragraph = Paragraph::new(help_text)
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.text))
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(
                    Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border)),
                ),
        );
    f.render_widget(paragraph, area);
}

fn draw_quit_message_pane(f: &mut Frame, theme: &Theme, area: Rect) {
    let text = "Are you sure you want to quit?";
    let p = Paragraph::new(text)
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.error))
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        )
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}

fn draw_quit_confirmation_pane(
    f: &mut Frame,
    _settings_state: &mut SettingsState, // No longer directly using settings_state
    theme: &Theme,
    area: Rect,
    app_state: &AppState, // Added app_state
) {
    let confirmation_block = Block::default()
        .title("Confirm Quit")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::themes::rgb_to_color(&theme.colors.border_focus)))
        .bg(crate::themes::rgb_to_color(&theme.colors.background));

    let inner_area = confirmation_block.inner(area); // Revert to original
    f.render_widget(confirmation_block, area); // Revert to original

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0), // Message
            Constraint::Length(3), // Buttons
        ])
        .split(inner_area);

    // Message
            let message = "Ya really want to get out?";
    let p = Paragraph::new(message)
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.text))
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        )
        .alignment(Alignment::Center);
    f.render_widget(p, chunks[0]);

    // Buttons
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    let ye_button_style = if app_state.quit_selection == 0 {
        Style::default()
            .fg(crate::themes::rgb_to_color(&theme.colors.button_text_active))
            .bg(crate::themes::rgb_to_color(&theme.colors.button_bg_active))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(crate::themes::rgb_to_color(&theme.colors.text))
            .bg(crate::themes::rgb_to_color(&theme.colors.button))
    };

    let no_button_style = if app_state.quit_selection == 1 {
        Style::default()
            .fg(crate::themes::rgb_to_color(&theme.colors.button_text_active))
            .bg(crate::themes::rgb_to_color(&theme.colors.button_bg_active))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(crate::themes::rgb_to_color(&theme.colors.text))
            .bg(crate::themes::rgb_to_color(&theme.colors.button))
    };

    let ye_button = Paragraph::new("Ye")
        .style(ye_button_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(ye_button, button_chunks[0]);

    let no_button = Paragraph::new("Hell no")
        .style(no_button_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(no_button, button_chunks[1]);
}

fn draw_disconnect_pane(f: &mut Frame, theme: &Theme, area: Rect) {
    let text = "Press Enter to disconnect.";
    let p = Paragraph::new(text)
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.text))
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        )
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
