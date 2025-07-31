use crate::app::app_state::AppState;
use crate::themes::Theme;
use crate::tui::auth::page::ICONS;
use crate::tui::settings::state::{FocusedPane, SettingsScreen, SettingsState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

const SETTINGS_OPTIONS: &[&str] = &[" Themes", "󰞋 Help", " User Settings", "  Quit"];

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

    let main_area = centered_rect(80, 80, area); // Use the passed area as the base for centering
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
        let is_disabled = i == 2 && !settings_state.is_user_logged_in();

        let item_style = if is_disabled {
            Style::default().fg(crate::themes::rgb_to_color(&theme.colors.dim))
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
        SettingsScreen::UserSettings => {
            draw_user_settings_pane::<B>(f, settings_state, theme, inner_area, app_state)
        }
        SettingsScreen::Quit => draw_quit_pane(f, theme, inner_area),
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

fn draw_user_settings_pane<B: ratatui::backend::Backend>(
    f: &mut Frame,
    settings_state: &mut SettingsState,
    theme: &Theme,
    area: Rect,
    _app_state: &AppState,
) {
    if !settings_state.is_user_logged_in() {
        let message = "You must be logged in to change user settings.";
        let paragraph = Paragraph::new(message)
            .style(
                Style::default()
                    .fg(crate::themes::rgb_to_color(&theme.colors.dim))
                    .bg(crate::themes::rgb_to_color(&theme.colors.background)),
            )
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(area);

    // Username Input
    let username_block = Block::default()
        .title("Username")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(crate::themes::rgb_to_color(
            &theme.colors.input_border_inactive,
        )))
        .bg(crate::themes::rgb_to_color(&theme.colors.background));
    let username_p = Paragraph::new(settings_state.new_username.as_str())
        .block(username_block)
        .style(Style::default().bg(crate::themes::rgb_to_color(&theme.colors.background)));
    f.render_widget(username_p, chunks[0]);

    // Icon Selector
    let icon_block = Block::default()
        .title("Icon")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(crate::themes::rgb_to_color(
            &theme.colors.input_border_inactive,
        )))
        .bg(crate::themes::rgb_to_color(&theme.colors.background));

    let current_icon_index = ICONS
        .iter()
        .position(|&i| i == settings_state.new_icon)
        .unwrap_or(0);
    let icon_p = Paragraph::new(ICONS[current_icon_index])
        .block(icon_block)
        .alignment(Alignment::Center)
        .style(Style::default().bg(crate::themes::rgb_to_color(&theme.colors.background)));
    f.render_widget(icon_p, chunks[1]);

    // Save Button
    let save_button_block = Block::default()
        .title("Save")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(crate::themes::rgb_to_color(
            &theme.colors.button_border_active,
        )))
        .bg(crate::themes::rgb_to_color(&theme.colors.background));
    let save_button_p = Paragraph::new("Save Changes")
        .block(save_button_block)
        .alignment(Alignment::Center)
        .style(Style::default().bg(crate::themes::rgb_to_color(&theme.colors.background)));
    f.render_widget(save_button_p, chunks[2]);

    let hint = Paragraph::new("NOTE: User settings are not yet saved to the server.")
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.dim))
                .italic()
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        )
        .alignment(Alignment::Center);
    f.render_widget(hint, chunks[4]);
}

fn draw_quit_pane(f: &mut Frame, theme: &Theme, area: Rect) {
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
