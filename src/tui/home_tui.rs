use crate::app::AppState;
use crate::tui::utils::{interpolate_rgb, rgb_to_color, Rgb, Theme}; // Assuming these are in tui::utils
use crate::tui::TuiPage; // Import TuiPage enum
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant}, // Import Instant
};

// --- ASCII Art Animation Frames ---
// Replace this with your actual animation frames
const ANIMATION_FRAMES: [&str; 4] = [
    r#"
  _____
 /     \
|       |
 \  O  /
  -----
"#,
    r#"
  _____
 / _   \
| | |   |
 \/ O  /
  -----
"#,
    r#"
  _____
 /   _ \
|   | | |
 \ O \/
  -----
"#,
    r#"
  _____
 /     \
|_______|
 \  O  /
  -----
"#,
];
const FRAME_DURATION_MS: u64 = 20; // 20 milliseconds per frame

pub async fn run_home_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    // You'll need a default theme or get it from app_state if it's stored there.
    let default_theme = Theme {
        title_gradient_start: Rgb { r: 255, g: 0, b: 0 }, // Red
        title_gradient_end: Rgb { r: 0, g: 0, b: 255 },   // Blue
        background_color: Color::Black,
        text_color: Color::White,
        border_color: Color::Green,
    };

    loop {
        // Draw the UI
        terminal.draw(|f| {
            draw_home_ui::<B>(f, app_state.clone(), &default_theme);
        })?;

        // Handle animation update and events
        let mut state = app_state.lock().unwrap();

        if !state.animation_finished {
            let now = Instant::now();
            if now.duration_since(state.last_frame_time).as_millis() as u64 >= FRAME_DURATION_MS {
                state.animation_frame_index += 1;
                if state.animation_frame_index >= ANIMATION_FRAMES.len() {
                    state.animation_frame_index = ANIMATION_FRAMES.len() - 1; // Stay on last frame
                    state.animation_finished = true;
                }
                state.last_frame_time = now;
            }
        }
        drop(state); // Release the lock before polling events

        // Handle user input events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(TuiPage::Exit), // Global exit
                        _ => return Ok(TuiPage::Chat), // Any other key goes to Chat
                    }
                }
            }
        }
    }
}

/// Draws the home page UI.
fn draw_home_ui<B: Backend>(f: &mut Frame, app_state: Arc<Mutex<AppState>>, theme: &Theme) {
    let size = f.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Home Page ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));

    f.render_widget(block, size);

    // Define layout for the main content within the bordered area
    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2) // Reduced margin to give more space inside the main border
        .constraints(
            [
                Constraint::Length(3), // User info
                Constraint::Min(0),    // Space for ASCII logo
                Constraint::Length(3), // Instructions
            ]
            .as_ref(),
        )
        .split(size); // Split the entire frame area

    // User Info Paragraph
    let user_info_text = {
        let state = app_state.lock().unwrap();
        if let Some(username) = &state.username {
            format!(
                "Welcome, {} {}",
                username,
                state.user_icon.as_deref().unwrap_or("")
            )
        } else {
            "Not logged in.".to_string()
        }
    };

    let user_info_paragraph = Paragraph::new(Line::from(user_info_text))
        .block(Block::default().borders(Borders::ALL).title("User Info"))
        .style(Style::default().fg(Color::LightGreen))
        .alignment(Alignment::Center);
    f.render_widget(user_info_paragraph, outer_chunks[0]);

    // --- ASCII Animation Drawing ---
    let current_frame_index = app_state.lock().unwrap().animation_frame_index;
    let current_frame_str = ANIMATION_FRAMES[current_frame_index];

    // The `trim_start()` is important here to remove the leading newline from the r#"..."# literal,
    // but preserve any trailing blank lines if they're part of your art's vertical spacing.
    let lines: Vec<&str> = current_frame_str
        .trim_start()
        .lines()
        .filter(|&line| !line.is_empty() || line.chars().any(|c| !c.is_whitespace()))
        .collect();

    let num_logo_lines = lines.len();
    let max_logo_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

    let mut text_lines: Vec<Line> = Vec::new();

    let num_lines_for_gradient = if num_logo_lines <= 1 {
        1.0
    } else {
        (num_logo_lines - 1) as f32
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

    let logo_paragraph = Paragraph::new(text_lines).alignment(Alignment::Center);

    // Calculate the area for the logo within the main content chunk (outer_chunks[1])
    let logo_area_width = outer_chunks[1].width.saturating_sub(4); // Give some horizontal padding
    let logo_area_height = num_logo_lines as u16;

    let logo_x =
        outer_chunks[1].x + (outer_chunks[1].width.saturating_sub(max_logo_line_width)) / 2;
    let logo_y = outer_chunks[1].y + (outer_chunks[1].height.saturating_sub(logo_area_height)) / 2;

    let centered_logo_rect = Rect {
        x: logo_x.max(outer_chunks[1].x), // Ensure x is not less than chunk start
        y: logo_y.max(outer_chunks[1].y), // Ensure y is not less than chunk start
        width: max_logo_line_width.min(outer_chunks[1].width),
        height: logo_area_height.min(outer_chunks[1].height),
    };

    f.render_widget(logo_paragraph, centered_logo_rect);

    // Instructions Paragraph
    let instructions_text = if app_state.lock().unwrap().animation_finished {
        "Press any key to go to Chat, 'Q' to quit."
    } else {
        "Loading... Please wait."
    };

    let instructions = Paragraph::new(Line::from(instructions_text))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, outer_chunks[2]);
}
