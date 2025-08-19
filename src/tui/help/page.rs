use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::app_state::AppState;
use crate::themes::{rgb_to_color, Theme};

pub fn render_help_page(frame: &mut Frame, app_state: &mut AppState, area: Rect) {
    let theme = &app_state.current_theme;
    let pages = [
        render_logo_page as fn(&mut Frame, Rect, &Theme),
        render_keyboard_page as fn(&mut Frame, Rect, &Theme),
        render_test_page as fn(&mut Frame, Rect, &Theme),
    ];

    let current_page_index = app_state.help_state.current_page;
    if let Some(render_fn) = pages.get(current_page_index) {
        render_fn(frame, area, theme);
    }

    // Render page indicator
    let page_indicator_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let page_indicator_text = format!(
        "Page {}/{} (Press Enter to continue)",
        current_page_index + 1,
        app_state.help_state.total_pages
    );
    let page_indicator = Paragraph::new(page_indicator_text)
        .style(Style::default().fg(rgb_to_color(&theme.colors.help_text)))
        .alignment(Alignment::Center);
    frame.render_widget(page_indicator, page_indicator_layout[1]);
}

fn render_logo_page(frame: &mut Frame, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(6), // Height for text box
            Constraint::Percentage(50),
        ])
        .split(area);

    // ReeTUI Logo (Placeholder for now)
    let logo_text = Text::from(vec![
        Line::from(Span::styled(
            r"",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
        Line::from(Span::styled(
            r"",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
        Line::from(Span::styled(
            r"Not ReeTOING",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
        Line::from(Span::styled(
            r"",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
        Line::from(Span::styled(
            r"",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
        Line::from(Span::styled(
            r"                             ",
            Style::default().fg(rgb_to_color(&theme.colors.accent)),
        )),
    ]);
    let logo_paragraph = Paragraph::new(logo_text).alignment(Alignment::Center);
    frame.render_widget(logo_paragraph, chunks[0]);

    // Text box at the bottom
    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Max(40), Constraint::Min(0)])
        .split(chunks[1]);

    let text_content = Text::from(vec![
        Line::from(Span::raw("Welcome to ReeTUI! This is a chat application")),
        Line::from(Span::raw("built with Rust and Ratatui. Navigate through")),
        Line::from(Span::raw("the application using your keyboard.")),
        Line::from(Span::raw("")),
        Line::from(Span::raw("Press 'Enter' to continue...")),
    ]);

    let text_box = Paragraph::new(text_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(rgb_to_color(&theme.colors.border))),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(rgb_to_color(&theme.colors.instructions_text)));
    frame.render_widget(text_box, text_box_layout[1]);
}

fn render_keyboard_page(frame: &mut Frame, area: Rect, theme: &Theme) {
    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(6), // Height for text box
            Constraint::Percentage(10),
        ])
        .split(area);

    let keyboard_area = keyboard_layout[1];
    let text_box_area = keyboard_layout[3];

    let render_key = |f: &mut Frame,
                      key_str: &str,
                      x: u16,
                      y: u16,
                      key_width: u16,
                      is_highlighted: bool,
                      is_ctrl_s_highlight: bool| {
        let style = if is_highlighted {
            Style::default()
                .fg(rgb_to_color(&theme.colors.background))
                .bg(rgb_to_color(&theme.colors.accent))
        } else if is_ctrl_s_highlight {
            Style::default()
                .fg(rgb_to_color(&theme.colors.background))
                .bg(rgb_to_color(&theme.colors.success_color))
        } else {
            Style::default().fg(rgb_to_color(&theme.colors.text))
        };

        let key_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let key_height = 3;

        let key_area = Rect::new(x, y, key_width, key_height);
        let key_paragraph = Paragraph::new(Text::from(key_str))
            .alignment(Alignment::Center)
            .style(style)
            .block(key_block);
        f.render_widget(key_paragraph, key_area);
    };

    let mut current_y = keyboard_area.y;

    // Row 1: Numbers and Symbols
    let row1_keys = vec![
        "~",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "0",
        "-",
        "=",
        "Backspace",
    ];
    let row1_widths = vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 12];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row1_widths.iter().sum()) / 2);
    for (i, key_str) in row1_keys.iter().enumerate() {
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row1_widths[i],
            false,
            false,
        );
        current_x += row1_widths[i];
    }

    current_y += 3;

    // Row 2: QWERTY
    let row2_keys = vec![
        "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "\\",
    ];
    let row2_widths = vec![8, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 8];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row2_widths.iter().sum()) / 2);
    for (i, key_str) in row2_keys.iter().enumerate() {
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row2_widths[i],
            false,
            false,
        );
        current_x += row2_widths[i];
    }

    current_y += 3;

    // Row 3: ASDFGHJKL
    let row3_keys = vec![
        "Caps Lock ",
        "A",
        "S",
        "D",
        "F",
        "G",
        "H",
        "J",
        "K",
        "L",
        ";",
        "\'",
        "Enter",
    ];
    let row3_widths = vec![10, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 10];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row3_widths.iter().sum()) / 2);
    for (i, key_str) in row3_keys.iter().enumerate() {
        let is_s_key = key_str == &"S";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row3_widths[i],
            false,
            is_s_key,
        );
        current_x += row3_widths[i];
    }

    current_y += 3;

    // Row 4: ZXCVBNM
    let row4_keys = vec![
        "Shift", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "Shift",
    ];
    let row4_widths = vec![12, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 12];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row4_widths.iter().sum()) / 2);
    for (i, key_str) in row4_keys.iter().enumerate() {
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row4_widths[i],
            false,
            false,
        );
        current_x += row4_widths[i];
    }

    current_y += 3;

    // Row 5: Ctrl, Alt, Space
    let row5_keys = vec!["Ctrl", "Alt", " ", "Alt", "Ctrl"];
    let row5_widths = vec![8, 8, 40, 8, 8];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row5_widths.iter().sum()) / 2);
    for (i, key_str) in row5_keys.iter().enumerate() {
        let is_ctrl_key = key_str == &"Ctrl" && i == 0; // Only highlight the first Ctrl
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row5_widths[i],
            false,
            is_ctrl_key,
        );
        current_x += row5_widths[i];
    }

    // Text box at the bottom
    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(text_box_area);

    let text_content = Text::from(vec![
        Line::from(Span::raw("Settings are important! You can open the")),
        Line::from(Span::raw("settings page by pressing ")),
        Line::from(Span::styled(
            "Ctrl + S",
            Style::default().fg(rgb_to_color(&theme.colors.success_color)),
        )),
        Line::from(Span::raw(" from anywhere in the application.")),
        Line::from(Span::raw("")),
        Line::from(Span::raw("Press 'Enter' to continue...")),
    ]);

    let text_box = Paragraph::new(text_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Info")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(rgb_to_color(&theme.colors.border))),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(rgb_to_color(&theme.colors.instructions_text)));
    frame.render_widget(text_box, text_box_layout[1]);
}

fn render_test_page(frame: &mut Frame, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20), // Space above custom ASCII art
            Constraint::Length(15),     // Height for custom ASCII art
            Constraint::Percentage(20), // Space between custom and multicolor
            Constraint::Length(7),      // Height for multicolor ASCII art
            Constraint::Percentage(40), // Space below multicolor ASCII art
        ])
        .split(area);

    // ASCII art with gradient
    let ascii_art = vec![
        Line::from(vec![
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.error)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.warning_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.accent)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.success_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.info_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.loading_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.dimmed_icon)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.error)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.warning_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.accent)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.success_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.info_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.loading_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.dimmed_icon)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.error)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.warning_color)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.accent)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.success_color)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.info_color)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.loading_color)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.dimmed_icon)),
            ),
            Span::styled(
                "██  ██  ██",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.error)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.warning_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.accent)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.success_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.info_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.loading_color)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.dimmed_icon)),
            ),
            Span::styled(
                "██      ██",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.error)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.warning_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.accent)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.success_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.info_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.loading_color)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.dimmed_icon)),
            ),
            Span::styled(
                "████████",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ),
        ]),
    ];

    let user_ascii_art = Text::from(vec![
        Line::from(Span::raw(
            r" /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\ ",
        )),
        Line::from(Span::raw(
            r"( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )",
        )),
        Line::from(Span::raw(
            r" > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ < ",
        )),
        Line::from(Span::raw(
            r" /\_/\    ██████╗████████╗██████╗ ██╗         ███████╗   /\_/\ ",
        )),
        Line::from(Span::raw(
            r"( o.o )  ██╔════╝╚══██╔══╝██╔══██╗██║         ██╔════╝  ( o.o )",
        )),
        Line::from(Span::raw(
            r" > ^ <   ██║        ██║   ██████╔╝██║         ███████╗   > ^ < ",
        )),
        Line::from(Span::raw(
            r" /\_/\   ██║        ██║   ██╔══██╗██║         ╚════██║   /\_/\ ",
        )),
        Line::from(Span::raw(
            r"( o.o )  ╚██████╗   ██║   ██║  ██║███████╗    ███████║  ( o.o )",
        )),
        Line::from(Span::raw(
            r" > ^ <    ╚═════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝    ╚══════╝   > ^ < ",
        )),
        Line::from(Span::raw(
            r" /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\  /\_/\ ",
        )),
        Line::from(Span::raw(
            r"( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )( o.o )",
        )),
        Line::from(Span::raw(
            r" > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ <  > ^ < ",
        )),
    ]);
    let user_ascii_paragraph = Paragraph::new(user_ascii_art).alignment(Alignment::Center);
    frame.render_widget(user_ascii_paragraph, chunks[1]);

    let ascii_paragraph = Paragraph::new(ascii_art).alignment(Alignment::Center);
    frame.render_widget(ascii_paragraph, chunks[3]);
}
