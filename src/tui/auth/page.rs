// funny
// --- BEGIN MOVED CODE ---

use crate::tui::themes::interpolate_rgb;
use crate::tui::themes::{get_theme, rgb_to_color, Theme, ThemeName};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

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

pub const ICONS: [&str; 11] = ["󰱨", "󰱩", "󱃞", "󰱫", "󰱬", "󰱮", "󰱰", "󰽌", "󰱱", "󰱸", "󰇹"];

pub fn get_validation_error(
    username_input: &str,
    password_input: &str,
    _current_mode: &AuthMode,
) -> Option<String> {
    if username_input.trim().is_empty() {
        return Some("Username cannot be empty.".to_string());
    }
    if username_input.contains(' ') {
        return Some("Username cannot contain spaces.".to_string());
    }
    if password_input.trim().is_empty() {
        return Some("Password cannot be empty.".to_string());
    }
    if password_input.contains(' ') {
        return Some("Password cannot contain spaces.".to_string());
    }
    None
}

pub fn draw_ascii_title(f: &mut Frame, area: Rect, theme: &Theme, current_mode: &AuthMode) {
    let ascii_art_str = match current_mode {
        AuthMode::Register => {
            r#"
 ███████████                      ███           █████                      
░░███░░░░░███                    ░░░           ░░███                       
 ░███    ░███   ██████   ███████ ████   █████  ███████    ██████  ████████ 
 ░██████████   ███░░███ ███░░███░░███  ███░░  ░░░███░    ███░░███░░███░░███
 ░███░░░░░███ ░███████ ░███ ░███ ░███ ░░█████   ░███    ░███████  ░███ ░░░ 
 ░███    ░███ ░███░░░  ░███ ░███ ░███  ░░░░███  ░███ ███░███░░░   ░███     
 █████   █████░░██████ ░░███████ █████ ██████   ░░█████ ░░██████  █████    
░░░░░   ░░░░░  ░░░░░░   ░░░░░███░░░░░ ░░░░░░     ░░░░░   ░░░░░░  ░░░░░     
                         ███ ░███                                           
                        ░░██████                                            
                         ░░░░░░                                             "#
        }
        AuthMode::Login => {
            r#"
 █████                          ███            
░░███                          ░░░             
 ░███         ██████   ███████ ████  ████████  
 ░███        ███░░███ ███░░███░░███ ░░███░░███ 
 ░███       ░███ ░███░███ ░███ ░███  ░███ ░███ 
 ░███      █░███ ░███░███ ░███ ░███  ░███ ░███ 
 ███████████░░██████ ░░███████ █████ ████ █████
░░░░░░░░░░░  ░░░░░░   ░░░░░███░░░░░ ░░░░ ░░░░░ 
                       ███ ░███                 
                      ░░██████                  
                       ░░░░░░                   "#
        }
    };
    let lines: Vec<&str> = ascii_art_str
        .lines()
        .filter(|&line| !line.is_empty() || line.chars().any(|c| !c.is_whitespace()))
        .collect();
    let num_lines = lines.len();
    let max_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;
    let mut text_lines: Vec<Line> = Vec::new();
    let num_lines_for_gradient = if num_lines <= 1 {
        1.0
    } else {
        (num_lines - 1) as f32
    };
    for (line_idx, line_str) in lines.iter().enumerate() {
        let mut spans = Vec::new();
        let fraction_y = line_idx as f32 / num_lines_for_gradient;
        for ch in line_str.chars() {
            let interpolated_rgb = interpolate_rgb(
                &theme.title_gradient_start,
                &theme.title_gradient_end,
                fraction_y,
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

#[warn(clippy::too_many_arguments)]
pub fn draw_auth_ui(
    f: &mut Frame,
    username_input: &str,
    password_input: &str,
    selected_icon_index: usize,
    current_mode: &AuthMode,
    selected_field: &SelectedField,
    message: &str,
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
 ███████████                      ███           █████                      
░░███░░░░░███                    ░░░           ░░███                       
 ░███    ░███   ██████   ███████ ████   █████  ███████    ██████  ████████ 
 ░██████████   ███░░███ ███░░███░░███  ███░░  ░░░███░    ███░░███░░███░░███
 ░███░░░░░███ ░███████ ░███ ░███ ░███ ░░█████   ░███    ░███████  ░███ ░░░ 
 ░███    ░███ ░███░░░  ░███ ░███ ░███  ░░░░███  ░███ ███░███░░░   ░███     
 █████   █████░░██████ ░░███████ █████ ██████   ░░█████ ░░██████  █████    
░░░░░   ░░░░░  ░░░░░░   ░░░░░███░░░░░ ░░░░░░     ░░░░░   ░░░░░░  ░░░░░     
                          ███ ░███                                           
                         ░░██████                                            
                          ░░░░░░                                             "#
        }
        AuthMode::Login => {
            r#"
 █████                          ███            
░░███                          ░░░             
 ░███         ██████   ███████ ████  ████████  
 ░███        ███░░███ ███░░███░░███ ░░███░░███ 
 ░███       ░███ ░███░███ ░███ ░███  ░███ ░███ 
 ░███      █░███ ░███░███ ░███ ░███  ░███ ░███ 
 ███████████░░██████ ░░███████ █████ ████ █████
░░░░░░░░░░░  ░░░░░░   ░░░░░███░░░░░ ░░░░ ░░░░░ 
                        ███ ░███                 
                       ░░██████                  
                        ░░░░░░                   "#
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
    let input_labels = ["Username", "Password", "Icon"];
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
                password_input.chars().map(|_| "ILoveTv").collect()
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
    let button_border_color = if button_is_selected {
        rgb_to_color(&theme.button_border_active)
    } else if validation_error.is_some() {
        rgb_to_color(&theme.error)
    } else {
        rgb_to_color(&theme.button_border_inactive)
    };
    let button_text_color = if button_is_selected {
        rgb_to_color(&theme.button_border_active)
    } else {
        rgb_to_color(&theme.button_text_inactive)
    };

    let button_style = Style::default()
        .fg(button_text_color)
        .add_modifier(Modifier::BOLD);
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
        "Already have an account? Press < Tab> to switch to Login."
    } else {
        "Don't have an account? Press < Tab> to switch to Register."
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

    let instructions_text = "Q: Quit |   : Navigate | Enter: Submit";
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

    let show_popup = !message.is_empty();

    if show_popup {
        let popup_message_content = message;
        let popup_block = Block::default()
            .title(" Error")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.popup_border)));

        let popup_width: u16 = 40;
        let mut estimated_height_from_text: u16 = 0;
        let chars_per_line_estimate = popup_width.saturating_sub(2).max(1);

        if !popup_message_content.is_empty() {
            estimated_height_from_text = popup_message_content
                .lines()
                .map(|line| {
                    let num_chars = line.chars().count() as u16;
                    (num_chars + chars_per_line_estimate - 1) / chars_per_line_estimate
                })
                .sum();
            estimated_height_from_text = estimated_height_from_text.max(1);
        }

        let popup_height = (estimated_height_from_text + 2)
            .max(3)
            .min(size.height.saturating_sub(2));

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

        let popup_paragraph = Paragraph::new(Line::from(popup_message_content))
            .style(Style::default().fg(rgb_to_color(&theme.popup_text)))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(popup_paragraph, popup_text_area);
    }
}
