use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Clear, Table, Row, Cell, TableState},
    style::{Modifier, Style},
    text::Text,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use crate::tui::chat::ws_command::WsCommand;
use crate::app::PopupType;
use crate::app::app_state::AppState;
use crate::themes::rgb_to_color;
use ratatui::widgets::BorderType;

pub fn draw_downloads_popup(f: &mut Frame, app_state: &mut AppState) {
    let current_theme = &app_state.current_theme;
    let area = centered_rect(60, 60, f.area());

    // Clear the area with the background color first
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().bg(rgb_to_color(&current_theme.colors.background)),
        area,
    );

    let block = Block::default()
        .title("Downloads")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb_to_color(&current_theme.colors.text)))
        .bg(rgb_to_color(&current_theme.colors.background)); // Set background on the block as well

    f.render_widget(block.clone(), area);

    let inner_area = block.inner(area);

    if app_state.downloadable_files.is_empty() {
        let no_downloads_message = "No downloads available.";
        let paragraph = Paragraph::new(no_downloads_message)
            .block(Block::default())
            .alignment(Alignment::Center);
        f.render_widget(paragraph, inner_area);
    } else {
        let file_constraints: Vec<Constraint> = app_state.downloadable_files.iter().map(|_| Constraint::Length(3)).collect();
        let file_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(file_constraints)
            .split(inner_area);

        for (i, file) in app_state.downloadable_files.iter().enumerate() {
            let is_selected = app_state.selected_download_index.selected() == Some(i);
            let item_style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&current_theme.colors.accent))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.dim))
            };

            let file_size_formatted = if file.file_size < 1024 {
                format!("{} B", file.file_size)
            } else if file.file_size < 1024 * 1024 {
                format!("{:.2} KB", file.file_size as f64 / 1024.0)
            } else if file.file_size < 1024 * 1024 * 1024 {
                format!("{:.2} MB", file.file_size as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.2} GB", file.file_size as f64 / (1024.0 * 1024.0 * 1024.0))
            };

            let file_block_border_style = if is_selected {
                Style::default().fg(rgb_to_color(&current_theme.colors.accent))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.dim))
            };

            let file_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(file_block_border_style)
                .style(item_style);

            let inner_file_area = file_block.inner(file_layout[i]);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // For filename
                    Constraint::Percentage(30), // For size
                    Constraint::Percentage(30), // For sender
                ])
                .split(inner_file_area);

            let filename_paragraph = Paragraph::new(format!("{} {}", file.devicon, file.file_name))
                .alignment(Alignment::Left)
                .style(item_style);

            let size_paragraph = Paragraph::new(file_size_formatted)
                .alignment(Alignment::Center)
                .style(item_style);

            let sender_paragraph = Paragraph::new(format!("{} {}", file.sender_icon, file.sender_username))
                .alignment(Alignment::Right)
                .style(item_style);

            f.render_widget(file_block, file_layout[i]); // Render the block first
            f.render_widget(filename_paragraph, chunks[0]);
            f.render_widget(size_paragraph, chunks[1]);
            f.render_widget(sender_paragraph, chunks[2]);
        }
    }
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

pub fn handle_downloads_popup_events(key: &KeyEvent, app_state: &mut AppState) -> Option<WsCommand> {
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Up => {
                if !app_state.downloadable_files.is_empty() {
                    let i = match app_state.selected_download_index.selected() {
                        Some(i) => {
                            if i == 0 {
                                app_state.downloadable_files.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    app_state.selected_download_index.select(Some(i));
                }
                None
            }
            KeyCode::Down => {
                if !app_state.downloadable_files.is_empty() {
                    let i = match app_state.selected_download_index.selected() {
                        Some(i) => {
                            if i >= app_state.downloadable_files.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    app_state.selected_download_index.select(Some(i));
                }
                None
            }
            KeyCode::Enter => {
                if let Some(selected_index) = app_state.selected_download_index.selected() {
                    if let Some(file) = app_state.downloadable_files.get(selected_index) {
                        app_state.popup_state.show = false;
                        app_state.popup_state.popup_type = PopupType::None;
                        return Some(WsCommand::DownloadFile {
                            file_id: file.file_id.clone(),
                            file_name: file.file_name.clone(),
                        });
                    }
                }
                None
            }
            KeyCode::Esc => {
                app_state.popup_state.show = false;
                app_state.popup_state.popup_type = PopupType::None;
                None
            }
            _ => None,
        }
    } else {
        None
    }
}
