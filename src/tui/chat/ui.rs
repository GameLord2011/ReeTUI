use crate::api::models::BroadcastMessage;
use crate::app::{AppState, PopupType};
use crate::themes::{get_contrasting_text_color, rgb_to_color, Theme};
use crate::tui::chat::create_channel_form::CreateChannelForm;
use crate::tui::chat::gif_renderer::GifAnimationState;
use crate::tui::chat::popups::create_channel::{
    draw_create_channel_popup, get_create_channel_popup_size,
};

use crate::tui::chat::popups::deconnection::{
    draw_deconnection_popup, get_deconnection_popup_size,
};
use crate::tui::chat::popups::download_progress::{
    draw_download_progress_popup, get_download_progress_popup_size,
};
use crate::tui::chat::popups::emojis::{draw_emojis_popup, get_emojis_popup_size};
use crate::tui::chat::popups::helpers::get_file_manager_popup_size;
use crate::tui::chat::popups::mentions::{draw_mentions_popup, get_mentions_popup_size};

use crate::tui::file_manager_module::file_manager::FileManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::tui::chat::utils::{centered_rect, get_color_for_user};
use ansi_to_tui::IntoText as _;
use chrono::{TimeZone, Utc};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::tui::notification::ui::draw_notifications;
use crate::tui::settings;
use ratatui::Frame;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use regex::Regex;

fn wrap_spans<'a>(spans: Vec<Span<'a>>, max_width: u16) -> Vec<Line<'a>> {
    let mut lines = vec![Line::default()];
    let mut current_width = 0;

    for span in spans {
        for c in span.content.chars() {
            let char_width = c.width().unwrap_or(0) as u16;
            if current_width + char_width > max_width && current_width > 0 {
                lines.push(Line::default());
                current_width = 0;
            }
            let last_line = lines.last_mut().unwrap();
            last_line
                .spans
                .push(Span::styled(c.to_string(), span.style));
            current_width += char_width;
        }
    }
    lines
}

pub fn draw_chat_ui<B: Backend>(
    f: &mut Frame<'_>,
    state: &mut AppState,
    input_text: &str,
    channel_list_state: &mut ListState,
    create_channel_form: &mut CreateChannelForm,
    file_manager: &mut FileManager,
    mention_regex: &Regex,
    emoji_regex: &Regex,
    settings_state: &mut settings::state::SettingsState,
) {
    let size = f.area();
    let current_theme = state.current_theme.clone();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(size);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(chunks[0]);

    let channels_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Channels")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.colors.border_focus))
                .bg(rgb_to_color(&current_theme.colors.background)),
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
                    .fg(rgb_to_color(&current_theme.colors.accent))
            } else {
                Style::default().fg(rgb_to_color(&current_theme.colors.text))
            };
            ListItem::new(format!("{} {}", channel.icon, channel.name.as_str())).style(style)
        })
        .collect();
    let channels_list = List::new(channel_items)
        .block(channels_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(rgb_to_color(&current_theme.colors.button_text_active))
                .bg(rgb_to_color(&current_theme.colors.button_bg_active)),
        )
        .highlight_symbol(" ");
    f.render_stateful_widget(channels_list, left_chunks[0], channel_list_state);

    let user_info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("User Info")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.colors.border_focus))
                .bg(rgb_to_color(&current_theme.colors.background)),
        );

    let username_text = state.username.clone().unwrap_or_default();
    let user_icon = state.user_icon.clone().unwrap_or_default();

    let user_info_paragraph = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", user_icon),
            Style::default().fg(rgb_to_color(&current_theme.colors.text)),
        ),
        Span::styled(
            username_text,
            Style::default()
                .fg(rgb_to_color(&current_theme.colors.accent))
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
                .fg(rgb_to_color(&current_theme.colors.text))
                .bg(rgb_to_color(&current_theme.colors.background)),
        );
    let inner_messages_area = messages_block.inner(chat_chunks[0]);
    state.chat_width = inner_messages_area.width;
    f.render_widget(messages_block, chat_chunks[0]);
    let current_channel_clone = state.current_channel.clone();
    if let Some(current_channel) = &current_channel_clone {
        let channel_id = &current_channel.id;
        let mut all_rendered_lines: Vec<Line<'static>> = Vec::new();

        if let Some(messages) = state.messages.get(channel_id) {
            for i in 0..messages.len() {
                let msg = &messages[i];
                let message_id = msg
                    .file_id
                    .clone()
                    .unwrap_or_else(|| msg.timestamp.to_string());
                let needs_re_render_this_message = state
                    .needs_re_render
                    .get(channel_id)
                    .and_then(|channel_map| channel_map.get(&message_id).copied())
                    .unwrap_or(true);

                let mut is_first_in_group = true;
                let mut is_last_in_group = true;

                if i > 0 {
                    let prev_msg = &messages[i - 1];
                    if prev_msg.user == msg.user
                        && (msg.timestamp - prev_msg.timestamp).abs() < 60
                        && prev_msg.file_id.is_none()
                        && !prev_msg.is_image.unwrap_or(false)
                        && msg.file_id.is_none()
                        && !msg.is_image.unwrap_or(false)
                    {
                        is_first_in_group = false;
                    }
                }

                if i < messages.len() - 1 {
                    let next_msg = &messages[i + 1];
                    if next_msg.user == msg.user
                        && (next_msg.timestamp - msg.timestamp).abs() < 60
                        && next_msg.file_id.is_none()
                        && !next_msg.is_image.unwrap_or(false)
                        && msg.file_id.is_none()
                        && !msg.is_image.unwrap_or(false)
                    {
                        is_last_in_group = false;
                    }
                }

                if needs_re_render_this_message || !is_first_in_group {
                    let lines = format_message_lines(
                        msg,
                        &state.current_theme,
                        inner_messages_area.width,
                        mention_regex,
                        emoji_regex,
                        &state.active_animations,
                        is_first_in_group,
                        is_last_in_group,
                    );
                    state
                        .rendered_messages
                        .entry(channel_id.clone())
                        .or_default()
                        .insert(message_id.clone(), lines.clone());
                    state
                        .needs_re_render
                        .entry(channel_id.clone())
                        .or_default()
                        .insert(message_id.clone(), false);
                    all_rendered_lines.extend(lines);
                } else {
                    if let Some(lines) = state
                        .rendered_messages
                        .get(channel_id)
                        .and_then(|channel_map| channel_map.get(&message_id))
                    {
                        all_rendered_lines.extend(lines.clone());
                    } else {
                        let lines = format_message_lines(
                            msg,
                            &state.current_theme,
                            inner_messages_area.width,
                            mention_regex,
                            emoji_regex,
                            &state.active_animations,
                            is_first_in_group,
                            is_last_in_group,
                        );
                        state
                            .rendered_messages
                            .entry(channel_id.clone())
                            .or_default()
                            .insert(message_id.clone(), lines.clone());
                        state
                            .needs_re_render
                            .entry(channel_id.clone())
                            .or_default()
                            .insert(message_id.clone(), false);
                        all_rendered_lines.extend(lines);
                    }
                }
            }
        }
        state.total_chat_buffer_length = all_rendered_lines.len();

        let messages_paragraph = Paragraph::new({
            let message_count = all_rendered_lines.len();
            let view_height = inner_messages_area.height as usize;
            state.last_chat_view_height = view_height;

            let max_offset = message_count.saturating_sub(view_height).max(0);
            let scroll_offset = state.message_scroll_offset.min(max_offset);

            let start_index = message_count
                .saturating_sub(view_height)
                .saturating_sub(scroll_offset)
                .min(message_count);
            let end_index = message_count
                .saturating_sub(scroll_offset)
                .min(message_count);

            log::debug!(
                "ui: message_count={} view_height={} scroll_offset={} start_index={} end_index={}",
                message_count,
                view_height,
                scroll_offset,
                start_index,
                end_index
            );

            if message_count > view_height {
                all_rendered_lines[start_index..end_index].to_vec()
            } else {
                all_rendered_lines.to_vec()
            }
        })
        .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(messages_paragraph, inner_messages_area);
    }
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Input")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.colors.input_border_active))
                .bg(rgb_to_color(&current_theme.colors.background)),
        );
    let input_lines = input_text.split('\n').count();
    let input_height = (input_lines as u16 + 2).min(chat_chunks[1].height);
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(input_height)])
        .split(chat_chunks[1])[1];
    let input_paragraph = Paragraph::new(Text::from(input_text))
        .block(input_block)
        .style(Style::default().fg(rgb_to_color(&current_theme.colors.input_text_active)));
    f.render_widget(input_paragraph, input_area);

    f.set_cursor_position((
        input_area.x + state.cursor_position as u16 + 1,
        input_area.y + 1,
    ));

    if state.popup_state.show {
        log::debug!(
            "ui: Popup is shown, type: {:?}",
            state.popup_state.popup_type
        );
        let popup_title = match state.popup_state.popup_type {
            PopupType::CreateChannel => "Create Channel",
            PopupType::Deconnection => "Deconnection",
            PopupType::None => "",
            PopupType::Mentions => "Mentions",
            PopupType::Emojis => "Emojis",
            PopupType::FileManager => "File Manager",
            PopupType::DownloadProgress => "Downloading",
            
            PopupType::Downloads => "Downloads",
            PopupType::Notification => "Notification",
            PopupType::Settings => "Settings",
        };
        let popup_block_widget = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(popup_title)
            .title_alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.colors.popup_border))
                    .bg(rgb_to_color(&current_theme.colors.background)),
            );

        let (popup_width, popup_height) = match state.popup_state.popup_type {
            PopupType::Deconnection => get_deconnection_popup_size(),
            PopupType::CreateChannel => get_create_channel_popup_size(),
            PopupType::Mentions => get_mentions_popup_size(state),
            PopupType::Emojis => get_emojis_popup_size(state),
            PopupType::FileManager => get_file_manager_popup_size(),
            PopupType::DownloadProgress => get_download_progress_popup_size(),
            
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
            PopupType::CreateChannel => {
                draw_create_channel_popup(
                    f,
                    state,
                    popup_area,
                    create_channel_form,
                    &popup_block_widget,
                );
            }
            PopupType::Deconnection => {
                draw_deconnection_popup(f, state, popup_area, &popup_block_widget);
            }

            PopupType::Mentions => {
                draw_mentions_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::Emojis => {
                draw_emojis_popup(f, state, popup_area, &popup_block_widget);
            }
            PopupType::FileManager => {
                file_manager.ui(f, popup_area, state);
            }
            PopupType::DownloadProgress => {
                draw_download_progress_popup(f, popup_area, state.download_progress);
            }
            
            PopupType::Downloads => {
                crate::tui::chat::popups::downloads::draw_downloads_popup(f, state);
            }
            _ => {}
        }
    }

    if state.show_settings {
        settings::render_settings_popup::<B>(f, &state, settings_state, f.area()).unwrap();
    }

    draw_notifications(f, state);
}

pub fn format_message_lines(
    msg: &BroadcastMessage,
    theme: &Theme,
    width: u16,
    mention_regex: &Regex,
    emoji_regex: &Regex,
    active_animations: &HashMap<String, Arc<Mutex<GifAnimationState>>>,
    is_first_in_group: bool,
    is_last_in_group: bool,
) -> Vec<Line<'static>> {
    let mut content_lines = Vec::new();
    let timestamp_str = Utc
        .timestamp_opt(msg.timestamp, 0)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%H:%M")
        .to_string();
    let user_color = get_color_for_user(&msg.user);
    let is_special_message = msg.file_id.is_some() || msg.is_image.unwrap_or(false);

    if msg.is_image.unwrap_or(false) {
        if let Some(_gif_frames) = &msg.gif_frames {
            if let Some(file_id) = &msg.file_id {
                if let Some(animation_state_arc) = active_animations.get(file_id) {
                    let animation_state = match animation_state_arc.try_lock() {
                        Ok(state) => state,
                        Err(_) => return Vec::new(),
                    };
                    let frame_count = animation_state.frames.len();
                    let frame_index = animation_state.current_frame;
                    if frame_count > 0 && frame_index < frame_count {
                        let current_frame_content = &animation_state.frames[frame_index];
                        let chafa_text: ratatui::text::Text = current_frame_content
                            .clone()
                            .as_str()
                            .into_text()
                            .expect("Failed to convert ANSI to Text");
                        content_lines.extend(chafa_text.lines);
                    } else {
                        content_lines.push(Line::from(Span::styled(
                            "[Error: GIF frame unavailable]",
                            Style::default()
                                .fg(rgb_to_color(&theme.colors.error))
                                .add_modifier(Modifier::ITALIC),
                        )));
                    }
                }
            }
        } else if let Some(image_preview) = &msg.image_preview {
            let chafa_text: ratatui::text::Text = image_preview
                .clone()
                .into_text()
                .expect("Failed to convert ANSI to Text");
            content_lines.extend(chafa_text.lines);
        }
    } else if msg.file_id.is_some() {
        content_lines.push(Line::from(vec![Span::styled(
            format!(
                "{} {}.{} ({} MB)",
                msg.file_icon.as_deref().unwrap_or("📁"),
                msg.file_name.as_deref().unwrap_or("Unknown"),
                msg.file_extension.as_deref().unwrap_or(""),
                msg.file_size_mb.unwrap_or(0.0)
            ),
            Style::default()
                .fg(rgb_to_color(&theme.colors.accent))
                .add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::from(vec![Span::styled(
            format!(
                "Download with: /download {}",
                msg.file_id.as_deref().unwrap_or("")
            ),
            Style::default()
                .fg(rgb_to_color(&theme.colors.dim))
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    let mut message_content_spans: Vec<Span> = Vec::new();
    let mut current_text_slice = msg.content.as_str();
    while !current_text_slice.is_empty() {
        if let Some(mention_match) = mention_regex.find(current_text_slice) {
            let (before_mention, after_mention) =
                current_text_slice.split_at(mention_match.start());
            let mut temp_slice = before_mention;
            while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                message_content_spans.push(
                    Span::raw(temp_slice[..emoji_match.start()].to_string())
                        .fg(rgb_to_color(&theme.colors.text)),
                );
                let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                    message_content_spans
                        .push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.colors.text)));
                } else {
                    message_content_spans.push(
                        Span::raw(emoji_match.as_str().to_string())
                            .fg(rgb_to_color(&theme.colors.text)),
                    );
                }
                temp_slice = &temp_slice[emoji_match.end()..];
            }
            message_content_spans
                .push(Span::raw(temp_slice.to_string()).fg(rgb_to_color(&theme.colors.text)));
            message_content_spans.push(Span::styled(
                "",
                Style::default().fg(rgb_to_color(&theme.colors.mention_bg)),
            ));
            message_content_spans.push(Span::styled(
                mention_match.as_str().to_string(),
                Style::default()
                    .fg(get_contrasting_text_color(&theme.colors.mention_bg))
                    .bg(rgb_to_color(&theme.colors.mention_bg)),
            ));
            message_content_spans.push(Span::styled(
                "",
                Style::default().fg(rgb_to_color(&theme.colors.mention_bg)),
            ));
            current_text_slice = &after_mention[mention_match.len()..];
        } else {
            let mut temp_slice = current_text_slice;
            while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                message_content_spans.push(
                    Span::raw(temp_slice[..emoji_match.start()].to_string())
                        .fg(rgb_to_color(&theme.colors.text)),
                );
                let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                    message_content_spans
                        .push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.colors.text)));
                } else {
                    message_content_spans.push(
                        Span::raw(emoji_match.as_str().to_string())
                            .fg(rgb_to_color(&theme.colors.text)),
                    );
                }
                temp_slice = &temp_slice[emoji_match.end()..];
            }
            message_content_spans
                .push(Span::raw(temp_slice.to_string()).fg(rgb_to_color(&theme.colors.text)));
            current_text_slice = "";
        }
    }

    if !is_special_message {
        let available_text_width = width.saturating_sub(4);
        content_lines.extend(wrap_spans(message_content_spans, available_text_width));
    }

    let mut new_lines = Vec::new();
    let available_width = width as usize;

    if is_first_in_group {
        let user_info_str = format!("{} {}", msg.icon, msg.user.as_str());
        let user_info_width = user_info_str.width();
        let user_box_width = user_info_width + 4;

        let top_border = format!("╭{}╮", "─".repeat(user_box_width - 2));
        new_lines.push(Line::from(Span::styled(
            top_border,
            Style::default().fg(user_color),
        )));

        let mut user_line_spans = vec![
            Span::styled("│ ", Style::default().fg(user_color)),
            Span::styled(
                user_info_str,
                Style::default().fg(user_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │", Style::default().fg(user_color)),
        ];

        let user_line_width = user_line_spans.iter().map(|s| s.width()).sum::<usize>();
        let timestamp_width = timestamp_str.len();

        if available_width > user_line_width + timestamp_width {
            let padding = available_width - user_line_width - timestamp_width;
            user_line_spans.push(Span::raw(" ".repeat(padding)));
        }
        user_line_spans.push(Span::styled(
            timestamp_str.clone(),
            Style::default().fg(rgb_to_color(&theme.colors.dim)),
        ));
        new_lines.push(Line::from(user_line_spans));

        let separator_left = format!("├{}┴", "─".repeat(user_box_width - 2));
        let separator_left_width = separator_left.width();
        let separator_right_width = available_width.saturating_sub(separator_left_width + 1);
        let separator_right = "─".repeat(separator_right_width);

        new_lines.push(Line::from(vec![
            Span::styled(
                separator_left,
                Style::default().fg(rgb_to_color(&theme.colors.dim)),
            ),
            Span::styled(
                separator_right,
                Style::default().fg(rgb_to_color(&theme.colors.dim)),
            ),
            Span::styled("╮", Style::default().fg(rgb_to_color(&theme.colors.dim))),
        ]));
    } else if !is_first_in_group {
        let separator_left = format!("├{}─", "─".repeat(2)); // Small separator for continuation
        let separator_left_width = separator_left.width();
        let separator_right_width = available_width.saturating_sub(separator_left_width + 1);
        let separator_right = "─".repeat(separator_right_width);

        new_lines.push(Line::from(vec![
            Span::styled(
                separator_left,
                Style::default().fg(rgb_to_color(&theme.colors.dim)),
            ),
            Span::styled(
                separator_right,
                Style::default().fg(rgb_to_color(&theme.colors.dim)),
            ),
            Span::styled("┤", Style::default().fg(rgb_to_color(&theme.colors.dim))),
        ]));
    }

    for line in content_lines.iter_mut() {
        let prefix_span = Span::styled("│ ", Style::default().fg(rgb_to_color(&theme.colors.dim)));
        line.spans.insert(0, prefix_span);
        let line_width = line.width();
        if available_width > line_width {
            let padding = available_width - line_width - 1;
            line.spans.push(Span::raw(" ".repeat(padding)));
        }
        line.spans.push(Span::styled(
            "│",
            Style::default().fg(rgb_to_color(&theme.colors.dim)),
        ));
    }
    new_lines.extend(content_lines);

    if is_last_in_group {
        let footer = format!("╰{}╯", "─".repeat(available_width.saturating_sub(2)));
        new_lines.push(Line::from(Span::styled(
            footer,
            Style::default().fg(rgb_to_color(&theme.colors.dim)),
        )));
    }

    new_lines
}