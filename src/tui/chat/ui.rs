use crate::api::models::BroadcastMessage;
use crate::app::{AppState, PopupType};
use crate::themes::{
    color_to_rgb,
    get_contrasting_text_color,
    interpolate_rgb,
    rgb_to_color,
    Theme,
};
use crate::tui::chat::create_channel_form::CreateChannelForm;
use crate::tui::chat::gif_renderer::GifAnimationState;
use crate::tui::chat::popups::create_channel::{
    draw_create_channel_popup,
    get_create_channel_popup_size,
};

use crate::tui::chat::popups::deconnection::{
    draw_deconnection_popup,
    get_deconnection_popup_size,
};
use crate::tui::chat::popups::download_progress::{
    draw_download_progress_popup,
    get_download_progress_popup_size,
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
    widgets::{Block, BorderType, Borders, Clear, ListState, Paragraph},
};
use regex::Regex;

#[derive(Debug)]
pub struct RenderedMessage {
    pub id: String,
    pub lines: Vec<Line<'static>>,
    pub is_first_in_group: bool,
    pub is_last_in_group: bool,
}

#[allow(unused_assignments)]
fn wrap_spans<'a>(spans: Vec<Span<'a>>, max_width: u16) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut current_line_width = 0;

    for span in spans {
        let mut remaining_content = span.content.as_ref();
        let original_style = span.style;

        while !remaining_content.is_empty() {
            if let Some(newline_pos) = remaining_content.find('\n') {
                let (segment, rest) = remaining_content.split_at(newline_pos);
                if !segment.is_empty() {
                    current_line_spans.push(Span::styled(segment.to_string(), original_style));
                    current_line_width += segment.width() as u16;
                }
                lines.push(Line::from(
                    current_line_spans.drain(..).collect::<Vec<Span<'a>>>(),
                ));
                current_line_width = 0;
                remaining_content = &rest[1..]; // Skip the newline
            } else {
                // No newline in remaining_content
                let segment_width = remaining_content.width() as u16;
                if current_line_width + segment_width > max_width {
                    // Need to break the segment
                    let mut break_point = 0;
                    for (idx, c) in remaining_content.char_indices() {
                        let char_width = c.width().unwrap_or(0) as u16;
                        if current_line_width + char_width > max_width {
                            break;
                        }
                        current_line_width += char_width;
                        break_point = idx + c.len_utf8();
                    }

                    if break_point > 0 {
                        let (segment, rest) = remaining_content.split_at(break_point);
                        current_line_spans.push(Span::styled(segment.to_string(), original_style));
                        lines.push(Line::from(
                            current_line_spans.drain(..).collect::<Vec<Span<'a>>>(),
                        ));
                        current_line_width = 0;
                        remaining_content = rest;
                    } else {
                        // Single character wider than max_width or no progress
                        current_line_spans
                            .push(Span::styled(remaining_content.to_string(), original_style));
                        lines.push(Line::from(
                            current_line_spans.drain(..).collect::<Vec<Span<'a>>>(),
                        ));
                        current_line_width = 0;
                        remaining_content = "";
                    }
                } else {
                    // Segment fits entirely
                    current_line_spans
                        .push(Span::styled(remaining_content.to_string(), original_style));
                    current_line_width += segment_width;
                    remaining_content = "";
                }
            }
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
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
    if state
        .last_rendered_theme
        .map_or(true, |name| name != state.current_theme.name)
    {
        state.rendered_messages.clear();
        state.last_rendered_theme = Some(state.current_theme.name);
    }
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
                .fg(
                    if state.chat_focused_pane
                        == crate::app::app_state::ChatFocusedPane::ChannelList
                    {
                        rgb_to_color(&current_theme.colors.accent)
                    } else {
                        rgb_to_color(&current_theme.colors.border)
                    },
                )
                .bg(rgb_to_color(&current_theme.colors.background)),
        );
    f.render_widget(channels_block.clone(), left_chunks[0]); // Render the outer block

    let inner_channels_area = channels_block.inner(left_chunks[0]);
    let item_height = 3; // User requested 3 lines of height per channel button

    let _num_channels = state.channels.len();
    let visible_items_count = (inner_channels_area.height / item_height) as usize;

    // Calculate scroll offset for the channel list
    let channel_scroll_offset = channel_list_state.selected().unwrap_or(0);
    let start_channel_index = if channel_scroll_offset >= visible_items_count {
        channel_scroll_offset - visible_items_count + 1
    } else {
        0
    };

    for (i, channel) in state
        .channels
        .iter()
        .enumerate()
        .skip(start_channel_index)
        .take(visible_items_count)
    {
        let is_selected = channel_list_state.selected().map_or(false, |s| s == i);

        let item_rect = Rect::new(
            inner_channels_area.x,
            inner_channels_area.y + (i - start_channel_index) as u16 * item_height,
            inner_channels_area.width,
            item_height,
        );

        let border_color = if is_selected {
            rgb_to_color(&current_theme.colors.accent)
        } else {
            rgb_to_color(&current_theme.colors.dim)
        };
        let border_style = Style::default().fg(border_color);

        let icon_inner_width = channel.icon.width() as u16 + 2;
        let name_inner_width = inner_channels_area
            .width
            .saturating_sub(icon_inner_width + 3);

        let top_border = Line::from(vec![
            Span::styled("╭", border_style),
            Span::styled("─".repeat(icon_inner_width as usize), border_style),
            Span::styled("┬", border_style),
            Span::styled("─".repeat(name_inner_width as usize), border_style),
            Span::styled("╮", border_style),
        ]);

        let channel_name = channel.name.clone();
        let max_name_width = if name_inner_width > 1 {
            name_inner_width - 1
        } else {
            0
        } as usize;

        let mut truncated_name = String::new();
        let mut current_width = 0;
        for c in channel_name.chars() {
            let char_width = c.width().unwrap_or(1);
            if current_width + char_width > max_name_width {
                break;
            }
            truncated_name.push(c);
            current_width += char_width;
        }

        let padding_width = max_name_width.saturating_sub(truncated_name.width());
        let padded_name = format!(" {}{}", truncated_name, " ".repeat(padding_width));

        let text_style = if is_selected {
            Style::default().fg(rgb_to_color(&current_theme.colors.accent))
        } else {
            Style::default().fg(rgb_to_color(&current_theme.colors.text))
        };

        let middle_line = Line::from(vec![
            Span::styled("│", border_style),
            Span::styled(format!(" {} ", channel.icon), text_style),
            Span::styled("│", border_style),
            Span::styled(padded_name, text_style),
            Span::styled("│", border_style),
        ]);

        let bottom_border = Line::from(vec![
            Span::styled("╰", border_style),
            Span::styled("─".repeat(icon_inner_width as usize), border_style),
            Span::styled("┴", border_style),
            Span::styled("─".repeat(name_inner_width as usize), border_style),
            Span::styled("╯", border_style),
        ]);

        let text = Text::from(vec![top_border, middle_line, bottom_border]);
        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, item_rect);
    }

    let user_info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("User Info")
        .style(
            Style::default()
                .fg(rgb_to_color(&current_theme.colors.border))
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
            "{}  Messages",
            state
                .current_channel
                .as_ref()
                .map_or("XXXXXX".to_string(), |c| c.name.clone())
        ))
        .style(
            Style::default()
                .fg(
                    if state.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Messages {
                        rgb_to_color(&current_theme.colors.accent)
                    } else {
                        rgb_to_color(&current_theme.colors.text)
                    },
                )
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
                let message_id = msg.client_id.clone().unwrap();
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

                let rendered_message_entry = state
                    .rendered_messages
                    .entry(channel_id.clone())
                    .or_default()
                    .get(&message_id);

                if needs_re_render_this_message || rendered_message_entry.is_none() {
                    let rendered_message = format_message_lines(
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
                        .insert(message_id.clone(), rendered_message);
                    state
                        .needs_re_render
                        .entry(channel_id.clone())
                        .or_default()
                        .insert(message_id.clone(), false);
                } else {
                }

                if let Some(rendered_message) = state
                    .rendered_messages
                    .get(channel_id)
                    .and_then(|channel_map| channel_map.get(&message_id))
                {
                    all_rendered_lines.extend(rendered_message.lines.clone());
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
                .fg(
                    if state.chat_focused_pane == crate::app::app_state::ChatFocusedPane::Input {
                        rgb_to_color(&current_theme.colors.accent)
                    } else {
                        rgb_to_color(&current_theme.colors.input_border_inactive)
                    },
                )
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
        let popup_title = match state.popup_state.popup_type {
            PopupType::CreateChannel => "Create Channel",
            PopupType::Deconnection => "Deconnection",
            PopupType::None => "",
            PopupType::Mentions => "",
            PopupType::Emojis => "",
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
        settings::render_settings_popup::<B>(f, state, settings_state, f.area()).unwrap();
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
) -> RenderedMessage {
    let message_id = msg
        .file_id
        .clone()
        .unwrap_or_else(|| msg.timestamp.to_string());
    let mut content_lines = Vec::new();
    let timestamp_str = Utc
        .timestamp_opt(msg.timestamp, 0)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%H:%M")
        .to_string();
    let user_color = get_color_for_user(&msg.user, &theme.colors.username_colors);
    let border_rgb = theme.colors.dim;
    let user_rgb = color_to_rgb(user_color).unwrap_or(border_rgb);

    let is_special_message = msg.file_id.is_some() || msg.is_image.unwrap_or(false);

    if msg.is_image.unwrap_or(false) {
        if let Some(_gif_frames) = &msg.gif_frames {
            if let Some(file_id) = &msg.file_id {
                if let Some(animation_state_arc) = active_animations.get(file_id) {
                    if let Ok(animation_state) = animation_state_arc.try_lock() {
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
                        }
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
                "{} {}.{} 󰋊 {} MB",
                msg.file_icon.as_deref().unwrap_or("󱧸"),
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

        let top_border_str = format!("╭{}╮", "─".repeat(user_box_width - 2));
        let mut top_border_spans = Vec::new();
        for (i, c) in top_border_str.chars().enumerate() {
            let fraction = i as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            top_border_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
        }
        new_lines.push(Line::from(top_border_spans));

        let mut user_line_spans = Vec::new();
        let mut current_col = 0;
        for c in "│ ".chars() {
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            user_line_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += c.width().unwrap_or(1);
        }

        for c in user_info_str.chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            user_line_spans.push(Span::styled(
                c.to_string(),
                Style::default()
                    .fg(interpolated_color)
                    .add_modifier(Modifier::BOLD),
            ));
            current_col += char_width;
        }

        current_col = user_box_width - 2;
        for c in " │".chars() {
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            user_line_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += c.width().unwrap_or(1);
        }

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

        let separator_left_str = format!("├{}┴", "─".repeat(user_box_width - 2));
        let separator_left_width = separator_left_str.width();
        let mut separator_spans = Vec::new();
        let mut current_col = 0;
        for c in separator_left_str.chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            separator_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += char_width;
        }

        let separator_right_width = available_width.saturating_sub(separator_left_width + 1);
        let separator_right_str = "─".repeat(separator_right_width);
        for c in separator_right_str.chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            separator_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += char_width;
        }

        let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
        let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
        let interpolated_color = rgb_to_color(&interpolated_rgb);
        separator_spans.push(Span::styled("╮", Style::default().fg(interpolated_color)));
        new_lines.push(Line::from(separator_spans));
    } else if !is_first_in_group {
        let separator_left = format!("├{}─", "─".repeat(2));
        let separator_left_width = separator_left.width();
        let separator_right_width = available_width.saturating_sub(separator_left_width + 1);
        let separator_right = "─".repeat(separator_right_width);
        let separator_str = format!("{}{}", separator_left, separator_right);
        let mut separator_spans = Vec::new();
        let mut current_col = 0;
        for c in separator_str.chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            separator_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += char_width;
        }
        let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
        let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
        let interpolated_color = rgb_to_color(&interpolated_rgb);
        separator_spans.push(Span::styled("┤", Style::default().fg(interpolated_color)));
        new_lines.push(Line::from(separator_spans));
    }

    for line in content_lines.iter_mut() {
        let mut new_line_spans = Vec::new();
        let mut current_col = 0;
        for c in "│ ".chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            new_line_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += char_width;
        }

        new_line_spans.extend(line.spans.clone());

        let line_width = new_line_spans.iter().map(|s| s.width()).sum::<usize>();
        if available_width > line_width {
            let padding = available_width - line_width - 1;
            new_line_spans.push(Span::raw(" ".repeat(padding)));
        }

        let fraction = (available_width - 1) as f32 / (available_width - 1).max(1) as f32;
        let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
        let interpolated_color = rgb_to_color(&interpolated_rgb);
        new_line_spans.push(Span::styled("│", Style::default().fg(interpolated_color)));
        *line = Line::from(new_line_spans);
    }
    new_lines.extend(content_lines);

    if is_last_in_group {
        let footer_str = format!("╰{}╯", "─".repeat(available_width.saturating_sub(2)));
        let mut footer_spans = Vec::new();
        let mut current_col = 0;
        for c in footer_str.chars() {
            let char_width = c.width().unwrap_or(1);
            let fraction = current_col as f32 / (available_width - 1).max(1) as f32;
            let interpolated_rgb = interpolate_rgb(&user_rgb, &border_rgb, fraction);
            let interpolated_color = rgb_to_color(&interpolated_rgb);
            footer_spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(interpolated_color),
            ));
            current_col += char_width;
        }
        new_lines.push(Line::from(footer_spans));
    }

    RenderedMessage {
        id: message_id,
        lines: new_lines
            .into_iter()
            .map(|line| {
                let spans: Vec<Span> = line
                    .spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.into_owned(), span.style))
                    .collect();
                Line::from(spans)
            })
            .collect(),
        is_first_in_group,
        is_last_in_group,
    }
}