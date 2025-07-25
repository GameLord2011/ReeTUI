use crate::tui::auth::page::{AuthMode, SelectedField};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AuthState {
    pub username_input: String,
    pub password_input: String,
    pub selected_icon_index: usize,
    pub current_mode: AuthMode,
    pub selected_field: SelectedField,
    pub message_state: Arc<Mutex<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            username_input: String::new(),
            password_input: String::new(),
            selected_icon_index: 0,
            current_mode: AuthMode::Register,
            selected_field: SelectedField::Username,
            message_state: Arc::new(Mutex::new(String::new())),
        }
    }
}
