use std::{
    env,
    path::PathBuf,
};

use crate::tui::chat::utils::{centered_rect_with_size, centered_rect_with_size_and_padding};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

pub enum FileManagerMode {
    LocalUpload,
    RemoteDownload,
}

pub enum FileManagerEvent {
    FileSelectedForUpload(PathBuf),
    FileSelectedForDownload(String, String),
    CloseFileManager,
    None,
}

#[derive(Debug, Clone)]
pub struct DownloadableFile {
    pub id: String,
    pub name: String,
}

pub struct FileManager {
    pub current_path: PathBuf,
    pub local_files: Vec<PathBuf>,
    pub remote_files: Vec<DownloadableFile>,
    pub list_state: ListState,
    pub mode: FileManagerMode,
}

impl FileManager {
    pub fn new(mode: FileManagerMode, remote_files: Vec<DownloadableFile>) -> Self {
        let current_path = env::current_dir().unwrap_or_default();
        let mut file_manager = Self {
            current_path,
            local_files: Vec::new(),
            remote_files,
            list_state: ListState::default(),
            mode,
        };

        match file_manager.mode {
            FileManagerMode::LocalUpload => file_manager.read_current_dir(),
            FileManagerMode::RemoteDownload => { /* remote files are passed in */ }
        }

        file_manager.list_state.select(Some(0));
        file_manager
    }

    pub fn ui(&mut self, f: &mut Frame) {
        let popup_area = centered_rect_with_size(90, 90, f.area());
        let block_title = match self.mode {
            FileManagerMode::LocalUpload => "Upload Your Treasures ",
            FileManagerMode::RemoteDownload => "Download Your Loot ",
        };
        let block = Block::default()
            .title(block_title)
            .borders(Borders::ALL)
            .style(Style::default());
        f.render_widget(Clear, popup_area);
        f.render_widget(block, popup_area);

        let list_items: Vec<ListItem> = match self.mode {
            FileManagerMode::LocalUpload => self
                .local_files
                .iter()
                .map(|path| {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    ListItem::new(file_name)
                })
                .collect(),
            FileManagerMode::RemoteDownload => self
                .remote_files
                .iter()
                .map(|file| ListItem::new(file.name.clone()))
                .collect(),
        };

        let list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let list_area = centered_rect_with_size_and_padding(85, 85, 5, 5, popup_area);
        f.render_stateful_widget(list, list_area, &mut self.list_state);

        if let Some(_selected) = self.list_state.selected() {
            match self.mode {
                FileManagerMode::LocalUpload => {
                    // No preview in file manager
                }
                FileManagerMode::RemoteDownload => {
                    // No preview for remote files yet
                }
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> FileManagerEvent {
        match key.code {
            KeyCode::Up => {
                self.select_previous();
                FileManagerEvent::None
            }
            KeyCode::Down => {
                self.select_next();
                FileManagerEvent::None
            }
            KeyCode::Enter => match self.mode {
                FileManagerMode::LocalUpload => {
                    if let Some(selected) = self.list_state.selected() {
                        let path = &self.local_files[selected];
                        if path.is_dir() {
                            self.current_path = path.clone();
                            self.read_current_dir();
                            self.list_state.select(Some(0));
                            FileManagerEvent::None
                        } else {
                            FileManagerEvent::FileSelectedForUpload(path.clone())
                        }
                    } else {
                        FileManagerEvent::None
                    }
                }
                FileManagerMode::RemoteDownload => {
                    if let Some(selected) = self.list_state.selected() {
                        let file = &self.remote_files[selected];
                        FileManagerEvent::FileSelectedForDownload(file.id.clone(), file.name.clone())
                    } else {
                        FileManagerEvent::None
                    }
                }
            },
            KeyCode::Backspace => match self.mode {
                FileManagerMode::LocalUpload => {
                    if self.current_path.pop() {
                        self.read_current_dir();
                        self.list_state.select(Some(0));
                    }
                    FileManagerEvent::None
                }
                FileManagerMode::RemoteDownload => FileManagerEvent::None, // No going back in remote view
            },
            KeyCode::Esc => FileManagerEvent::CloseFileManager,
            _ => FileManagerEvent::None,
        }
    }

    fn read_current_dir(&mut self) {
        self.local_files.clear();
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    self.local_files.push(entry.path());
                }
            }
        }
    }

    fn select_previous(&mut self) {
        let len = match self.mode {
            FileManagerMode::LocalUpload => self.local_files.len(),
            FileManagerMode::RemoteDownload => self.remote_files.len(),
        };
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_next(&mut self) {
        let len = match self.mode {
            FileManagerMode::LocalUpload => self.local_files.len(),
            FileManagerMode::RemoteDownload => self.remote_files.len(),
        };
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

}
