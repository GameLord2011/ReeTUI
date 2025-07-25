use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io;
use std::time::Duration;
use crate::tui::auth::page::{AuthMode, SelectedField, ICONS};

pub struct AuthEventResult {
    pub next_page: Option<crate::tui::TuiPage>,
    pub username_input: String,
    pub password_input: String,
    pub selected_icon_index: usize,
    pub current_mode: AuthMode,
    pub selected_field: SelectedField,
    pub message: Option<String>,
}

pub fn handle_auth_event(
    wait_time: Duration,
    mut username_input: String,
    mut password_input: String,
    mut selected_icon_index: usize,
    mut current_mode: AuthMode,
    mut selected_field: SelectedField,
    message: Option<String>,
) -> io::Result<AuthEventResult> {
    let mut msg = message;
    let mut next_page = None;
    if event::poll(wait_time)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('Q') | KeyCode::Esc => {
                        next_page = Some(crate::tui::TuiPage::Exit);
                    }
                    KeyCode::Tab => {
                        current_mode = match current_mode {
                            AuthMode::Register => AuthMode::Login,
                            AuthMode::Login => AuthMode::Register,
                        };
                        selected_field = SelectedField::Username;
                        msg = None;
                    }
                    KeyCode::Up => {
                        selected_field = match (current_mode, selected_field) {
                            (_, SelectedField::Username) => {
                                if current_mode == AuthMode::Register {
                                    SelectedField::RegisterButton
                                } else {
                                    SelectedField::LoginButton
                                }
                            }
                            (_, SelectedField::Password) => SelectedField::Username,
                            (AuthMode::Register, SelectedField::Icon) => SelectedField::Password,
                            (AuthMode::Register, SelectedField::RegisterButton) => SelectedField::Icon,
                            (AuthMode::Login, SelectedField::LoginButton) => SelectedField::Password,
                            _ => selected_field,
                        };
                        msg = None;
                    }
                    KeyCode::Down => {
                        selected_field = match (current_mode, selected_field) {
                            (_, SelectedField::Username) => SelectedField::Password,
                            (_, SelectedField::Password) => {
                                if current_mode == AuthMode::Register {
                                    SelectedField::Icon
                                } else {
                                    SelectedField::LoginButton
                                }
                            }
                            (AuthMode::Register, SelectedField::Icon) => SelectedField::RegisterButton,
                            (AuthMode::Register, SelectedField::RegisterButton) => SelectedField::Username,
                            (AuthMode::Login, SelectedField::LoginButton) => SelectedField::Username,
                            _ => selected_field,
                        };
                        msg = None;
                    }
                    KeyCode::Left => {
                        if matches!(selected_field, SelectedField::Icon) {
                            selected_icon_index = (selected_icon_index + ICONS.len() - 1) % ICONS.len();
                        }
                        msg = None;
                    }
                    KeyCode::Right => {
                        if matches!(selected_field, SelectedField::Icon) {
                            selected_icon_index = (selected_icon_index + 1) % ICONS.len();
                        }
                        msg = None;
                    }
                    KeyCode::Enter => {
                        msg = None;
                        match selected_field {
                            SelectedField::Username => selected_field = SelectedField::Password,
                            SelectedField::Password => {
                                if current_mode == AuthMode::Register {
                                    selected_field = SelectedField::Icon;
                                } else {
                                    selected_field = SelectedField::LoginButton;
                                }
                            }
                            SelectedField::Icon => selected_field = SelectedField::RegisterButton,
                            SelectedField::RegisterButton | SelectedField::LoginButton => {
                                // Validation and API calls should be handled in mod.rs
                                // Here, just set the field for submission
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        msg = None;
                        match selected_field {
                            SelectedField::Username => { username_input.pop(); },
                            SelectedField::Password => { password_input.pop(); },
                            _ => {}
                        }
                    }
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
                            // funny
                        } else {
                            msg = None;
                            match selected_field {
                                SelectedField::Username => { username_input.push(c); },
                                SelectedField::Password => { password_input.push(c); },
                                _ => {}
                            }
                        }
                    }
                    _ => { msg = None; }
                }
            }
        }
    }
    Ok(AuthEventResult {
        next_page,
        username_input,
        password_input,
        selected_icon_index,
        current_mode,
        selected_field,
        message: msg,
    })
}
