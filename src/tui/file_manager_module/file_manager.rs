use crate::app::AppState;
use crate::themes::rgb_to_color;
use crossterm::event::{KeyCode, KeyEvent};
use devicons::{icon_for_file, FileIcon, Theme};
use ratatui::{
    prelude::*,
    widgets::Block,
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
}

impl FileItem {
    fn new(path: PathBuf, is_dir: bool) -> Self {
        let icon = icon_for_file(path.as_path(), Some(Theme::Dark));
        Self {
            path,
            is_dir,
            icon,
            children: Vec::new(),
            expanded: false,
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

        let mut root = FileItem::new(current_path, true);
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
                    FileItem::new(path, is_dir)
                })
                .collect();
            children.sort_by(|a, b| (b.is_dir, &a.path).cmp(&(a.is_dir, &b.path)));
            item.children = children;
        }
    }

    pub fn ui(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.current_theme;
        let block = Block::default().bg(rgb_to_color(&theme.colors.background));
        f.render_widget(block, area);

        let mut lines = Vec::new();
        self.displayed_items.clear();
        let mut displayed_items = Vec::new();
        self.render_tree(&self.tree, &mut lines, 0, &mut displayed_items);
        self.displayed_items = displayed_items;

        let list_height = area.height as usize;
        let start_index = self.selected_index.saturating_sub(list_height / 2).min(self.displayed_items.len().saturating_sub(list_height));


        for (i, line) in lines.iter().enumerate().skip(start_index).take(list_height) {
            let y = area.y + (i - start_index) as u16;
            let is_selected = i == self.selected_index;
            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            f.buffer_mut().set_line(area.x, y, line, area.width);
            if is_selected {
                f.buffer_mut().set_style(Rect::new(area.x, y, area.width, 1), style);
            }
        }
    }

    fn render_tree<'a>(
        &self,
        item: &'a FileItem,
        lines: &mut Vec<Line<'a>>,
        depth: usize,
        displayed_items: &mut Vec<PathBuf>,
    ) {
        let prefix = "  ".repeat(depth);
        let connector = if item.is_dir {
            if item.expanded { "▼" } else { "▶" }
        } else {
            " "
        };

        let color = u32::from_str_radix(item.icon.color.trim_start_matches('#'), 16).unwrap_or(0xFFFFFF);
        let icon_style = Style::default().fg(Color::from_u32(color));
        let file_name = item.path.file_name().unwrap_or_default().to_string_lossy();

        let line = Line::from(vec![
            Span::raw(prefix),
            Span::raw(connector),
            Span::raw(" "),
            Span::styled(item.icon.icon.to_string(), icon_style),
            Span::raw(" "),
            Span::raw(file_name.to_string()),
        ]);
        lines.push(line);
        displayed_items.push(item.path.clone());

        if item.expanded {
            for child in &item.children {
                self.render_tree(child, lines, depth + 1, displayed_items);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> FileManagerEvent {
        match key.code {
            KeyCode::Up => self.select_previous(),
            KeyCode::Down => self.select_next(),
            KeyCode::Left => self.collapse_dir(),
            KeyCode::Right => self.expand_dir(),
            KeyCode::Enter => {
                if let Some(item) = self.get_selected_item_mut() {
                    if item.is_dir {
                        item.expanded = !item.expanded;
                        if item.expanded {
                            Self::read_dir(item);
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

    fn collapse_dir(&mut self) {
        if let Some(item) = self.get_selected_item_mut() {
            if item.is_dir && item.expanded {
                item.expanded = false;
            }
        }
    }
}
