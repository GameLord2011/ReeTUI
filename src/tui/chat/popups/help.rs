use crate::app::app_state::AppState;
use crate::tui::chat::popups::helpers::render_styled_paragraph;

use ratatui::{
    layout::{Alignment, Rect},
    text::Line,
    widgets::Block,
    Frame,
};

// Commands defined once as a static constant
static HELP_COMMANDS: &[&str] = &[
    " General:",
    "  Echap                - Open Quit popup (exit automaticly) 󰩈",
    "  Ctrl+S               - Open Settings popup ",
    "  Ctrl+N               - Open Create Channel popup ",
    "  Tab                  - Switch to next channel ",
    "  Ctrl+Up/Down         - Scroll messages ",
    "  Up/Down              - Switch channels 󰀙",
    "  Enter                - Send message ",
    "  Backspace            - Delete last char in input ",
    "",
    "Popups (varies per popup): 󱨇",
    "  Esc                  - Close popup / Cancel 󰈆",
    "  Enter                - Confirm / Select / Create ",
    "  Tab/Up/Down          - Navigate fields/options (in forms) 󰍍",
    "  Left/Right           - Select icon (in Create Channel) ",
    "  Q/q (Quit popup)     - Confirm quit ",
    "  Y/y (Deconn popup)   - Confirm deconnection ",
    "  N/n (Deconn popup)   - Cancel deconnection ",
    "  T/t (Settings)       - Open Themes ",
    "  D/d (Settings)       - Open Deconnection ",
    "  H/h (Settings)       - Open Help (this page) 󰞋",
];

pub fn get_help_popup_size() -> (u16, u16) {
    let height = HELP_COMMANDS.len() as u16 + 2 + 2; // +2 for borders, +2 for padding
    let width = HELP_COMMANDS.iter().map(|s| s.len()).max().unwrap_or(0) as u16 + 4; // +4 for borders
    (width, height)
}

pub fn draw_help_popup(f: &mut Frame, state: &mut AppState, area: Rect, popup_block: &Block) {
    let current_theme = &state.current_theme;
    let formatted_commands: Vec<Line> = HELP_COMMANDS.iter().map(|&s| Line::from(s)).collect();
    render_styled_paragraph(
        f,
        formatted_commands,
        &current_theme,
        popup_block.inner(area),
        Alignment::Left,
        Some(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 2, 2))),
        None,
    );
}
