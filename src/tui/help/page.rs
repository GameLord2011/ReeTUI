use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::app_state::AppState;
use crate::themes::{rgb_to_color, Theme};

fn create_animated_text<'a>(original_text: &Text<'a>, progress: usize) -> Text<'a> {
    let mut taken_chars = 0;
    let mut new_lines = vec![];

    for line in &original_text.lines {
        if taken_chars >= progress {
            break;
        }
        let mut new_spans = vec![];
        for span in &line.spans {
            let remaining_chars = progress - taken_chars;
            if remaining_chars == 0 {
                break;
            }
            let content = span.content.to_string();
            let take_len = content.len().min(remaining_chars);
            new_spans.push(Span::styled(content[..take_len].to_string(), span.style));
            taken_chars += take_len;
            if taken_chars >= progress {
                break;
            }
        }
        new_lines.push(Line::from(new_spans));
    }

    Text::from(new_lines)
}

pub fn render_help_page(frame: &mut Frame, app_state: &mut AppState, area: Rect) {
    let theme = app_state.current_theme.clone();

    let background =
        Block::default().style(Style::default().bg(rgb_to_color(&theme.colors.background)));
    frame.render_widget(background, area);

    let mut pages: Vec<fn(&mut Frame, &mut AppState, Rect, &Theme)> = Vec::new();

    if app_state.help_state.show_font_check_page {
        pages.push(render_font_check_page);
    }
    if app_state.help_state.show_chafa_check_page {
        pages.push(render_chafa_check_page);
    }
    pages.push(render_logo_page);
    pages.push(render_keyboard_page);
    pages.push(render_create_channel_page);
    pages.push(render_ctrl_u_page);
    pages.push(render_ctrl_d_page);
    pages.push(render_esc_page);

    app_state.help_state.total_pages = pages.len();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3), // For gauh and page %
        ])
        .split(area);

    let current_page_index = app_state.help_state.current_page;
    if let Some(render_fn) = pages.get(current_page_index) {
        render_fn(frame, app_state, layout[0], &theme);
    }

    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Gauh
            Constraint::Length(2), // page %
        ])
        .split(layout[1]);

    let ratio = if app_state.help_state.gauge_animation_active {
        let progress = app_state.help_state.gauge_animation_progress;
        // Ease-out cubic: t => 1 - pow(1 - t, 3)
        let eased_progress = 1.0 - (1.0 - progress).powi(3);
        app_state.help_state.gauge_animation_start_ratio
            + (app_state.help_state.gauge_animation_end_ratio
                - app_state.help_state.gauge_animation_start_ratio)
                * eased_progress
    } else if app_state.help_state.total_pages > 0 {
        (app_state.help_state.current_page + 1) as f64 / app_state.help_state.total_pages as f64
    } else {
        1.0
    };

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
        .ratio(ratio)
        .label("");
    frame.render_widget(gauge, bottom_chunks[0]);

    let page_indicator_text = format!(
        "Page {}/{}\n(Press Enter to continue)",
        app_state.help_state.current_page + 1,
        app_state.help_state.total_pages
    );
    let page_indicator = Paragraph::new(page_indicator_text)
        .style(Style::default().fg(rgb_to_color(&theme.colors.help_text)))
        .alignment(Alignment::Center);
    frame.render_widget(page_indicator, bottom_chunks[1]);
}

fn render_font_check_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw("A Nerd Font is recommended for the best experience, like u can't see the most of the icon without of it")),
        Line::from(Span::raw("")),
        Line::from(Span::raw("You can download one from:")),
        Line::from(Span::raw("https://www.nerdfonts.com/font-downloads")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_height),
            Constraint::Min(0),
        ])
        .split(area);

    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let text_box = Paragraph::new(animated_text)
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

fn render_chafa_check_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw(
            "'chafa' is required, so u can see those fancy gifs",
        )),
        Line::from(Span::raw("")),
        Line::from(Span::raw("You can download it from:")),
        Line::from(Span::raw("https://hpjansson.org/chafa/download/")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_height),
            Constraint::Min(0),
        ])
        .split(area);

    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let text_box = Paragraph::new(animated_text)
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

fn render_logo_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw(
            "Welcome to ReeTUI(also not ReeTOING)! This is a chat application",
        )),
        Line::from(Span::raw("built with Rust and Ratatui. Navigate through")),
        Line::from(Span::raw("the application using your keyboard.")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(text_height), // Height for text box
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
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(chunks[1]);
    let text_box = Paragraph::new(animated_text)
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
fn render_keyboard_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw("Settings are important! You can open the")),
        Line::from(Span::raw("settings page by pressing ")),
        Line::from(Span::styled(
            "Ctrl + S",
            Style::default().fg(rgb_to_color(&theme.colors.success_color)),
        )),
        Line::from(Span::raw(" from anywhere in the application.")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 4;

    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(text_height), // Height for text box
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
        "'",
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
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(text_box_area);
    let text_box = Paragraph::new(animated_text)
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
fn render_create_channel_page(
    frame: &mut Frame,
    app_state: &mut AppState,
    area: Rect,
    theme: &Theme,
) {
    let text_content = Text::from(vec![
        Line::from(Span::raw("You can create a new channel by pressing")),
        Line::from(Span::styled(
            "Ctrl + N",
            Style::default().fg(rgb_to_color(&theme.colors.success_color)),
        )),
        Line::from(Span::raw(" from the home page.")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 4;

    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(text_height), // Height for text box
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
                      is_ctrl_n_highlight: bool| {
        let style = if is_highlighted {
            Style::default()
                .fg(rgb_to_color(&theme.colors.background))
                .bg(rgb_to_color(&theme.colors.accent))
        } else if is_ctrl_n_highlight {
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
        "'",
        "Enter",
    ];
    let row3_widths = vec![10, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 10];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row3_widths.iter().sum()) / 2);
    for (i, key_str) in row3_keys.iter().enumerate() {
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row3_widths[i],
            false,
            false,
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
        let is_n_key = key_str == &"N";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row4_widths[i],
            false,
            is_n_key,
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
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(text_box_area);
    let text_box = Paragraph::new(animated_text)
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
fn render_ctrl_u_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw("File management is important!")),
        Line::from(Span::raw("")),
        Line::from(Span::raw(
            "Press 'Ctrl + U' to open the file manager for uploads.",
        )),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 4;

    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(text_height), // Height for text box
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
                      is_ctrl_highlight: bool,
                      is_u_highlight: bool| {
        let style = if is_ctrl_highlight {
            Style::default()
                .fg(rgb_to_color(&theme.colors.background))
                .bg(rgb_to_color(&theme.colors.accent))
        } else if is_u_highlight {
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
        "Esc",
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
        let is_u_key = key_str == &"U";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row2_widths[i],
            false,
            is_u_key,
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
        "'",
        "Enter",
    ];
    let row3_widths = vec![10, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 10];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row3_widths.iter().sum()) / 2);
    for (i, key_str) in row3_keys.iter().enumerate() {
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row3_widths[i],
            false,
            false,
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
        let is_ctrl_key = key_str == &"Ctrl";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row5_widths[i],
            is_ctrl_key,
            false,
        );
        current_x += row5_widths[i];
    }
    // Text box at the bottom
    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(text_box_area);
    let text_box = Paragraph::new(animated_text)
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
fn render_ctrl_d_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw("Press 'Ctrl + D' to view your downloads.")),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 4;

    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(text_height), // Height for text box
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
                      is_ctrl_highlight: bool,
                      is_d_highlight: bool| {
        let style = if is_ctrl_highlight {
            Style::default()
                .fg(rgb_to_color(&theme.colors.background))
                .bg(rgb_to_color(&theme.colors.accent))
        } else if is_d_highlight {
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
        "Esc",
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
        "'",
        "Enter",
    ];
    let row3_widths = vec![10, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 10];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row3_widths.iter().sum()) / 2);
    for (i, key_str) in row3_keys.iter().enumerate() {
        let is_d_key = key_str == &"D";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row3_widths[i],
            false,
            is_d_key,
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
        let is_ctrl_key = key_str == &"Ctrl";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row5_widths[i],
            is_ctrl_key,
            false,
        );
        current_x += row5_widths[i];
    }
    // Text box at the bottom
    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(text_box_area);
    let text_box = Paragraph::new(animated_text)
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
fn render_esc_page(frame: &mut Frame, app_state: &mut AppState, area: Rect, theme: &Theme) {
    let text_content = Text::from(vec![
        Line::from(Span::raw(
            "Press 'Esc' to close any popup, this will be ur how to exit key.",
        )),
        Line::from(Span::raw("")),
    ]);

    let animated_text = create_animated_text(
        &text_content,
        app_state.help_state.info_text_animation_progress,
    );

    let text_height = text_content.height() as u16 + 2;
    let text_width = text_content.width() as u16 + 4;

    let keyboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(15), // Height for keyboard
            Constraint::Percentage(10),
            Constraint::Length(text_height), // Height for text box
            Constraint::Percentage(10),
        ])
        .split(area);
    let keyboard_area = keyboard_layout[1];
    let text_box_area = keyboard_layout[3];
    let render_key =
        |f: &mut Frame, key_str: &str, x: u16, y: u16, key_width: u16, is_esc_highlight: bool| {
            let style = if is_esc_highlight {
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
        "Esc",
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
        let is_esc_key = key_str == &"Esc";
        render_key(
            frame,
            key_str,
            current_x,
            current_y,
            row1_widths[i],
            is_esc_key,
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
        render_key(frame, key_str, current_x, current_y, row2_widths[i], false);
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
        "'",
        "Enter",
    ];
    let row3_widths = vec![10, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 10];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row3_widths.iter().sum()) / 2);
    for (i, key_str) in row3_keys.iter().enumerate() {
        render_key(frame, key_str, current_x, current_y, row3_widths[i], false);
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
        render_key(frame, key_str, current_x, current_y, row4_widths[i], false);
        current_x += row4_widths[i];
    }
    current_y += 3;
    // Row 5: Ctrl, Alt, Space
    let row5_keys = vec!["Ctrl", "Alt", " ", "Alt", "Ctrl"];
    let row5_widths = vec![8, 8, 40, 8, 8];
    let mut current_x =
        keyboard_area.x + (keyboard_area.width.saturating_sub(row5_widths.iter().sum()) / 2);
    for (i, key_str) in row5_keys.iter().enumerate() {
        render_key(frame, key_str, current_x, current_y, row5_widths[i], false);
        current_x += row5_widths[i];
    }
    // Text box at the bottom
    let text_box_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(text_width),
            Constraint::Min(0),
        ])
        .split(text_box_area);
    let text_box = Paragraph::new(animated_text)
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
