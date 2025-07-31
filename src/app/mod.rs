use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TuiPage {
    Home,
    Chat,
    Auth,
    Exit,
    Settings,
}

pub mod app_state;
pub use app_state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Notification {
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupType {
    CreateChannel,
    Deconnection,
    Mentions,
    Emojis,
    FileManager,
    DownloadProgress,
    DebugJson,
    Settings,
    Downloads,
    None,
    Notification,
}

#[derive(Debug, Clone, Copy)]
pub struct PopupState {
    pub show: bool,
    pub popup_type: PopupType,
}

impl Default for PopupState {
    fn default() -> Self {
        Self {
            show: false,
            popup_type: PopupType::None,
        }
    }
}