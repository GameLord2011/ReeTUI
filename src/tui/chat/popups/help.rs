use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::tui::themes::{get_theme, rgb_to_color};

pub fn draw_help_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = get_theme(state.current_theme);
    let commands = vec![
        "General:",
        "  :                    - Open Quit popup",
        "  Ctrl+S               - Open Settings popup",
        "  Ctrl+N               - Open Create Channel popup",
        "  Tab                  - Switch to next channel",
        "  Ctrl+Up/Down         - Scroll messages",
        "  Up/Down              - Switch channels",
        "  Enter                - Send message",
        "  Backspace            - Delete last char in input",
        "",
        "Popups (varies per popup):",
        "  Esc                  - Close popup / Cancel",
        "  Enter                - Confirm / Select / Create",
        "  Tab/Up/Down          - Navigate fields/options (in forms)",
        "  Left/Right           - Select icon (in Create Channel)",
        "  Q/q (Quit popup)     - Confirm quit",
        "  Y/y (Deconn popup)   - Confirm deconnection",
        "  N/n (Deconn popup)   - Cancel deconnection",
        "  T/t (Settings)       - Open Themes",
        "  D/d (Settings)       - Open Deconnection",
        "  H/h (Settings)       - Open Help (this page)",
    ];

    let formatted_commands: Vec<Line> = commands
        .iter()
        .map(|&s|
            Line::from(Span::styled(
                s,
                Style::default().fg(rgb_to_color(&current_theme.text)),
            ))
        )
        .collect();

    let commands_paragraph = Paragraph::new(formatted_commands)
        .alignment(ratatui::layout::Alignment::Left)
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 2, 2)));

    f.render_widget(commands_paragraph, popup_block.inner(area));
}
