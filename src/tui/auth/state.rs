use crate::tui::text_input::TextInput;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthMode {
    Register,
    Login,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SelectedField {
    Username,
    Password,
    Icon,
    RegisterButton,
    LoginButton,
}

pub struct AuthState {
    pub username_input: TextInput,
    pub password_input: TextInput,
    pub selected_icon_index: usize,
    pub current_mode: AuthMode,
    pub selected_field: SelectedField,
    pub message_state: Arc<Mutex<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        let mut username_input = TextInput::new("Username".to_string());
        username_input.is_focused = true;
        Self {
            username_input,
            password_input: {
                let mut password_input = TextInput::new("Password".to_string());
                password_input.is_password = true;
                password_input.password_char = Some('•');
                password_input
            },
            selected_icon_index: 0,
            current_mode: AuthMode::Register,
            selected_field: SelectedField::Username,
            message_state: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn next_field(&mut self) {
        self.selected_field = match self.selected_field {
            SelectedField::Username => SelectedField::Password,
            SelectedField::Password => {
                if self.current_mode == AuthMode::Register {
                    SelectedField::Icon
                } else {
                    SelectedField::LoginButton
                }
            }
            SelectedField::Icon => SelectedField::RegisterButton,
            SelectedField::RegisterButton => SelectedField::Username,
            SelectedField::LoginButton => SelectedField::Username,
        };
        self.update_focus();
    }

    pub fn update_focus(&mut self) {
        self.username_input.is_focused = matches!(self.selected_field, SelectedField::Username);
        self.password_input.is_focused = matches!(self.selected_field, SelectedField::Password);
    }
}
