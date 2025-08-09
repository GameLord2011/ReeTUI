use crate::tui::auth::page::ICONS;
use crate::tui::auth::state::{AuthMode, SelectedField};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::tui::text_input::TextInput;

pub struct AuthEventResult {
    pub next_page: Option<crate::app::TuiPage>,
    pub selected_icon_index: usize,
    pub current_mode: AuthMode,
    pub selected_field: SelectedField,

    pub should_submit: bool, // New field to signal a submission
    pub show_settings: bool,
}

pub fn handle_auth_event(
    event: Event,
    username_input: &mut TextInput,
    password_input: &mut TextInput,
    mut selected_icon_index: usize,
    mut current_mode: AuthMode,
    mut selected_field: SelectedField,
    _app_state: Arc<Mutex<crate::app::app_state::AppState>>,
) -> io::Result<AuthEventResult> {
    let mut next_page = None;
    let mut should_submit = false; // Initialize to false

    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('Q') | KeyCode::Esc => {
                    next_page = Some(crate::app::TuiPage::Exit);
                }
                KeyCode::Tab => {
                    current_mode = match current_mode {
                        AuthMode::Register => AuthMode::Login,
                        AuthMode::Login => AuthMode::Register,
                    };
                    selected_field = SelectedField::Username;
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
                        (AuthMode::Register, SelectedField::RegisterButton) => {
                            SelectedField::Username
                        }
                        (AuthMode::Login, SelectedField::LoginButton) => SelectedField::Username,
                        _ => selected_field,
                    };
                }
                KeyCode::Left => {
                    if matches!(selected_field, SelectedField::Icon) {
                        selected_icon_index = (selected_icon_index + ICONS.len() - 1) % ICONS.len();
                    } else if matches!(selected_field, SelectedField::Username) {
                        username_input.move_cursor_left();
                    } else if matches!(selected_field, SelectedField::Password) {
                        password_input.move_cursor_left();
                    }
                }
                KeyCode::Right => {
                    if matches!(selected_field, SelectedField::Icon) {
                        selected_icon_index = (selected_icon_index + 1) % ICONS.len();
                    } else if matches!(selected_field, SelectedField::Username) {
                        username_input.move_cursor_right();
                    } else if matches!(selected_field, SelectedField::Password) {
                        password_input.move_cursor_right();
                    }
                }
                KeyCode::Enter => {
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
                            should_submit = true; // Set the flag to true
                        }
                    }
                }
                KeyCode::Backspace => match selected_field {
                    SelectedField::Username => {
                        username_input.delete_char();
                    }
                    SelectedField::Password => {
                        password_input.delete_char();
                    }
                    _ => {}
                },
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::ALT)
                    {
                        // funny
                    } else {
                        match selected_field {
                            SelectedField::Username => {
                                username_input.insert_char(c);
                            }
                            SelectedField::Password => {
                                password_input.insert_char(c);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(AuthEventResult {
        next_page,
        selected_icon_index,
        current_mode,
        selected_field,
        should_submit, // Return the new flag
        show_settings: false,
    })
}
