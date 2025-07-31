use crate::themes::{interpolate_rgb, rgb_to_color, Theme};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub const ANIMATION_FRAMES: [&str; 6] = [
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
    r#"
 .S_sSSs      sSSs    sSSs  sdSS_SSSSSSbs   .S       S.    .S 
.SS~YS%%b    d%%SP   d%%SP  YSSS~S%SSSSSP  .SS       SS.  .SS 
S%S   `S%b  d%S'    d%S'         S%S       S%S       S%S  S%S 
S%S    S%S  S%S     S%S          S%S       S%S       S%S  S%S 
S%S    d*S  S&S     S&S          S&S       S&S       S&S  S&S 
S&S   .S*S  S&S_Ss  S&S_Ss       S&S       S&S       S&S  S&S 
S&S_sdSSS   S&S~SP  S&S~SP       S&S       S&S       S&S  S&S 
S&S~YSY%b   S&S     S&S          S&S       S&S       S&S  S&S 
S*S   `S%b  S*b     S*b          S*S       S*b       d*S  S*S 
S*S    S%S  S*S.    S*S.         S*S       S*S.     .S*S  S*S 
S*S    S&S   SSSbs   SSSbs       S*S        SSSbs_sdSSS   S*S 
S*S    SSS    YSSP    YSSP       S*S         YSSP~YSSY    S*S 
SP                               SP                       SP  
Y                                Y                        Y   
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
 ▄▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄▄▄  ▄         ▄  ▄▄▄▄▄▄▄▄▄▄▄ 
▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌▐░▌       ▐░▌▐░░░░░░░░░░░▌
▐░█▀▀▀▀▀▀▀█░▌▐░█▀▀▀▀▀▀▀▀▀ ▐░█▀▀▀▀▀▀▀▀▀  ▀▀▀▀█░█▀▀▀▀ ▐░▌       ▐░▌ ▀▀▀▀█░█▀▀▀▀ 
▐░▌  -w-  ▐░▌▐░▌          ▐░▌               ▐░▌     ▐░▌       ▐░▌     ▐░▌     
▐░█▄▄▄▄▄▄▄█░▌▐░█▄▄▄▄▄▄▄▄▄ ▐░█▄▄▄▄▄▄▄▄▄      ▐░▌     ▐░▌       ▐░▌     ▐░▌     
▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌     ▐░▌     ▐░▌       ▐░▌     ▐░▌     
▐░█▀▀▀▀█░█▀▀ ▐░█▀▀▀▀▀▀▀▀▀ ▐░█▀▀▀▀▀▀▀▀▀      ▐░▌     ▐░▌       ▐░▌     ▐░▌     
▐░▌     ▐░▌  ▐░▌          ▐░▌               ▐░▌     ▐░▌       ▐░▌     ▐░▌     
▐░▌      ▐░▌ ▐░█▄▄▄▄▄▄▄▄▄ ▐░█▄▄▄▄▄▄▄▄▄      ▐░▌     ▐░█▄▄▄▄▄▄▄█░▌ ▄▄▄▄█░█▄▄▄▄ 
▐░▌       ▐░▌▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌     ▐░▌     ▐░░░░░░░░░░░▌▐░░░░░░░░░░░▌
 ▀         ▀  ▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀       ▀       ▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀ 
"#,
    r#"
_|_|_|                      _|_|_|_|_|  _|    _|  _|_|_| 
_|    _|    _|_|      _|_|      _|      _|    _|    _|   
_|_|_|    _|_|_|_|  _|_|_|_|    _|      _|    _|    _|   
_|    _|  _|        _|          _|      _|    _|    _|   
_|    _|    _|_|_|    _|_|_|    _|        _|_|    _|_|_| 
"#,
    r#"
 ,ggggggggggg,                   ,ggggggggggggggg ,ggg,         gg       ,a8a, 
dP"""88"""""Y8,                dP""""""88"""""""dP""Y8a        88      ,8" "8,
Yb,  88      `8b                Yb,_    88       Yb, `88        88      d8   8b
 `"  88      ,8P                 `""    88        `"  88        88      88   88
     88aaaad8P"                         88            88        88      88   88
     88""""Yb,     ,ggg,    ,ggg,       88            88        88      Y8   8P
     88     "8b   i8" "8i  i8" "8i      88            88        88      `8, ,8'
     88      `8i  I8, ,8I  I8, ,8Igg,   88            88        88 8888  "8,8" 
     88       Yb, `YbadP'  `YbadP' "Yb,,8P   |\ /|    Y8b,____,d88,`8b,  ,d8b, 
     88        Y8888P"Y888888P"Y888  "Y8P'    (>v<)     "Y888888P"Y8  "Y88P" "Y8
"#, // don't ask my why                        i made this
];

pub const FRAME_DURATION_MS: u64 = 500;

pub fn draw_home_ui<B: Backend>(f: &mut Frame, current_frame_index: usize, theme: &Theme) {
    let size = f.area();

    let background = ratatui::widgets::Block::default()
        .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
    f.render_widget(background, size);

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
                &theme.colors.title_gradient_start,
                &theme.colors.title_gradient_end,
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
        .style(Style::default().fg(rgb_to_color(&theme.colors.instructions_text)))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);
}
