use crate::app::AppState;
use crate::tui::TuiPage; // Import TuiPage enum
                         // Import the necessary items from your themes module
use crate::tui::themes::{get_theme, interpolate_rgb, rgb_to_color, Rgb, Theme, ThemeName};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph}, // Removed Borders, BorderType from use
    Frame,
    Terminal,
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

// --- ASCII Art Animation Frames ---
const ANIMATION_FRAMES: [&str; 9] = [
    r#"
▄▄▄  ▄▄▄ .▄▄▄ .▄▄▄▄▄▄• ▄▌▪  
▀▄ █·▀▄.▀·▀▄.▀·•██  █▪██▌██ 
▐▀▀▄ ▐▀▀▪▄▐▀▀▪▄ ▐█.▪█▌▐█▌▐█·
▐█•█▌▐█▄▄▌▐█▄▄▌ ▐█▌·▐█▄█▌▐█▌
.▀  ▀ ▀▀▀  ▀▀▀  ▀▀▀  ▀▀▀ ▀▀▀
"#,
    r#"
 ________  _______   _______  _________  ___  ___  ___     
|\   __  \|\  ___ \ |\  ___ \|\___   ___\\  \|\  \|\  \    
\ \  \|\  \ \   __/|\ \   __/\|___ \  \_\ \  \\\  \ \  \   
 \ \   _  _\ \  \_|/_\ \  \_|/__  \ \  \ \ \  \\\  \ \  \  
  \ \  \\  \\ \  \_|\ \ \  \_|\ \  \ \  \ \ \  \\\  \ \  \ 
   \ \__\\ _\\ \_______\ \_______\  \ \__\ \ \_______\ \__\
    \|__|\|__|\|_______|\|_______|   \|__|  \|_______|\|__|
"#,
    r#"
     ___           ___           ___           ___           ___                 
    /\  \         /\  \         /\  \         /\  \         /\__\          ___   
   /::\  \       /::\  \       /::\  \        \:\  \       /:/  /         /\  \  
  /:/\:\  \     /:/\:\  \     /:/\:\  \        \:\  \     /:/  /          \:\  \ 
 /::\~\:\  \   /::\~\:\  \   /::\~\:\  \       /::\  \   /:/  /  ___      /::\__\
/:/\:\ \:\__\ /:/\:\ \:\__\ /:/\:\ \:\__\     /:/\:\__\ /:/__/  /\__\  __/:/\/__/
\/_|::\/:/  / \:\~\:\ \/__/ \:\~\:\ \/__/    /:/  \/__/ \:\  \ /:/  / /\/:/  /   
   |:|::/  /   \:\ \:\__\    \:\ \:\__\     /:/  /       \:\  /:/  /  \::/__/    
   |:|\/__/     \:\ \/__/     \:\ \/__/     \/__/         \:\/:/  /    \:\__\    
   |:|  |        \:\__\        \:\__\                      \::/  /      \/__/    
    \|__|         \/__/         \/__/                       \/__/                
"#,
    r#"
____/\\\\\\\\\____________________________________/\\\\\\\\\\\\\\\__/\\\________/\\\__/\\\\\\\\\\\_        
 __/\\\///////\\\_________________________________\///////\\\/////__\/\\\_______\/\\\_\/////\\\///__       
  _\/\\\_____\/\\\_______________________________________\/\\\_______\/\\\_______\/\\\_____\/\\\_____      
   _\/\\\\\\\\\\\/________/\\\\\\\\______/\\\\\\\\________\/\\\_______\/\\\_______\/\\\_____\/\\\_____     
    _\/\\\//////\\\______/\\\/////\\\___/\\\/////\\\_______\/\\\_______\/\\\_______\/\\\_____\/\\\_____    
     _\/\\\____\//\\\____/\\\\\\\\\\\___/\\\\\\\\\\\________\/\\\_______\/\\\_______\/\\\_____\/\\\_____   
      _\/\\\_____\//\\\__\//\\///////___\//\\///////_________\/\\\_______\//\\\______/\\\______\/\\\_____  
       _\/\\\______\//\\\__\//\\\\\\\\\\__\//\\\\\\\\\\_______\/\\\________\///\\\\\\\\\/____/\\\\\\\\\\\_ 
        _\///________\///____\//////////____\//////////________\///___________\/////////_____\///////////__
"#,
    r#"
   ▄████████    ▄████████    ▄████████     ███     ███    █▄   ▄█ 
  ███    ███   ███    ███   ███    ███ ▀█████████▄ ███    ███ ███ 
  ███    ███   ███    █▀    ███    █▀     ▀███▀▀██ ███    ███ ███▌
 ▄███▄▄▄▄██▀  ▄███▄▄▄      ▄███▄▄▄         ███   ▀ ███    ███ ███▌
▀▀███▀▀▀▀▀   ▀▀███▀▀▀     ▀▀███▀▀▀         ███     ███    ███ ███▌
▀███████████   ███    █▄    ███    █▄      ███     ███    ███ ███ 
  ███    ███   ███    ███   ███    ███     ███     ███    ███ ███ 
  ███    ███   ██████████   ██████████    ▄████▀   ████████▀  █▀  
  ███    ███                                                      
"#,
    r#"
 _____        _______ _    _ _____ 
|  __ \      |__   __| |  | |_   _|
| |__) |___  ___| |  | |  | | | |  
|  _  // _ \/ _ \ |  | |  | | | |  
| | \ \  __/  __/ |  | |__| |_| |_ 
|_|  \_\___|\___|_|   \____/|_____|
"#,
    r#"
 ██▀███  ▓█████ ▓█████▄▄▄█████▓ █    ██  ██▓
▓██ ▒ ██▒▓█   ▀ ▓█   ▀▓  ██▒ ▓▒ ██  ▓██▒▓██▒
▓██ ░▄█ ▒▒███   ▒███  ▒ ▓██░ ▒░▓██  ▒██░▒██▒
▒██▀▀█▄  ▒▓█  ▄ ▒▓█  ▄░ ▓██▓ ░ ▓▓█  ░██░░██░
░██▓ ▒██▒░▒████▒░▒████▒ ▒██▒ ░ ▒▒█████▓ ░██░
░ ▒▓ ░▒▓░░░ ▒░ ░░░ ▒░ ░ ▒ ░░   ░▒▓▒ ▒ ▒ ░▓  
  ░▒ ░ ▒░ ░ ░  ░ ░ ░  ░   ░    ░░▒░ ░ ░  ▒ ░
  ░░   ░    ░      ░    ░       ░░░ ░ ░  ▒ ░
   ░        ░  ░   ░  ░           ░      ░  
"#,
    r#"
██████╗ ███████╗███████╗████████╗██╗   ██╗██╗
██╔══██╗██╔════╝██╔════╝╚══██╔══╝██║   ██║██║
██████╔╝█████╗  █████╗     ██║   ██║   ██║██║
██╔══██╗██╔══╝  ██╔══╝     ██║   ██║   ██║██║
██║  ██║███████╗███████╗   ██║   ╚██████╔╝██║
╚═╝  ╚═╝╚══════╝╚══════╝   ╚═╝    ╚═════╝ ╚═╝
"#,
    r#"
 ███████████                     ███████████ █████  █████ █████
░░███░░░░░███                   ░█░░░███░░░█░░███  ░░███ ░░███ 
 ░███    ░███   ██████   ██████ ░   ░███  ░  ░███   ░███  ░███ 
 ░██████████   ███░░███ ███░░███    ░███     ░███   ░███  ░███ 
 ░███░░░░░███ ░███████ ░███████     ░███     ░███   ░███  ░███ 
 ░███    ░███ ░███░░░  ░███░░░      ░███     ░███   ░███  ░███ 
 █████   █████░░██████ ░░██████     █████    ░░████████   █████
░░░░░   ░░░░░  ░░░░░░   ░░░░░░     ░░░░░      ░░░░░░░░   ░░░░░ 
"#,
];
const FRAME_DURATION_MS: u64 = 1500; // ms

pub async fn run_home_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let current_theme = get_theme(ThemeName::CatppuccinMocha);

    loop {
        // Draw the UI
        terminal.draw(|f| {
            draw_home_ui::<B>(f, app_state.clone(), &current_theme);
        })?;

        // --- Animation Update and Dynamic Polling Logic ---
        let mut state = app_state.lock().unwrap();
        let mut wait_time = Duration::from_millis(0); // Default to no wait if animation needs update

        let now = Instant::now();
        let elapsed_since_last_frame = now.duration_since(state.last_frame_time);
        let required_duration_per_frame = Duration::from_millis(FRAME_DURATION_MS);

        if elapsed_since_last_frame >= required_duration_per_frame {
            // Enough time has passed for the next frame, update it
            // Use modulo to loop the animation infinitely
            state.animation_frame_index =
                (state.animation_frame_index + 1) % ANIMATION_FRAMES.len();
            state.last_frame_time = now; // Reset last frame time
            wait_time = Duration::from_millis(0); // Don't wait, draw next frame immediately
        } else {
            // Not enough time has passed, calculate how long to wait until the next frame is due
            wait_time = required_duration_per_frame.saturating_sub(elapsed_since_last_frame);
        }

        drop(state); // bye bye

        // Handle user input events with the calculated dynamic wait time
        if event::poll(wait_time)? {
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

    // Set the entire background color of the terminal frame
    f.render_widget(Block::default().bg(rgb_to_color(&theme.background)), size);

    // Define main layout for the screen content without borders
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2), // Top padding/empty space
                Constraint::Min(0),    // Flexible space for ASCII logo (this will be chunks[1])
                Constraint::Length(1), // Instructions (this will be chunks[2])
                Constraint::Length(1), // Gemini notice (this will be chunks[3])
                Constraint::Length(1), // Bottom padding/empty space
            ]
            .as_ref(),
        )
        .split(size);

    // --- ASCII Animation Drawing ---
    let current_frame_index = app_state.lock().unwrap().animation_frame_index;
    let current_frame_str = ANIMATION_FRAMES[current_frame_index];

    // Removed the filter to ensure all lines, including those with only whitespace, are preserved.
    // This should fix "deformed" lines if they were caused by the filter removing intended spacing.
    let lines: Vec<&str> = current_frame_str.lines().collect(); // No filter here

    let num_logo_lines = lines.len();
    let max_logo_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

    let mut text_lines: Vec<Line> = Vec::new();

    // Calculate gradient progression based on line index
    let num_lines_for_gradient = if num_logo_lines <= 1 {
        1.0
    } else {
        (num_logo_lines - 1) as f32
    };

    for (line_idx, line_str) in lines.iter().enumerate() {
        let mut spans = Vec::new();
        // Calculate the vertical fraction for the gradient
        let fraction_y = line_idx as f32 / num_lines_for_gradient;

        for ch in line_str.chars() {
            // Apply gradient to each character
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

    let logo_area_height = num_logo_lines as u16;

    let logo_x = chunks[1].x + (chunks[1].width.saturating_sub(max_logo_line_width)) / 2;
    let logo_y = chunks[1].y + (chunks[1].height.saturating_sub(logo_area_height)) / 2;

    let centered_logo_rect = Rect {
        x: logo_x.max(chunks[1].x), // Ensure x is not less than chunk start
        y: logo_y.max(chunks[1].y), // Ensure y is not less than chunk start
        width: max_logo_line_width.min(chunks[1].width),
        height: logo_area_height.min(chunks[1].height),
    };

    f.render_widget(logo_paragraph, centered_logo_rect);

    // Instructions Paragraph (in chunks[2])
    // The instructions no longer depend on `animation_finished` as it's an infinite loop
    let instructions_text = "Press any key to go to Chat, 'Q' to quit.";

    let instructions = Paragraph::new(Line::from(instructions_text))
        .style(Style::default().fg(rgb_to_color(&theme.instructions_text)))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]); // Render in chunks[2]

    // Gemini Notice (in chunks[3])
    let gemini_notice = Paragraph::new(Line::from(
        "This app may contain some code generated by Gemini.",
    ))
    .style(Style::default().fg(rgb_to_color(&theme.help_text))) // Using help_text for a slightly dimmed look
    .alignment(Alignment::Center);
    f.render_widget(gemini_notice, chunks[3]); // Render in chunks[3]
}
