use crate::app::app_state::AppState;
use crate::app::PopupType;
use crate::themes::rgb_to_color;
use crate::tui::chat::ws_command::WsCommand;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::BorderType;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_downloads_popup(f: &mut Frame, app_state: &mut AppState) {
    let current_theme = &app_state.current_theme;
    let area = centered_rect(50, 60, f.area());

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
        .border_style(Style::default().fg(Color::Blue))
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
        let visible_height = inner_area.height / 3; // Assuming 3 lines per item
        let start_index = app_state.download_scroll_offset;
        let end_index =
            (start_index + visible_height as usize).min(app_state.downloadable_files.len());

        let mut current_y = 0;
        for i in start_index..end_index {
            let file = &app_state.downloadable_files[i];
            let is_selected = app_state.selected_download_index.selected() == Some(i);

            let item_area = Rect::new(inner_area.x, inner_area.y + current_y, inner_area.width, 3);
            current_y += 3;

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
                format!(
                    "{:.2} GB",
                    file.file_size as f64 / (1024.0 * 1024.0 * 1024.0)
                )
            };

            let file_block_border_style = if is_selected {
                Style::default().fg(rgb_to_color(&current_theme.colors.accent))
            } else {
                Style::default().fg(Color::Blue)
            };

            let file_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(file_block_border_style)
                .style(item_style);

            let inner_file_area = file_block.inner(item_area);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // For filename
                    Constraint::Percentage(30), // For size
                    Constraint::Percentage(30), // For sender
                ])
                .split(inner_file_area);

            let filename_paragraph = Paragraph::new(format!(
                "{} {}.{}",
                file.devicon, file.file_name, file.file_extension
            ))
            .alignment(Alignment::Left)
            .style(item_style);

            let size_paragraph = Paragraph::new(file_size_formatted)
                .alignment(Alignment::Center)
                .style(item_style);

            let sender_paragraph =
                Paragraph::new(format!("{} {}", file.sender_icon, file.sender_username))
                    .alignment(Alignment::Right)
                    .style(item_style);

            f.render_widget(file_block, item_area); // Render the block first
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

pub fn handle_downloads_popup_events(
    key: &KeyEvent,
    app_state: &mut AppState,
) -> Option<WsCommand> {
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
                    // Adjust scroll offset to keep selected item in view
                    if i < app_state.download_scroll_offset {
                        app_state.download_scroll_offset = i;
                    }
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
                    // Adjust scroll offset to keep selected item in view
                    // Assuming 3 lines per item and a visible area height of 15 (5 items)
                    let visible_items_count = 5; // This needs to be dynamic based on popup height
                    if i >= app_state.download_scroll_offset + visible_items_count {
                        app_state.download_scroll_offset = i - visible_items_count + 1;
                    }
                }
                None
            }
            KeyCode::PageUp => {
                if !app_state.downloadable_files.is_empty() {
                    let current_selection =
                        app_state.selected_download_index.selected().unwrap_or(0);
                    let new_selection = current_selection.saturating_sub(5); // Jump by 5 items
                    app_state
                        .selected_download_index
                        .select(Some(new_selection));
                    app_state.download_scroll_offset =
                        app_state.download_scroll_offset.saturating_sub(5);
                }
                None
            }
            KeyCode::PageDown => {
                if !app_state.downloadable_files.is_empty() {
                    let current_selection =
                        app_state.selected_download_index.selected().unwrap_or(0);
                    let new_selection =
                        (current_selection + 5).min(app_state.downloadable_files.len() - 1);
                    app_state
                        .selected_download_index
                        .select(Some(new_selection));
                    app_state.download_scroll_offset = (app_state.download_scroll_offset + 5)
                        .min(app_state.downloadable_files.len().saturating_sub(1));
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
