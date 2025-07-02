use crate::api::auth_api;
use crate::app::AppState;
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use sha2::{Digest, Sha256};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AuthMode {
    Register,
    Login,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SelectedField {
    Username,
    Password,
    Icon,
    RegisterButton,
    LoginButton,
}

const ICONS: [&str; 11] = ["󰱨", "󰱩", "󱃞", "󰱫", "󰱬", "󰱮", "󰱰", "󰽌", "󰱱", "󰱸", "󰇹"];

#[derive(Clone, Debug)]
pub struct Rgb(u8, u8, u8);

#[derive(Debug, Clone, Copy)]
pub enum ThemeName {
    Default,
    Oceanic,
    Forest,
    Monochrome,
    CatppuccinMocha,
    Dracula,
    SolarizedDark,
    GruvboxDark,
    Nord,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button: Rgb,
    pub button_focus: Rgb,
    pub text: Rgb,
    pub error: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,

    pub title_gradient_start: Rgb,
    pub title_gradient_end: Rgb,
    pub input_border_active: Rgb,
    pub input_border_inactive: Rgb,
    pub input_text_active: Rgb,
    pub input_text_inactive: Rgb,
    pub placeholder_text: Rgb,
    pub selected_icon: Rgb,
    pub dimmed_icon: Rgb,
    pub button_text_active: Rgb,
    pub button_text_inactive: Rgb,
    pub button_border_active: Rgb,
    pub button_border_inactive: Rgb,
    pub help_text: Rgb,
    pub instructions_text: Rgb,
    pub popup_border: Rgb,
    pub popup_text: Rgb,
}

fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn interpolate_rgb(start: &Rgb, end: &Rgb, fraction: f32) -> Rgb {
    let r = (start.0 as f32 + (end.0 as f32 - start.0 as f32) * fraction) as u8;
    let g = (start.1 as f32 + (end.1 as f32 - start.1 as f32) * fraction) as u8;
    let b = (start.2 as f32 + (end.2 as f32 - start.2 as f32) * fraction) as u8;
    Rgb(r, g, b)
}

fn get_theme(theme_name: ThemeName) -> Theme {
    match theme_name {
        ThemeName::Default => Theme {
            border: Rgb(120, 120, 120),
            border_focus: Rgb(255, 215, 0),
            button: Rgb(50, 150, 50),
            button_focus: Rgb(80, 200, 80),
            text: Rgb(240, 240, 240),
            error: Rgb(230, 60, 60),
            dim: Rgb(90, 90, 90),
            accent: Rgb(255, 165, 0),

            title_gradient_start: Rgb(255, 165, 0),
            title_gradient_end: Rgb(255, 215, 0),
            input_border_active: Rgb(255, 215, 0),
            input_border_inactive: Rgb(120, 120, 120),
            input_text_active: Rgb(240, 240, 240),
            input_text_inactive: Rgb(180, 180, 180),
            placeholder_text: Rgb(90, 90, 90),
            selected_icon: Rgb(80, 200, 80),
            dimmed_icon: Rgb(90, 90, 90),
            button_text_active: Rgb(255, 255, 255),
            button_text_inactive: Rgb(240, 240, 240),
            button_border_active: Rgb(80, 200, 80),
            button_border_inactive: Rgb(50, 150, 50),
            help_text: Rgb(90, 90, 90),
            instructions_text: Rgb(90, 90, 90),
            popup_border: Rgb(230, 60, 60),
            popup_text: Rgb(230, 60, 60),
        },
        ThemeName::Oceanic => Theme {
            border: Rgb(70, 130, 180),
            border_focus: Rgb(0, 191, 255),
            button: Rgb(60, 179, 113),
            button_focus: Rgb(0, 255, 127),
            text: Rgb(200, 230, 255),
            error: Rgb(255, 99, 71),
            dim: Rgb(100, 149, 237),
            accent: Rgb(135, 206, 250),

            title_gradient_start: Rgb(0, 191, 255),
            title_gradient_end: Rgb(135, 206, 250),
            input_border_active: Rgb(0, 191, 255),
            input_border_inactive: Rgb(70, 130, 180),
            input_text_active: Rgb(200, 230, 255),
            input_text_inactive: Rgb(170, 200, 230),
            placeholder_text: Rgb(100, 149, 237),
            selected_icon: Rgb(0, 255, 127),
            dimmed_icon: Rgb(100, 149, 237),
            button_text_active: Rgb(255, 255, 255),
            button_text_inactive: Rgb(200, 230, 255),
            button_border_active: Rgb(0, 255, 127),
            button_border_inactive: Rgb(60, 179, 113),
            help_text: Rgb(100, 149, 237),
            instructions_text: Rgb(100, 149, 237),
            popup_border: Rgb(255, 99, 71),
            popup_text: Rgb(255, 99, 71),
        },
        ThemeName::Forest => Theme {
            border: Rgb(107, 142, 35),
            border_focus: Rgb(154, 205, 50),
            button: Rgb(34, 139, 34),
            button_focus: Rgb(85, 107, 47),
            text: Rgb(245, 255, 250),
            error: Rgb(205, 92, 92),
            dim: Rgb(139, 69, 19),
            accent: Rgb(144, 238, 144),

            title_gradient_start: Rgb(144, 238, 144),
            title_gradient_end: Rgb(154, 205, 50),
            input_border_active: Rgb(154, 205, 50),
            input_border_inactive: Rgb(107, 142, 35),
            input_text_active: Rgb(245, 255, 250),
            input_text_inactive: Rgb(200, 230, 200),
            placeholder_text: Rgb(139, 69, 19),
            selected_icon: Rgb(85, 107, 47),
            dimmed_icon: Rgb(139, 69, 19),
            button_text_active: Rgb(255, 255, 255),
            button_text_inactive: Rgb(245, 255, 250),
            button_border_active: Rgb(85, 107, 47),
            button_border_inactive: Rgb(34, 139, 34),
            help_text: Rgb(139, 69, 19),
            instructions_text: Rgb(139, 69, 19),
            popup_border: Rgb(205, 92, 92),
            popup_text: Rgb(205, 92, 92),
        },
        ThemeName::Monochrome => Theme {
            border: Rgb(160, 160, 160),
            border_focus: Rgb(255, 255, 255),
            button: Rgb(100, 100, 100),
            button_focus: Rgb(180, 180, 180),
            text: Rgb(220, 220, 220),
            error: Rgb(255, 50, 50),
            dim: Rgb(80, 80, 80),
            accent: Rgb(200, 200, 200),

            title_gradient_start: Rgb(255, 255, 255),
            title_gradient_end: Rgb(200, 200, 200),
            input_border_active: Rgb(255, 255, 255),
            input_border_inactive: Rgb(160, 160, 160),
            input_text_active: Rgb(220, 220, 220),
            input_text_inactive: Rgb(180, 180, 180),
            placeholder_text: Rgb(80, 80, 80),
            selected_icon: Rgb(180, 180, 180),
            dimmed_icon: Rgb(80, 80, 80),
            button_text_active: Rgb(255, 255, 255),
            button_text_inactive: Rgb(220, 220, 220),
            button_border_active: Rgb(180, 180, 180),
            button_border_inactive: Rgb(100, 100, 100),
            help_text: Rgb(80, 80, 80),
            instructions_text: Rgb(80, 80, 80),
            popup_border: Rgb(255, 50, 50),
            popup_text: Rgb(255, 50, 50),
        },
        ThemeName::CatppuccinMocha => Theme {
            border: Rgb(88, 91, 112),
            border_focus: Rgb(250, 179, 135),
            button: Rgb(166, 227, 161),
            button_focus: Rgb(148, 226, 213),
            text: Rgb(205, 214, 244),
            error: Rgb(243, 139, 168),
            dim: Rgb(108, 112, 134),
            accent: Rgb(245, 224, 220),

            // Gradient from Maroon to Flamingo
            title_gradient_start: Rgb(235, 160, 172),
            title_gradient_end: Rgb(221, 120, 120),

            input_border_active: Rgb(250, 179, 135),
            input_border_inactive: Rgb(88, 91, 112),
            input_text_active: Rgb(205, 214, 244),
            input_text_inactive: Rgb(170, 180, 200),
            placeholder_text: Rgb(108, 112, 134),
            selected_icon: Rgb(148, 226, 213),
            dimmed_icon: Rgb(108, 112, 134),
            button_text_active: Rgb(30, 30, 46),
            button_text_inactive: Rgb(205, 214, 244),
            button_border_active: Rgb(148, 226, 213),
            button_border_inactive: Rgb(166, 227, 161),
            help_text: Rgb(108, 112, 134),
            instructions_text: Rgb(108, 112, 134),
            popup_border: Rgb(243, 139, 168),
            popup_text: Rgb(243, 139, 168),
        },
        ThemeName::Dracula => Theme {
            border: Rgb(98, 114, 164),
            border_focus: Rgb(255, 121, 198),
            button: Rgb(80, 250, 123),
            button_focus: Rgb(189, 147, 249),
            text: Rgb(248, 248, 242),
            error: Rgb(255, 85, 85),
            dim: Rgb(68, 71, 90),
            accent: Rgb(255, 184, 108),

            title_gradient_start: Rgb(255, 121, 198),
            title_gradient_end: Rgb(255, 184, 108),
            input_border_active: Rgb(255, 121, 198),
            input_border_inactive: Rgb(98, 114, 164),
            input_text_active: Rgb(248, 248, 242),
            input_text_inactive: Rgb(200, 200, 190),
            placeholder_text: Rgb(68, 71, 90),
            selected_icon: Rgb(189, 147, 249),
            dimmed_icon: Rgb(68, 71, 90),
            button_text_active: Rgb(248, 248, 242),
            button_text_inactive: Rgb(248, 248, 242),
            button_border_active: Rgb(189, 147, 249),
            button_border_inactive: Rgb(80, 250, 123),
            help_text: Rgb(68, 71, 90),
            instructions_text: Rgb(68, 71, 90),
            popup_border: Rgb(255, 85, 85),
            popup_text: Rgb(255, 85, 85),
        },
        ThemeName::SolarizedDark => Theme {
            border: Rgb(88, 104, 117),
            border_focus: Rgb(42, 161, 152),
            button: Rgb(133, 153, 0),
            button_focus: Rgb(38, 139, 210),
            text: Rgb(147, 161, 161),
            error: Rgb(220, 50, 47),
            dim: Rgb(101, 123, 131),
            accent: Rgb(203, 75, 22),

            title_gradient_start: Rgb(42, 161, 152),
            title_gradient_end: Rgb(203, 75, 22),
            input_border_active: Rgb(42, 161, 152),
            input_border_inactive: Rgb(88, 104, 117),
            input_text_active: Rgb(147, 161, 161),
            input_text_inactive: Rgb(120, 130, 130),
            placeholder_text: Rgb(101, 123, 131),
            selected_icon: Rgb(38, 139, 210),
            dimmed_icon: Rgb(101, 123, 131),
            button_text_active: Rgb(255, 255, 255),
            button_text_inactive: Rgb(147, 161, 161),
            button_border_active: Rgb(38, 139, 210),
            button_border_inactive: Rgb(133, 153, 0),
            help_text: Rgb(101, 123, 131),
            instructions_text: Rgb(101, 123, 131),
            popup_border: Rgb(220, 50, 47),
            popup_text: Rgb(220, 50, 47),
        },
        ThemeName::GruvboxDark => Theme {
            border: Rgb(146, 160, 146),
            border_focus: Rgb(251, 241, 199),
            button: Rgb(152, 151, 26),
            button_focus: Rgb(184, 187, 38),
            text: Rgb(235, 219, 178),
            error: Rgb(251, 73, 52),
            dim: Rgb(129, 129, 129),
            accent: Rgb(250, 187, 85),

            title_gradient_start: Rgb(251, 241, 199),
            title_gradient_end: Rgb(250, 187, 85),
            input_border_active: Rgb(251, 241, 199),
            input_border_inactive: Rgb(146, 160, 146),
            input_text_active: Rgb(235, 219, 178),
            input_text_inactive: Rgb(190, 180, 140),
            placeholder_text: Rgb(129, 129, 129),
            selected_icon: Rgb(184, 187, 38),
            dimmed_icon: Rgb(129, 129, 129),
            button_text_active: Rgb(235, 219, 178),
            button_text_inactive: Rgb(235, 219, 178),
            button_border_active: Rgb(184, 187, 38),
            button_border_inactive: Rgb(152, 151, 26),
            help_text: Rgb(129, 129, 129),
            instructions_text: Rgb(129, 129, 129),
            popup_border: Rgb(251, 73, 52),
            popup_text: Rgb(251, 73, 52),
        },
        ThemeName::Nord => Theme {
            border: Rgb(76, 86, 106),
            border_focus: Rgb(143, 188, 187),
            button: Rgb(109, 142, 183),
            button_focus: Rgb(136, 192, 208),
            text: Rgb(236, 239, 244),
            error: Rgb(191, 97, 106),
            dim: Rgb(94, 129, 172),
            accent: Rgb(163, 190, 140),

            title_gradient_start: Rgb(143, 188, 187),
            title_gradient_end: Rgb(163, 190, 140),
            input_border_active: Rgb(143, 188, 187),
            input_border_inactive: Rgb(76, 86, 106),
            input_text_active: Rgb(236, 239, 244),
            input_text_inactive: Rgb(190, 200, 210),
            placeholder_text: Rgb(94, 129, 172),
            selected_icon: Rgb(136, 192, 208),
            dimmed_icon: Rgb(94, 129, 172),
            button_text_active: Rgb(236, 239, 244),
            button_text_inactive: Rgb(236, 239, 244),
            button_border_active: Rgb(136, 192, 208),
            button_border_inactive: Rgb(109, 142, 183),
            help_text: Rgb(94, 129, 172),
            instructions_text: Rgb(94, 129, 172),
            popup_border: Rgb(191, 97, 106),
            popup_text: Rgb(191, 97, 106),
        },
    }
}

pub async fn run_auth_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let mut username_input = String::new();
    let mut password_input = String::new();
    let mut selected_icon_index: usize = 0;
    let mut current_mode = AuthMode::Register;
    let mut selected_field = SelectedField::Username;
    let mut message = String::new();
    let mut current_theme = ThemeName::Default;

    let client = reqwest::Client::new();

    loop {
        if selected_icon_index >= ICONS.len() {
            selected_icon_index = ICONS.len() - 1;
        }

        terminal.draw(|f| {
            draw_auth_ui(
                f,
                &username_input,
                &password_input,
                selected_icon_index,
                &current_mode,
                &selected_field,
                &message,
                current_theme,
            );
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('1') => current_theme = ThemeName::Default,
                        KeyCode::Char('2') => current_theme = ThemeName::Oceanic,
                        KeyCode::Char('3') => current_theme = ThemeName::Forest,
                        KeyCode::Char('4') => current_theme = ThemeName::Monochrome,
                        KeyCode::Char('5') => current_theme = ThemeName::CatppuccinMocha,
                        KeyCode::Char('6') => current_theme = ThemeName::Dracula,
                        KeyCode::Char('7') => current_theme = ThemeName::SolarizedDark,
                        KeyCode::Char('8') => current_theme = ThemeName::GruvboxDark,
                        KeyCode::Char('9') => current_theme = ThemeName::Nord,
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(TuiPage::Exit),
                        KeyCode::Tab => {
                            current_mode = match current_mode {
                                AuthMode::Register => AuthMode::Login,
                                AuthMode::Login => AuthMode::Register,
                            };
                            selected_field = SelectedField::Username;
                            message.clear();
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
                                (AuthMode::Register, SelectedField::Icon) => {
                                    SelectedField::Password
                                }
                                (AuthMode::Register, SelectedField::RegisterButton) => {
                                    SelectedField::Icon
                                }
                                (AuthMode::Login, SelectedField::LoginButton) => {
                                    SelectedField::Password
                                }
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
                                (AuthMode::Register, SelectedField::Icon) => {
                                    SelectedField::RegisterButton
                                }
                                (AuthMode::Register, SelectedField::RegisterButton) => {
                                    SelectedField::Username
                                }
                                (AuthMode::Login, SelectedField::LoginButton) => {
                                    SelectedField::Username
                                }
                                _ => selected_field,
                            };
                        }
                        KeyCode::Left => {
                            if matches!(selected_field, SelectedField::Icon) {
                                selected_icon_index =
                                    (selected_icon_index + ICONS.len() - 1) % ICONS.len();
                            }
                        }
                        KeyCode::Right => {
                            if matches!(selected_field, SelectedField::Icon) {
                                selected_icon_index = (selected_icon_index + 1) % ICONS.len();
                            }
                        }
                        KeyCode::Enter => match selected_field {
                            SelectedField::Username => selected_field = SelectedField::Password,
                            SelectedField::Password => {
                                if current_mode == AuthMode::Register {
                                    selected_field = SelectedField::Icon;
                                } else {
                                    selected_field = SelectedField::LoginButton;
                                }
                            }
                            SelectedField::Icon => selected_field = SelectedField::RegisterButton,
                            SelectedField::RegisterButton => {
                                if current_mode == AuthMode::Register {
                                    let validation_error = get_validation_error(
                                        &username_input,
                                        &password_input,
                                        &current_mode,
                                    );
                                    if let Some(err_msg) = validation_error {
                                        message = err_msg;
                                        continue;
                                    }

                                    let hashed_password =
                                        format!("{:x}", Sha256::digest(password_input.as_bytes()));
                                    message = "Registering...".to_string();
                                    terminal.draw(|f| {
                                        draw_auth_ui(
                                            f,
                                            &username_input,
                                            &password_input,
                                            selected_icon_index,
                                            &current_mode,
                                            &selected_field,
                                            &message,
                                            current_theme,
                                        );
                                    })?;

                                    match auth_api::register(
                                        &client,
                                        &username_input,
                                        &hashed_password,
                                        ICONS[selected_icon_index],
                                    )
                                    .await
                                    {
                                        Ok(token_response) => {
                                            let mut state = app_state.lock().unwrap();
                                            state.set_user_auth(
                                                token_response.token,
                                                username_input.clone(),
                                                token_response.icon,
                                            );
                                            return Ok(TuiPage::Home);
                                        }
                                        Err(e) => {
                                            message = format!("Registration failed: {}", e);
                                        }
                                    }
                                }
                            }
                            SelectedField::LoginButton => {
                                if current_mode == AuthMode::Login {
                                    let validation_error = get_validation_error(
                                        &username_input,
                                        &password_input,
                                        &current_mode,
                                    );
                                    if let Some(err_msg) = validation_error {
                                        message = err_msg;
                                        continue;
                                    }

                                    let hashed_password =
                                        format!("{:x}", Sha256::digest(password_input.as_bytes()));
                                    message = "Logging in...".to_string();
                                    terminal.draw(|f| {
                                        draw_auth_ui(
                                            f,
                                            &username_input,
                                            &password_input,
                                            selected_icon_index,
                                            &current_mode,
                                            &selected_field,
                                            &message,
                                            current_theme,
                                        );
                                    })?;

                                    match auth_api::login(
                                        &client,
                                        &username_input,
                                        &hashed_password,
                                    )
                                    .await
                                    {
                                        Ok(token_response) => {
                                            let mut state = app_state.lock().unwrap();
                                            state.set_user_auth(
                                                token_response.token,
                                                username_input.clone(),
                                                token_response.icon,
                                            );
                                            return Ok(TuiPage::Home);
                                        }
                                        Err(e) => {
                                            message = format!("Login failed: {}", e);
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Backspace => match selected_field {
                            SelectedField::Username => {
                                username_input.pop();
                            }
                            SelectedField::Password => {
                                password_input.pop();
                            }
                            _ => {}
                        },
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                || key.modifiers.contains(KeyModifiers::ALT)
                            {
                                continue;
                            }
                            match selected_field {
                                SelectedField::Username => {
                                    username_input.push(c);
                                }
                                SelectedField::Password => {
                                    password_input.push(c);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn get_validation_error(
    username_input: &str,
    password_input: &str,
    _current_mode: &AuthMode,
) -> Option<String> {
    if username_input.trim().is_empty() {
        return Some("Username cannot be empty.".to_string());
    }
    if password_input.trim().is_empty() {
        return Some("Password cannot be empty.".to_string());
    }
    None
}

fn draw_ascii_title(f: &mut Frame, area: Rect, theme: &Theme, current_mode: &AuthMode) {
    let ascii_art_str = match current_mode {
        AuthMode::Register => {
            r#"
  _____            _     _            
 |  __ \          (_)   | |           
 | |__) |___  __ _ _ ___| |_ ___ _ __ 
 |  _  // _ \/ _` | / __| __/ _ \ '__|
 | | \ \  __/ (_| | \__ \ ||  __/ |   
 |_|  \_\___|\__, |_|___/\__\___|_|   
              __/ |                   
             |___/                    
    "#
        }
        AuthMode::Login => {
            r#"
  _                 _       
 | |               (_)      
 | |     ___   __ _ _ _ __  
 | |    / _ \ / _` | | '_ \ 
 | |___| (_) | (_| | | | | |
 |______\___/ \__, |_|_| |_|
               __/ |        
              |___/         
    "#
        }
    };

    let lines: Vec<&str> = ascii_art_str.lines().collect();
    let num_lines = lines.len();
    let max_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

    let mut text_lines: Vec<Line> = Vec::new();

    for (line_idx, line_str) in lines.iter().enumerate() {
        let mut spans = Vec::new();
        let fraction_y = if num_lines > 1 {
            line_idx as f32 / (num_lines - 1) as f32
        } else {
            0.0
        };

        for (char_idx, ch) in line_str.chars().enumerate() {
            let line_len_f32 = if line_str.len() > 1 {
                (line_str.len() - 1) as f32
            } else {
                0.0
            };
            let fraction_x = if line_len_f32 > 0.0 {
                char_idx as f32 / line_len_f32
            } else {
                0.0
            };

            let combined_fraction = (fraction_y + fraction_x) / 2.0;

            let interpolated_rgb = interpolate_rgb(
                &theme.title_gradient_start,
                &theme.title_gradient_end,
                combined_fraction,
            );
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(rgb_to_color(&interpolated_rgb)),
            ));
        }
        text_lines.push(Line::from(spans));
    }

    let title_paragraph = Paragraph::new(text_lines).alignment(Alignment::Center);

    let centered_title_width = max_line_width;
    let title_x = area.x + (area.width.saturating_sub(centered_title_width)) / 2;

    let title_area = Rect {
        x: title_x,
        y: area.y + 1,
        width: centered_title_width.min(area.width),
        height: num_lines as u16,
    };
    f.render_widget(title_paragraph, title_area);
}
fn draw_auth_ui(
    f: &mut Frame,
    username_input: &str,
    password_input: &str,
    selected_icon_index: usize,
    current_mode: &AuthMode,
    selected_field: &SelectedField,
    _message: &str,
    theme_name: ThemeName,
) {
    let size = f.area();
    let theme = get_theme(theme_name);

    f.render_widget(
        Block::default().style(Style::default().fg(rgb_to_color(&theme.text))),
        f.area(),
    );

    draw_ascii_title(f, size, &theme, current_mode);

    let ascii_art_str = match current_mode {
        AuthMode::Register => {
            r#" 
   _____            _     _            
 |  __ \          (_)   | |           
 | |__) |___  __ _ _ ___| |_ ___ _ __ 
 |  _  // _ \/ _` | / __| __/ _ \ '__|
 | | \ \  __/ (_| | \__ \ ||  __/ |   
 |_|  \_\___|\__, |_|___/\__\___|_|   
              __/ |                   
             |___/                    "#
        }
        AuthMode::Login => {
            r#" 
   _                 _       
 | |               (_)      
 | |     ___   __ _ _ _ __  
 | |    / _ \ / _` | | '_ \ 
 | |___| (_) | (_| | | | | |
 |______\___/ \__, |_|_| |_|
               __/ |        
                   |___/         "#
        }
    };
    let title_height = ascii_art_str.trim().lines().count() as u16;
    let margin_after_title: u16 = 2;

    let visible_inputs = if *current_mode == AuthMode::Register {
        3
    } else {
        2
    };

    let main_box_width = 35;

    let content_height = (visible_inputs as u16 * 3) + 3;
    let main_box_height = content_height + 2;

    let main_box_x = size.x + (size.width.saturating_sub(main_box_width)) / 2;
    let main_box_y = size.y + title_height + margin_after_title;

    let main_area = Rect::new(
        main_box_x,
        main_box_y,
        main_box_width.min(size.width),
        main_box_height.min(size.height.saturating_sub(main_box_y)),
    );

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.border))),
        main_area,
    );

    let inner = Rect {
        x: main_area.x + 1,
        y: main_area.y + 1,
        width: main_area.width.saturating_sub(2),
        height: main_area.height.saturating_sub(2),
    };

    let mut constraints = Vec::with_capacity(visible_inputs + 1);
    for _ in 0..visible_inputs {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Length(3));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let input_labels = ["Username", "Password", "Select Icon"];

    for (idx, label) in input_labels.iter().take(visible_inputs).enumerate() {
        let focus = *selected_field
            == match idx {
                0 => SelectedField::Username,
                1 => SelectedField::Password,
                2 => SelectedField::Icon,
                _ => unreachable!(),
            };
        let input_area = rows[idx];
        let border_style = if focus {
            Style::default()
                .fg(rgb_to_color(&theme.input_border_active))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rgb_to_color(&theme.input_border_inactive))
        };
        let text_style = if focus {
            Style::default()
                .fg(rgb_to_color(&theme.input_text_active))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rgb_to_color(&theme.input_text_inactive))
        };

        if *current_mode == AuthMode::Register && idx == 2 {
            let len = ICONS.len();
            let center = selected_icon_index;
            let display_range = 3;

            let mut spans = Vec::with_capacity(display_range * 2 + 1);

            for i in (center as isize - display_range as isize)
                ..(center as isize + display_range as isize + 1)
            {
                let actual_index = (i % len as isize + len as isize) % len as isize;
                let icon_char = ICONS[actual_index as usize];
                if actual_index == center as isize {
                    spans.push(ratatui::text::Span::styled(
                        icon_char,
                        Style::default()
                            .fg(rgb_to_color(&theme.selected_icon))
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(ratatui::text::Span::styled(
                        icon_char,
                        Style::default()
                            .fg(rgb_to_color(&theme.dimmed_icon))
                            .add_modifier(Modifier::DIM),
                    ));
                }
                if i != center as isize + display_range as isize {
                    spans.push(ratatui::text::Span::raw("   "));
                }
            }

            let icon_para = Paragraph::new(Line::from(spans))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(border_style)
                        .title(ratatui::text::Span::styled(
                            label.to_string(),
                            Style::default()
                                .fg(rgb_to_color(&theme.placeholder_text))
                                .add_modifier(Modifier::ITALIC),
                        )),
                );
            f.render_widget(icon_para, input_area);
        } else {
            let input_value = if idx == 0 {
                username_input.to_string()
            } else {
                password_input.chars().map(|_| " ").collect()
            };

            f.render_widget(
                Paragraph::new(input_value)
                    .style(text_style)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(border_style)
                            .title(ratatui::text::Span::styled(
                                label.to_string(),
                                Style::default()
                                    .fg(rgb_to_color(&theme.placeholder_text))
                                    .add_modifier(Modifier::ITALIC),
                            )),
                    )
                    .alignment(Alignment::Left),
                input_area,
            );
        }
    }

    let button_chunk_index = visible_inputs;
    let button_text = match current_mode {
        AuthMode::Register => "Register",
        AuthMode::Login => "Login",
    };

    let button_is_selected = match selected_field {
        SelectedField::RegisterButton => *current_mode == AuthMode::Register,
        SelectedField::LoginButton => *current_mode == AuthMode::Login,
        _ => false,
    };

    let validation_error = get_validation_error(username_input, password_input, current_mode);
    let button_text_color = if button_is_selected {
        rgb_to_color(&theme.button_text_active)
    } else {
        rgb_to_color(&theme.button_text_inactive)
    };

    let button_border_color = if button_is_selected {
        rgb_to_color(&theme.button_border_active)
    } else if validation_error.is_some() {
        rgb_to_color(&theme.error)
    } else {
        rgb_to_color(&theme.button_border_inactive)
    };

    let button_style = if button_is_selected {
        Style::default()
            .fg(button_text_color)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
            .fg(button_text_color)
            .add_modifier(Modifier::BOLD)
    };

    let btn_para = Paragraph::new(ratatui::text::Span::styled(button_text, button_style))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(button_border_color)
                .title(""),
        );
    f.render_widget(btn_para, rows[button_chunk_index]);

    let help_text = if *current_mode == AuthMode::Register {
        "Already have an account? Press <Tab> to switch to Login."
    } else {
        "Don't have an account? Press <Tab> to switch to Register."
    };
    let help_line = Line::from(vec![ratatui::text::Span::styled(
        help_text,
        Style::default()
            .fg(rgb_to_color(&theme.help_text))
            .add_modifier(Modifier::ITALIC),
    )]);

    let help_text_area = Rect {
        x: size.x,
        y: main_area.y + main_area.height + 1,
        width: size.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(help_line)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE)),
        help_text_area,
    );

    let instructions_text = "Q: Quit | ↑ ↓: Navigate | Enter: Submit | 1-9: Themes";
    let instructions = Paragraph::new(Line::from(instructions_text))
        .style(Style::default().fg(rgb_to_color(&theme.instructions_text)))
        .alignment(Alignment::Center);

    let instructions_area = Rect {
        x: size.x + 1,
        y: size.height.saturating_sub(2),
        width: size.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(instructions, instructions_area);

    let show_popup = matches!(current_mode, AuthMode::Register | AuthMode::Login)
        && (matches!(selected_field, SelectedField::RegisterButton)
            || matches!(selected_field, SelectedField::LoginButton))
        && validation_error.is_some();

    if show_popup {
        let popup_message = validation_error.unwrap_or_else(|| "Unknown error.".to_string());
        let popup_block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.popup_border)));

        let popup_width = 40;
        let popup_height = 3;
        let popup_area = Rect::new(
            size.width.saturating_sub(popup_width).saturating_sub(1),
            size.y + 1,
            popup_width,
            popup_height,
        );

        f.render_widget(Clear, popup_area);
        f.render_widget(popup_block, popup_area);

        let popup_text_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .margin(1)
            .split(popup_area)[0];

        let popup_paragraph = Paragraph::new(Line::from(popup_message))
            .style(Style::default().fg(rgb_to_color(&theme.popup_text)))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(popup_paragraph, popup_text_area);
    }
}
