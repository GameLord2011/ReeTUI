use crate::app::AppState;
use crate::themes::rgb_to_color;
use crossterm::event::{KeyCode, KeyEvent};
use devicons::{icon_for_file, FileIcon, Theme};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Padding},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::mpsc;

pub enum FileManagerEvent {
    FileSelectedForUpload(PathBuf),
    CloseFileManager,
    None,
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub is_dir: bool,
    pub icon: FileIcon,
    pub children: Vec<FileItem>,
    pub expanded: bool,
    pub is_parent_nav: bool,
}

impl FileItem {
    fn new(path: PathBuf, is_dir: bool, is_parent_nav: bool) -> Self {
        let icon = icon_for_file(path.as_path(), &Some(Theme::Dark));
        Self {
            path,
            is_dir,
            icon,
            children: Vec::new(),
            expanded: false,
            is_parent_nav,
        }
    }
}

#[derive(Debug)]
pub struct FileManager {
    pub tree: FileItem,
    pub selected_index: usize,
    pub displayed_items: Vec<PathBuf>,
    pub redraw_tx: mpsc::UnboundedSender<String>,
    pub app_state: Arc<tokio::sync::Mutex<AppState>>,
}

impl FileManager {
    pub fn new(
        redraw_tx: mpsc::UnboundedSender<String>,
        app_state_param: Arc<tokio::sync::Mutex<AppState>>,
    ) -> Self {
        let current_path = dirs::home_dir().unwrap_or_else(|| {
            log::warn!("Could not determine home directory, falling back to current directory.");
            env::current_dir().unwrap_or_else(|e| {
                log::error!("Could not determine current directory: {}", e);
                PathBuf::from("/")
            })
        });

        let mut root = FileItem::new(current_path, true, false);
        Self::read_dir(&mut root);
        root.expanded = true;

        Self {
            tree: root,
            selected_index: 0,
            displayed_items: Vec::new(),
            redraw_tx,
            app_state: app_state_param,
        }
    }

    fn read_dir(item: &mut FileItem) {
        if !item.is_dir || !item.children.is_empty() {
            return;
        }

        if let Ok(entries) = fs::read_dir(&item.path) {
            let mut children: Vec<FileItem> = entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    FileItem::new(path, is_dir, false)
                })
                .collect();
            children.sort_by(|a, b| (b.is_dir, &a.path).cmp(&(a.is_dir, &b.path)));

            // Add ".." for parent directory if not at root
            if let Some(parent_path) = item.path.parent() {
                if parent_path != item.path {
                    children.insert(0, FileItem::new(parent_path.to_path_buf(), true, true));
                }
            }
            item.children = children;
        }
    }

    pub fn ui(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.current_theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)].as_ref())
            .split(area);

        let left_area = chunks[0];
        let right_area = chunks[1];

        // File tree block
        let file_tree_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
            .padding(Padding::new(0, 0, 0, 0));
        f.render_widget(file_tree_block.clone(), left_area);
        let inner_left_area = file_tree_block.inner(left_area);

        // File tree rendering
        let mut lines = Vec::new();
        self.displayed_items.clear();
        let mut displayed_items = Vec::new();
        for child in &self.tree.children {
            self.render_tree(child, &mut lines, 0, &mut displayed_items, theme);
        }
        self.displayed_items = displayed_items;

        let list_height = inner_left_area.height as usize;
        let start_index = self
            .selected_index
            .saturating_sub(list_height / 2)
            .min(self.displayed_items.len().saturating_sub(list_height));

        for (i, line) in lines.iter().enumerate().skip(start_index).take(list_height) {
            let y = inner_left_area.y + (i - start_index) as u16;
            let is_selected = i == self.selected_index;
            let style = if is_selected {
                Style::default().fg(rgb_to_color(&theme.colors.background)).bg(rgb_to_color(&theme.colors.accent))
            } else {
                Style::default().fg(Color::White)
            };
            f.buffer_mut()
                .set_line(inner_left_area.x, y, line, inner_left_area.width);
            if is_selected {
                f.buffer_mut()
                    .set_style(Rect::new(inner_left_area.x, y, inner_left_area.width, 1), style);
            }
        }

        // Preview pane block
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
            .padding(Padding::new(0, 0, 0, 0));
        f.render_widget(preview_block.clone(), right_area);
        let inner_right_area = preview_block.inner(right_area);

        let selected_item = self.get_selected_item();
        let preview_text = if let Some(item) = selected_item {
            if item.is_dir {
                "Select a file to preview".to_string()
            } else {
                match fs::read_to_string(&item.path) {
                    Ok(content) => content,
                    Err(_) => "Cannot preview this file type (might be binary).".to_string(),
                }
            }
        } else {
            "No item selected".to_string()
        };

        let paragraph = Paragraph::new(preview_text)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, inner_right_area);
    }

    fn render_tree<'a>(
        &self,
        item: &'a FileItem,
        lines: &mut Vec<Line<'a>>,
        depth: usize,
        displayed_items: &mut Vec<PathBuf>,
        theme: &crate::themes::Theme,
    ) {
        let prefix = "  ".repeat(depth);
        let icon_span = if item.is_dir {
            let folder_icon = if item.expanded { "" } else { "" };
            let folder_style = Style::default().fg(Color::White);
            Span::styled(folder_icon.to_string(), folder_style)
        } else {
            let icon_str = item.icon.icon.to_string();
            let file_icon = if icon_str == "*" {
                "".to_string() // Default file icon if devicon is '*'
            } else {
                icon_str
            };
            let color =
                u32::from_str_radix(item.icon.color.trim_start_matches('#'), 16).unwrap_or(0xFFFFFF);
            Span::styled(file_icon, Style::default().fg(Color::from_u32(color)))
        };

        let file_name = if item.is_parent_nav {
            "..".to_string()
        } else {
            item.path.file_name().unwrap_or_default().to_string_lossy().to_string()
        };
        let file_style = Style::default().fg(rgb_to_color(&theme.colors.text));

        lines.push(Line::from(vec![
            Span::raw(prefix.clone()),
            icon_span.clone(),
            Span::raw(" "),
            Span::styled(file_name.to_string(), file_style),
        ]));
        displayed_items.push(item.path.clone());

        if item.expanded {
            for child in &item.children {
                self.render_tree(child, lines, depth + 1, displayed_items, theme);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> FileManagerEvent {
        match key.code {
            KeyCode::Up => self.select_previous(),
            KeyCode::Down => self.select_next(),
            KeyCode::Right => self.expand_dir(),
            KeyCode::Enter => {
                if let Some(item) = self.get_selected_item_mut() {
                    if item.is_dir {
                        if item.is_parent_nav {
                            // Handle ".." navigation
                            if let Some(parent_path) = item.path.parent() {
                                self.tree = FileItem::new(parent_path.to_path_buf(), true, true);
                                Self::read_dir(&mut self.tree);
                                self.tree.expanded = true;
                                self.selected_index = 0;
                            }
                        } else { // It's a regular directory, not ".."
                            let new_root_path = item.path.clone(); // Get the path of the selected directory
                            self.tree = FileItem::new(new_root_path, true, false); // Make it the new root
                            Self::read_dir(&mut self.tree); // Read its contents
                            self.tree.expanded = true; // Ensure it's expanded
                            self.selected_index = 0; // Reset selection to the top (which will be ".." if present)
                        }
                    } else {
                        return FileManagerEvent::FileSelectedForUpload(item.path.clone());
                    }
                }
            }
            KeyCode::Esc => return FileManagerEvent::CloseFileManager,
            _ => {}
        }
        FileManagerEvent::None
    }

    fn get_selected_item_mut(&mut self) -> Option<&mut FileItem> {
        if self.selected_index >= self.displayed_items.len() {
            return None;
        }
        let path = &self.displayed_items[self.selected_index].clone();
        Self::find_item_mut(&mut self.tree, path)
    }

    fn get_selected_item(&self) -> Option<&FileItem> {
        if self.selected_index >= self.displayed_items.len() {
            return None;
        }
        let path = &self.displayed_items[self.selected_index];
        Self::find_item(&self.tree, path)
    }
    
    fn find_item_mut<'a>(item: &'a mut FileItem, path: &Path) -> Option<&'a mut FileItem> {
        if item.path == path {
            return Some(item);
        }
        if item.expanded {
            for child in &mut item.children {
                if let Some(found) = Self::find_item_mut(child, path) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn find_item<'a>(item: &'a FileItem, path: &Path) -> Option<&'a FileItem> {
        if item.path == path {
            return Some(item);
        }
        if item.expanded {
            for child in &item.children {
                if let Some(found) = Self::find_item(child, path) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn select_next(&mut self) {
        if self.selected_index < self.displayed_items.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn expand_dir(&mut self) {
        if let Some(item) = self.get_selected_item_mut() {
            if item.is_dir && !item.expanded {
                Self::read_dir(item);
                item.expanded = true;
            }
        }
    }

    
}
