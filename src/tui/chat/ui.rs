use crate::api::models::BroadcastMessage;
use crate::app::{AppState, PopupType};
use crate::tui::chat::create_channel_form::{CreateChannelForm, ICONS};
use crate::tui::chat::popups::create_channel::{draw_create_channel_popup, get_create_channel_popup_height};
use crate::tui::chat::popups::deconnection::{draw_deconnection_popup, get_deconnection_popup_height};
use crate::tui::chat::popups::emojis::{draw_emojis_popup, get_emojis_popup_height};
use crate::tui::chat::popups::help::{draw_help_popup, get_help_popup_height};
use crate::tui::chat::popups::mentions::{draw_mentions_popup, get_mentions_popup_height};
use crate::tui::chat::popups::quit::{draw_quit_popup, get_quit_popup_height};
use crate::tui::chat::popups::set_theme::{draw_set_theme_popup, get_set_theme_popup_height};
use crate::tui::chat::popups::settings::{draw_settings_popup, get_settings_popup_height};
use crate::tui::chat::theme_settings_form::ThemeSettingsForm;
use crate::tui::chat::utils::{centered_rect, get_color_for_user};
use crate::tui::themes::{get_contrasting_text_color, get_theme, rgb_to_color, Theme};
use chrono::{TimeZone, Utc};
use ratatui::prelude::Stylize;

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use regex::Regex;
use std::collections::VecDeque;
use unicode_segmentation::UnicodeSegmentation;



pub fn draw_chat_ui<B: Backend>(
    f: &mut Frame,
    state: &mut AppState,
    input_text: &str,
    channel_list_state: &mut ListState,
    create_channel_form: &mut CreateChannelForm,
    theme_settings_form: &mut ThemeSettingsForm,
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
            ListItem::new(format!("{} {}", channel.icon, channel.name)).style(style)
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
        .highlight_symbol(">> ");
    f.render_stateful_widget(channels_list, chunks[0], channel_list_state);
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
                rendered_lines[start_index..end_index].to_vec()
            } else {
                rendered_lines.clone().into()
            }
        };
        let messages_paragraph =
            Paragraph::new(messages_to_render).wrap(ratatui::widgets::Wrap { trim: false });
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
    let input_paragraph = Paragraph::new(Text::from(input_text.to_string()))
        .block(input_block)
        .style(Style::default().fg(rgb_to_color(&current_theme.input_text_active)));
    f.render_widget(input_paragraph, input_area);
    if !state.popup_state.show || state.popup_state.popup_type == PopupType::Emojis {
        let cursor_x_offset =
            UnicodeSegmentation::graphemes(&input_text[..state.cursor_position], true).count()
                as u16;
        let input_cursor_x = chat_chunks[1].x + 1 + cursor_x_offset;
        let input_cursor_y = chat_chunks[1].y + 1;
        f.set_cursor_position((input_cursor_x, input_cursor_y));
    }
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
        };
        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(popup_title)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.popup_border))
                    .bg(rgb_to_color(&current_theme.background)),
            );
        let area = match state.popup_state.popup_type {
            PopupType::Quit => {
                let height = get_quit_popup_height();
                let width = 40; // A reasonable default width for quit popup
                centered_rect(width, height, size)
            }
            PopupType::Deconnection => {
                let height = get_deconnection_popup_height();
                let width = 50; // A reasonable default width for deconnection popup
                centered_rect(width, height, size)
            }
            PopupType::Settings => {
                let height = get_settings_popup_height();
                let width = 40; // A reasonable default width for settings popup
                centered_rect(width, height, size)
            }
            PopupType::CreateChannel => {
                let height = get_create_channel_popup_height();
                let width = (ICONS.len() * 3) as u16 + 2 + 2; // Width based on icons
                centered_rect(width, height, size)
            }
            PopupType::SetTheme => {
                let height = get_set_theme_popup_height(theme_settings_form);
                let width = (ICONS.len() * 3) as u16 + 2 + 2; // Width based on icons
                centered_rect(width, height, size)
            }
            PopupType::Help => {
                let height = get_help_popup_height();
                let width = 80; // A reasonable default width for help popup
                centered_rect(width, height, size)
            }
            PopupType::Mentions => {
                let height = get_mentions_popup_height(state);
                let max_width = filtered_users
                    .iter()
                    .map(|user| user.len() as u16 + 4)
                    .max()
                    .unwrap_or(0)
                    .min(size.width - 4);
                let width = max_width.max(20);
                let input_area_y = chat_chunks[1].y + chat_chunks[1].height - input_height;
                Rect::new(
                    chat_chunks[1].x,
                    input_area_y.saturating_sub(height + 1),
                    width,
                    height,
                )
            }
            PopupType::Emojis => {
                let height = get_emojis_popup_height(state);
                let max_width = filtered_emojis
                    .iter()
                    .map(|emoji_str| (emoji_str.len() + 3) as u16)
                    .max()
                    .unwrap_or(0)
                    .min(size.width - 4);
                let width = max_width.max(20);
                let input_area_y = chat_chunks[1].y + chat_chunks[1].height - input_height;
                Rect::new(
                    chat_chunks[1].x,
                    input_area_y.saturating_sub(height + 1),
                    width,
                    height,
                )
            }
            _ => Rect::default(),
        };
        f.render_widget(Clear, area);
        f.render_widget(&popup_block, area);
        match state.popup_state.popup_type {
            PopupType::Quit => {
                draw_quit_popup(f, state, area, &popup_block);
            }
            PopupType::Settings => {
                draw_settings_popup(f, state, area, &popup_block);
            }
            PopupType::CreateChannel => {
                draw_create_channel_popup(f, state, area, create_channel_form, &popup_block);
            }
            PopupType::SetTheme => {
                draw_set_theme_popup(f, state, area, theme_settings_form, &popup_block);
            }
            PopupType::Deconnection => {
                draw_deconnection_popup(f, state, area, &popup_block);
            }
            PopupType::Help => {
                draw_help_popup(f, state, area, &popup_block);
            }
            PopupType::Mentions => {
                draw_mentions_popup(f, state, area, &popup_block);
            }
            PopupType::Emojis => {
                draw_emojis_popup(f, state, area, &popup_block);
            }
            _ => { /* No specific rendering for other popup types yet */ }
        }
    }
    if let Some(error_msg) = &state.error_message {
        let error_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(rgb_to_color(&current_theme.error))
                    .bg(rgb_to_color(&current_theme.background)),
            );
        let error_paragraph = Paragraph::new(Line::from(error_msg.clone()))
            .style(Style::default().fg(rgb_to_color(&current_theme.text)))
            .alignment(Alignment::Center)
            .block(error_block);
        let error_width = (error_msg.len() + 4) as u16;
        let error_height = 3;
        let error_area = Rect::new(
            size.width.saturating_sub(error_width).saturating_sub(1),
            1,
            error_width,
            error_height,
        );
        f.render_widget(Clear, error_area);
        f.render_widget(error_paragraph, error_area);
    }
    if !state.popup_state.show {
        let cursor_x_offset =
            UnicodeSegmentation::graphemes(&input_text[..state.cursor_position], true).count()
                as u16;
        let input_cursor_x = chat_chunks[1].x + 1 + cursor_x_offset;
        let input_cursor_y = chat_chunks[1].y + 1;
        f.set_cursor_position((input_cursor_x, input_cursor_y));
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
    if last_user.as_ref() == Some(&msg.user) {
        let mut current_spans = Vec::new();
        let mut current_text_slice = msg.content.as_str();
        while !current_text_slice.is_empty() {
            if let Some(mention_match) = mention_regex.find(current_text_slice) {
                let (before_mention, after_mention) =
                    current_text_slice.split_at(mention_match.start());
                let mut temp_slice = before_mention;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    current_spans.push(
                        Span::raw(&temp_slice[..emoji_match.start()]).fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        current_spans.push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        current_spans
                            .push(Span::raw(emoji_match.as_str()).fg(rgb_to_color(&theme.text)));
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                current_spans.push(Span::raw(temp_slice).fg(rgb_to_color(&theme.text)));
                current_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                current_spans.push(Span::styled(
                    mention_match.as_str(),
                    Style::default()
                        .fg(get_contrasting_text_color(&theme.mention_bg))
                        .bg(rgb_to_color(&theme.mention_bg)),
                ));
                current_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                current_text_slice = &after_mention[mention_match.len()..];
            } else {
                let mut temp_slice = current_text_slice;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    current_spans.push(
                        Span::raw(&temp_slice[..emoji_match.start()]).fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        current_spans.push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        current_spans
                            .push(Span::raw(emoji_match.as_str()).fg(rgb_to_color(&theme.text)));
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                current_spans.push(Span::raw(temp_slice).fg(rgb_to_color(&theme.text)));
                current_text_slice = "";
            }
        }
        lines.push_back(Line::from(vec![
            Span::styled(
                "│ ".to_string(),
                Style::default().fg(rgb_to_color(&theme.dim)),
            ),
            Span::raw(
                current_spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>(),
            ),
        ]));
    } else {
        let header_spans = vec![
            Span::styled("╭ ".to_string(), Style::default().fg(user_color)),
            Span::styled(
                format!("{} ", msg.icon),
                Style::default().fg(rgb_to_color(&theme.text)),
            ),
            Span::styled(
                msg.user.clone(),
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
        let mut current_spans = Vec::new();
        let mut current_text_slice = msg.content.as_str();
        while !current_text_slice.is_empty() {
            if let Some(mention_match) = mention_regex.find(current_text_slice) {
                let (before_mention, after_mention) =
                    current_text_slice.split_at(mention_match.start());
                let mut temp_slice = before_mention;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    current_spans.push(
                        Span::raw(&temp_slice[..emoji_match.start()]).fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        current_spans.push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        current_spans
                            .push(Span::raw(emoji_match.as_str()).fg(rgb_to_color(&theme.text)));
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                current_spans.push(Span::raw(temp_slice).fg(rgb_to_color(&theme.text)));
                current_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                current_spans.push(Span::styled(
                    mention_match.as_str(),
                    Style::default()
                        .fg(get_contrasting_text_color(&theme.mention_bg))
                        .bg(rgb_to_color(&theme.mention_bg)),
                ));
                current_spans.push(Span::styled(
                    "",
                    Style::default().fg(rgb_to_color(&theme.mention_bg)),
                ));
                current_text_slice = &after_mention[mention_match.len()..];
            } else {
                let mut temp_slice = current_text_slice;
                while let Some(emoji_match) = emoji_regex.find(temp_slice) {
                    current_spans.push(
                        Span::raw(&temp_slice[..emoji_match.start()]).fg(rgb_to_color(&theme.text)),
                    );
                    let shortcode = &emoji_match.as_str()[1..emoji_match.as_str().len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(shortcode) {
                        current_spans.push(Span::raw(emoji.as_str()).fg(rgb_to_color(&theme.text)));
                    } else {
                        current_spans
                            .push(Span::raw(emoji_match.as_str()).fg(rgb_to_color(&theme.text)));
                    }
                    temp_slice = &temp_slice[emoji_match.end()..];
                }
                current_spans.push(Span::raw(temp_slice).fg(rgb_to_color(&theme.text)));
                current_text_slice = "";
            }
        }
        lines.push_back(Line::from(vec![
            Span::styled(
                "│ ".to_string(),
                Style::default().fg(rgb_to_color(&theme.dim)),
            ),
            Span::raw(
                current_spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>(),
            ),
        ]));
    }
}
