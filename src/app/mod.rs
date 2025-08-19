use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TuiPage {
    Home,
    Chat,
    Auth,
    Exit,
    Settings,
    Help,
}

pub mod app_state;
pub use app_state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PopupType {
    CreateChannel,
    Deconnection,
    Mentions,
    Emojis,
    FileManager,
    DownloadProgress,
    
    Settings,
    Downloads,
    None,
    Notification,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
