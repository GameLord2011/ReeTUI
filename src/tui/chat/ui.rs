use crate::api::models::BroadcastMessage;
use crate::app::{AppState, PopupType};
use crate::tui::chat::create_channel_form::CreateChannelForm;
use crate::tui::chat::popups::create_channel::{
    draw_create_channel_popup, get_create_channel_popup_size,
};
use crate::tui::chat::popups::debug_json::{draw_debug_json_popup, get_debug_json_popup_size};
use crate::tui::chat::popups::deconnection::{
    draw_deconnection_popup, get_deconnection_popup_size,
};
use crate::tui::chat::popups::download_progress::{
    draw_download_progress_popup, get_download_progress_popup_size,
};
use crate::tui::chat::popups::emojis::{draw_emojis_popup, get_emojis_popup_size};
use crate::tui::chat::popups::file_manager::FileManager;
use crate::tui::chat::popups::help::{draw_help_popup, get_help_popup_size};
use crate::tui::chat::popups::mentions::{draw_mentions_popup, get_mentions_popup_size};
use crate::tui::chat::popups::quit::{draw_quit_popup, get_quit_popup_size};
use crate::tui::chat::popups::set_theme::{draw_set_theme_popup, get_set_theme_popup_size};
use crate::tui::chat::popups::settings::{draw_settings_popup, get_settings_popup_size};
use crate::tui::chat::theme_settings_form::ThemeSettingsForm;
use crate::tui::chat::utils::{centered_rect, get_color_for_user};
use crate::tui::themes::{get_contrasting_text_color, get_theme, rgb_to_color, Theme};
use chrono::{TimeZone, Utc};
use sha2::Digest;

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use regex::Regex;
use std::collections::VecDeque;

pub fn draw_chat_ui<B: Backend>(
    f: &mut Frame,
    state: &mut AppState,
    input_text: &str,
    channel_list_state: &mut ListState,
    create_channel_form: &mut CreateChannelForm,
    theme_settings_form: &mut ThemeSettingsForm,
    file_manager: &mut FileManager,
    filtered_users: &Vec<String>,
    filtered_emojis: &Vec<String>,
    mention_regex: &Regex,
    emoji_regex: &Regex,
) {
    let size = f.area();
    let current_theme = get_theme(state.current_theme);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(size);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)]) // Channels and then user info
        .split(chunks[0]);

    let channels_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Channels")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.border_focus))
                .bg(rgb_to_color(&current_theme.background)),
        );
    let channel_items: Vec<ListItem> = state
        .channels
        .iter()
        .map(|channel| {
            let is_current = state
                .current_channel
                .as_ref()
                .map_or(false, |c| c.id == channel.id);
            let style = if is_current {
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(rgb_to_color(&current_theme.accent))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.text))
            };
            ListItem::new(format!("{} {}", channel.icon, channel.name.as_str())).style(style)
        })
        .collect();
    let channels_list = List::new(channel_items)
        .block(channels_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.button_text_active))
                .bg(rgb_to_color(&current_theme.button_bg_active)),
        )
        .highlight_symbol(" ");
    f.render_stateful_widget(channels_list, left_chunks[0], channel_list_state);

    // User Info Section
    let user_info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("User Info")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.border_focus))
                .bg(rgb_to_color(&current_theme.background)),
        );

    let username_text = state.username.clone().unwrap_or_default();
    let user_icon = state.user_icon.clone().unwrap_or_default();

    let user_info_paragraph = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", user_icon),
            Style::default().fg(rgb_to_color(&current_theme.text)),
        ),
        Span::styled(
            username_text,
            Style::default()
                .fg(rgb_to_color(&current_theme.accent))
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(user_info_block);

    f.render_widget(user_info_paragraph, left_chunks[1]);

    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(chunks[1]);
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            "Messages - {}",
            state
                .current_channel
                .as_ref()
                .map_or("No Channel Selected".to_string(), |c| c.name.clone())
        ))
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.text))
                .bg(rgb_to_color(&current_theme.background)),
        );
    let inner_messages_area = messages_block.inner(chat_chunks[0]);
    f.render_widget(messages_block, chat_chunks[0]);
    if let Some(current_channel) = &state.current_channel {
        let channel_id = &current_channel.id;
        if state.rendered_messages.get(channel_id).is_none()
            || state.rendered_messages.get(channel_id).unwrap().len()
                != state.messages.get(channel_id).unwrap().len()
        {
            let messages = state.get_messages_for_channel(channel_id).unwrap();
            let mut new_rendered_messages = VecDeque::new();
            let mut last_user: Option<String> = None;
            for msg in messages.iter() {
                format_message_lines(
                    msg,
                    &current_theme,
                    inner_messages_area.width,
                    &last_user,
                    &mut new_rendered_messages,
                    &mention_regex,
                    &emoji_regex,
                );
                last_user = Some(msg.user.clone());
            }
            state
                .rendered_messages
                .insert(channel_id.clone(), new_rendered_messages.into());
        }
        let rendered_lines = state.rendered_messages.get(channel_id).unwrap();
        let messages_to_render = {
            let message_count = rendered_lines.len();
            let view_height = inner_messages_area.height as usize;
            let scroll_offset = state.message_scroll_offset;
            let start_index = message_count.saturating_sub(view_height + scroll_offset);
            let end_index = message_count.saturating_sub(scroll_offset);
            if message_count > view_height {
                &rendered_lines[start_index..end_index]
            } else {
                rendered_lines.as_ref()
            }
        };
        let messages_paragraph = Paragraph::new(messages_to_render.to_vec())
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(messages_paragraph, inner_messages_area);
    }
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Input")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.input_border_active))
                .bg(rgb_to_color(&current_theme.background)),
        );
    let input_lines = input_text.split('\n').count();
    let input_height = (input_lines as u16 + 2).min(chat_chunks[1].height);
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(input_height)])
        .split(chat_chunks[1])[1];
    let input_paragraph = Paragraph::new(Text::from(input_text))
        .block(input_block)
        .style(Style::default().fg(rgb_to_color(&current_theme.input_text_active)));
    f.render_widget(input_paragraph, input_area);

    if state.popup_state.show {
        let popup_title = match state.popup_state.popup_type {
            PopupType::Quit => "Quit",
            PopupType::Settings => "Settings",
            PopupType::CreateChannel => "Create Channel",
            PopupType::SetTheme => "Select Theme",
            PopupType::Deconnection => "Deconnection",
            PopupType::Help => "Help - Commands",
            PopupType::None => "",
            PopupType::Mentions => "Mentions",
            PopupType::Emojis => "Emojis",
            PopupType::FileManager => "File Manager",
            PopupType::DownloadProgress => "Downloading",
            PopupType::DebugJson => "Debug JSON",
            PopupType::Notification => "Notification",
        };
        let popup_block_widget = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(popup_title)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.popup_border))
                    .bg(rgb_to_color(&current_theme.background)),
            );

        let (popup_width, popup_height) = match state.popup_state.popup_type {
            PopupType::Quit => get_quit_popup_size(),
            PopupType::Deconnection => get_deconnection_popup_size(),
            PopupType::Settings => get_settings_popup_size(),
            PopupType::CreateChannel => get_create_channel_popup_size(),
            PopupType::SetTheme => get_set_theme_popup_size(theme_settings_form),
            PopupType::Help => get_help_popup_size(),
            PopupType::Mentions => {
                let (_, height) = get_mentions_popup_size(state); // Get height from mentions module
                let max_width = filtered_users
                    .iter()
                    .map(|user| user.len() as u16 + 4)
                    .max()
                    .unwrap_or(0)
                    .min(size.width - 4);
                let width = max_width.max(20);
                (width, height)
            }
            PopupType::Emojis => {
                let (_, height) = get_emojis_popup_size(state); // Get height from emojis module
                let max_width = filtered_emojis
                    .iter()
                    .map(|emoji_str| (emoji_str.len() + 3) as u16)
                    .max()
                    .unwrap_or(0)
                    .min(size.width - 4);
                let width = max_width.max(20);
                (width, height)
            }
            PopupType::FileManager => (90, 90),
            PopupType::DownloadProgress => get_download_progress_popup_size(),
            PopupType::DebugJson => get_debug_json_popup_size(),
            _ => (0, 0),
        };

        let popup_area = match state.popup_state.popup_type {
            PopupType::Mentions | PopupType::Emojis => {
                let input_area_y = chat_chunks[1].y + chat_chunks[1].height - input_height;
                Rect::new(
                    chat_chunks[1].x,
                    input_area_y.saturating_sub(popup_height + 1),
                    popup_width,
                    popup_height,
                )
            }
            _ => centered_rect(popup_width, popup_height, size),
        };

        f.render_widget(Clear, popup_area);
        f.render_widget(&popup_block_widget, popup_area);
        match state.popup_state.popup_type {
            PopupType::Quit => {
                draw_quit_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::Settings => {
                draw_settings_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::CreateChannel => {
                draw_create_channel_popup(
                    f,
                    state,
                    popup_area,
                    create_channel_form,
                    &popup_block_widget,
                );
            }
            PopupType::SetTheme => {
                draw_set_theme_popup(
                    f,
                    state,
                    popup_area,
                    theme_settings_form,
                    &popup_block_widget,
                );
            }
            PopupType::Deconnection => {
                draw_deconnection_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::Help => {
                draw_help_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::Mentions => {
                draw_mentions_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::Emojis => {
                draw_emojis_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::FileManager => {
                file_manager.ui(f);
            }
            PopupType::DownloadProgress => {
                draw_download_progress_popup(f, popup_area, state.download_progress);
            }
            PopupType::DebugJson => {
                draw_debug_json_popup(f, popup_area, &state.debug_json_content);
            }
            _ => { /* No specific rendering for other popup types yet */ }
        }
    }
}
pub fn format_message_lines(
    msg: &BroadcastMessage,
    theme: &Theme,
    width: u16,
    last_user: &Option<String>,
    lines: &mut VecDeque<Line<'static>>,
    mention_regex: &Regex,
    emoji_regex: &Regex,
) {
    let timestamp_str = Utc
        .timestamp_opt(msg.timestamp, 0)
        .unwrap()
        .format("%H:%M")
        .to_string();
    let user_color = get_color_for_user(&msg.user);

    let mut message_content_spans = Vec::new();

    if msg.message_type == "file" {
        if msg.is_image.unwrap_or(false) {
            if let Some(download_url) = &msg.download_url {
                let mut hasher = sha2::Sha256::new();
                hasher.update(download_url.as_bytes());
                let hash_result = hasher.finalize();
                let file_hash = format!("{:x}", hash_result);

                let cache_dir = std::env::temp_dir().join("ReeTUI_cache");
                if !cache_dir.exists() {
                    std::fs::create_dir_all(&cache_dir).unwrap_or_default();
                }
                let file_name = msg.file_name.clone().unwrap_or_default();
                let file_extension = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("tmp");
                let cached_image_path =
                    cache_dir.join(format!("{}.{}", &file_hash, file_extension));

                if last_user.as_ref() != Some(&msg.user) {
                    let header_spans = vec![
                        Span::styled("╭ ", Style::default().fg(user_color)),
                        Span::styled(
                            format!("{} ", msg.icon),
                            Style::default().fg(rgb_to_color(&theme.text)),
                        ),
                        Span::styled(
                            msg.user.as_str().to_string(),
                            Style::default().fg(user_color).add_modifier(Modifier::BOLD),
                        ),
                    ];
                    let header_width = header_spans.iter().map(|s| s.width()).sum::<usize>();
                    let available_width = width as usize;
                    let timestamp_width = timestamp_str.len();
                    let mut header_line_spans = header_spans;
                    if available_width > header_width + timestamp_width + 1 {
                        let padding = available_width
                            .saturating_sub(header_width)
                            .saturating_sub(timestamp_width);
                        header_line_spans.push(Span::raw(" ".repeat(padding)));
                        header_line_spans.push(Span::styled(
                            timestamp_str.clone(),
                            Style::default().fg(rgb_to_color(&theme.dim)),
                        ));
                    } else {
                        header_line_spans.push(Span::raw(" "));
                        header_line_spans.push(Span::styled(
                            timestamp_str.clone(),
                            Style::default().fg(rgb_to_color(&theme.dim)),
                        ));
                    }
                    lines.push_back(Line::from(header_line_spans));
                }

                if cached_image_path.exists() {
                    if let Some(preview) = &msg.image_preview {
                        let text = Text::from(preview.clone());
                        for line in text.lines {
                            let mut spans = vec![Span::styled(
                                "│ ",
                                Style::default().fg(rgb_to_color(&theme.dim)),
                            )];
                            spans.extend(line.spans);
                            lines.push_back(Line::from(spans));
                        }
                    } else {
                        lines.push_back(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(rgb_to_color(&theme.dim))),
                            Span::styled(
                                format!(
                                    "Image preview not available for {}",
                                    msg.file_name.as_deref().unwrap_or("image")
                                ),
                                Style::default()
                                    .fg(rgb_to_color(&theme.dim))
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    }
                } else {
                    lines.push_back(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(rgb_to_color(&theme.dim))),
                        Span::styled(
                            format!(
                                "Image not yet downloaded: {}",
                                msg.file_name.as_deref().unwrap_or("image")
                            ),
                            Style::default()
                                .fg(rgb_to_color(&theme.dim))
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }

                lines.push_back(Line::from(vec![
                    Span::styled("│ ", Style::default().fg(rgb_to_color(&theme.dim))),
                    Span::styled(
                        format!(
                            "Download with: /download {}",
                            msg.file_id.as_deref().unwrap_or("")
                        ),
                        Style::default()
                            .fg(rgb_to_color(&theme.dim))
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                return;
            } else {
                message_content_spans.push(Span::styled(
                    format!(
                        "Image file with no download URL: {}",
                        msg.file_name.as_deref().unwrap_or("image")
                    ),
                    Style::default()
                        .fg(rgb_to_color(&theme.error))
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        } else {
            // This else is for non-image files within "file" type
            message_content_spans.push(Span::styled(
                format!(
                    "{} {}.{} ({} MB)",
                    msg.file_icon.as_deref().unwrap_or("📁"),
                    msg.file_name.as_deref().unwrap_or("Unknown"),
                    msg.file_extension.as_deref().unwrap_or(""),
                    msg.file_size_mb.unwrap_or(0.0)
                ),
                Style::default()
                    .fg(rgb_to_color(&theme.accent))
                    .add_modifier(Modifier::BOLD),
            ));
            if let Some(download_url) = &msg.download_url {
                message_content_spans.push(Span::raw("\n"));
                message_content_spans.push(Span::styled(
                    format!("Download available: {}", download_url),
                    Style::default()
                        .fg(rgb_to_color(&theme.help_text))
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }
    } else {
        // This else is for non-file messages (i.e., text messages)
        let mut current_text_slice = msg.content.as_str();
        while !current_text_slice.is_empty() {
            if let Some(mention_match) = mention_regex.find(current_text_slice) {
                let (before_mention, after_mention) =
                    current_text_slice.split_at(mention_match.start());
                let mut temp_slice = before_mention;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    message_content_spans.push(
                        Span::raw(temp_slice[..emoji_match.start()].to_string())
                            .fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        message_content_spans
                            .push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        message_content_spans.push(
                            Span::raw(emoji_match.as_str().to_string())
                                .fg(rgb_to_color(&theme.text)),
                        );
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                message_content_spans
                    .push(Span::raw(temp_slice.to_string()).fg(rgb_to_color(&theme.text)));
                message_content_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                message_content_spans.push(Span::styled(
                    mention_match.as_str().to_string(),
                    Style::default()
                        .fg(get_contrasting_text_color(&theme.mention_bg))
                        .bg(rgb_to_color(&theme.mention_bg)),
                ));
                message_content_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                current_text_slice = &after_mention[mention_match.len()..];
            } else {
                let mut temp_slice = current_text_slice;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    message_content_spans.push(
                        Span::raw(temp_slice[..emoji_match.start()].to_string())
                            .fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        message_content_spans
                            .push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        message_content_spans.push(
                            Span::raw(emoji_match.as_str().to_string())
                                .fg(rgb_to_color(&theme.text)),
                        );
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                message_content_spans
                    .push(Span::raw(temp_slice.to_string()).fg(rgb_to_color(&theme.text)));
                current_text_slice = "";
            }
        }
    }

    if last_user.as_ref() == Some(&msg.user) {
        lines.push_back(Line::from(
            std::iter::once(Span::styled(
                "│ ",
                Style::default().fg(rgb_to_color(&theme.dim)),
            ))
            .chain(message_content_spans.into_iter())
            .collect::<Vec<Span>>(),
        ));
    } else {
        let header_spans = vec![
            Span::styled("╭ ", Style::default().fg(user_color)),
            Span::styled(
                format!("{} ", msg.icon),
                Style::default().fg(rgb_to_color(&theme.text)),
            ),
            Span::styled(
                msg.user.as_str().to_string(),
                Style::default().fg(user_color).add_modifier(Modifier::BOLD),
            ),
        ];
        let header_width = header_spans.iter().map(|s| s.width()).sum::<usize>();
        let available_width = width as usize;
        let timestamp_width = timestamp_str.len();
        let mut header_line_spans = header_spans;
        if available_width > header_width + timestamp_width + 1 {
            let padding = available_width
                .saturating_sub(header_width)
                .saturating_sub(timestamp_width);
            header_line_spans.push(Span::raw(" ".repeat(padding)));
            header_line_spans.push(Span::styled(
                timestamp_str.clone(),
                Style::default().fg(rgb_to_color(&theme.dim)),
            ));
        } else {
            header_line_spans.push(Span::raw(" "));
            header_line_spans.push(Span::styled(
                timestamp_str.clone(),
                Style::default().fg(rgb_to_color(&theme.dim)),
            ));
        }
        lines.push_back(Line::from(header_line_spans));
        lines.push_back(Line::from(
            std::iter::once(Span::styled(
                "│ ",
                Style::default().fg(rgb_to_color(&theme.dim)),
            ))
            .chain(message_content_spans.into_iter())
            .collect::<Vec<Span>>(),
        ));
    }
}
