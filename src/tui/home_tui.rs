use crate::app::AppState;
use crate::tui::themes::{get_theme, interpolate_rgb, rgb_to_color, Theme, ThemeName};
use crate::tui::TuiPage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const ANIMATION_FRAMES: [&str; 5] = [
    // a collection of reetui logo, bc idon't know what i should
    // choose
    r#"
▄▄▄  ▄▄▄ .▄▄▄ .▄▄▄▄▄▄• ▄▌▪  
▀▄ █·▀▄.▀·▀▄.▀·•██  █▪██▌██ 
▐▀▀▄ ▐▀▀▪▄▐▀▀▪▄ ▐█.▪█▌▐█▌▐█·
▐█•█▌▐█▄▄▌▐█▄▄▌ ▐█▌·▐█▄█▌▐█▌
.▀  ▀ ▀▀▀  ▀▀▀  ▀▀▀  ▀▀▀ ▀▀▀
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
const FRAME_DURATION_MS: u64 = 600;

fn draw_home_ui<B: Backend>(f: &mut Frame, app_state: &mut AppState, theme: &Theme) {
    let size = f.area();

    f.render_widget(Block::default().bg(rgb_to_color(&theme.background)), size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(size);

    let current_frame_index = app_state.animation_frame_index;
    let current_frame_str = ANIMATION_FRAMES[current_frame_index];
    let lines: Vec<&str> = current_frame_str.lines().collect();

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

    let logo_area_height = num_logo_lines as u16;

    let logo_x = chunks[1].x + (chunks[1].width.saturating_sub(max_logo_line_width)) / 2;
    let logo_y = chunks[1].y + (chunks[1].height.saturating_sub(logo_area_height)) / 2;

    let centered_logo_rect = Rect {
        x: logo_x.max(chunks[1].x),
        y: logo_y.max(chunks[1].y),
        width: max_logo_line_width.min(chunks[1].width),
        height: logo_area_height.min(chunks[1].height),
    };
    f.render_widget(logo_paragraph, centered_logo_rect);
    let instructions_text = "Press any  key to go to Chat, 'Q' to quit.";

    let instructions = Paragraph::new(Line::from(instructions_text))
        .style(Style::default().fg(rgb_to_color(&theme.instructions_text)))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    let gemini_notice = Paragraph::new(Line::from(
        "This app may contain some code generated by Gemini.",
    ))
    .style(Style::default().fg(rgb_to_color(&theme.help_text)))
    .alignment(Alignment::Center);
    f.render_widget(gemini_notice, chunks[3]);
}

pub async fn run_home_page<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<tokio::sync::Mutex<AppState>>,
) -> io::Result<TuiPage> {
    let current_theme = get_theme(ThemeName::CatppuccinMocha);

    loop {
        let mut state_guard = app_state.lock().await;

        terminal.draw(|f| {
            draw_home_ui::<B>(f, &mut state_guard, &current_theme);
        })?;

        let now = Instant::now();
        let wait_time = Duration::from_millis(0);
        let elapsed_since_last_frame = now.duration_since(state_guard.last_frame_time);
        let required_duration_per_frame = Duration::from_millis(FRAME_DURATION_MS);

        if elapsed_since_last_frame >= required_duration_per_frame {
            state_guard.animation_frame_index =
                (state_guard.animation_frame_index + 1) % ANIMATION_FRAMES.len();
            state_guard.last_frame_time = now;
        } else {
            tokio::time::sleep(
                required_duration_per_frame.saturating_sub(elapsed_since_last_frame),
            )
            .await;
        }

        if event::poll(wait_time)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(TuiPage::Exit),
                        _ => return Ok(TuiPage::Chat),
                    }
                }
            }
        }
    }
}
