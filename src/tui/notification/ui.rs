use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{app::app_state::AppState, themes::Theme};
use crate::tui::animation::{Animation, AnimationType};
use crate::tui::notification::notification::Notification;
use std::time::Duration;

const NOTIFICATION_LIMIT: usize = 5;

pub fn draw_notifications(f: &mut Frame, app_state: &mut AppState) {
    let notifications = app_state.notification_manager.notifications_mut();
    if notifications.is_empty() {
        return;
    }

    let theme = &app_state.current_theme;
    let area = f.area();

    let max_width = 40;
    let notification_width = notifications
        .iter()
        .map(|n| n.title.len() + n.content.len() + 5)
        .max()
        .unwrap_or(0)
        .clamp(20, max_width) as u16;

    let _num_notifications = notifications.len();
    let mut notifications_to_display: Vec<&mut Notification> = notifications
        .iter_mut()
        .filter(|n| n.animation.is_some() || n.timeout.map_or(true, |t| n.created_at.elapsed() < t))
        .collect();

    notifications_to_display.sort_by_key(|n| n.created_at);

    // Apply NOTIFICATION_LIMIT after filtering and sorting
    let num_active_notifications = notifications_to_display.len();
    let start_index = if num_active_notifications > NOTIFICATION_LIMIT {
        num_active_notifications - NOTIFICATION_LIMIT
    } else {
        0
    };
    let mut notifications_to_display_limited = notifications_to_display.drain(start_index..).collect::<Vec<_>>();

    let mut total_height = 0;
    let mut heights = Vec::new();
    for n in &notifications_to_display_limited {
        let height = n.height(notification_width);
        heights.push(height);
        total_height += height;
    }

    if num_active_notifications > NOTIFICATION_LIMIT {
        total_height += 3;
    }

    let popup_area = Rect::new(
        area.width.saturating_sub(notification_width + 1),
        1,
        notification_width,
        total_height,
    );

    f.render_widget(Clear, popup_area);

    let mut y_offset = popup_area.y;

    for (i, notification) in notifications_to_display_limited.iter_mut().enumerate() {
        let height = heights[i];
        let final_y = y_offset;

        if notification.animation.is_none() && !notification.animated_once {
            // Initialize animation for new notifications
            let start_y = area.height as i32; // Start from bottom of the screen
            let end_y = final_y as i32; // End at its final position
            let animation_duration = Duration::from_millis(500);
            let end_color_rgb = theme.colors.background;
            let start_color = [0, 0, 0]; // Start from black
            let end_color = [end_color_rgb.0, end_color_rgb.1, end_color_rgb.2];
            let start_x = -(notification_width as i32); // Start from left of the screen
            let end_x = popup_area.x as i32; // End at its final position
            notification.animation = Some(Animation::new(
                AnimationType::SlideIn {
                    start_y,
                    end_y,
                    start_x,
                    end_x,
                    start_color,
                    end_color,
                },
                animation_duration,
            ));
        }

        let mut current_y = final_y;
        let mut current_x = popup_area.x;
        let mut current_bg_color = crate::themes::rgb_to_color(&theme.colors.background);
        if let Some(animation) = &mut notification.animation {
            let progress = animation.progress();
            if let AnimationType::SlideIn { start_y, end_y, start_x, end_x, .. } = animation.animation_type {
                let animated_y_f32 = start_y as f32 + ((end_y as f32 - start_y as f32) * progress);
                current_y = animated_y_f32.max(0.0).round() as u16;
                // Clamp current_y to prevent it from going out of bounds
                current_y = current_y.min(popup_area.y + popup_area.height - 1);

                let animated_x_f32 = start_x as f32 + ((end_x as f32 - start_x as f32) * progress);
                current_x = animated_x_f32.max(0.0).round() as u16;
                // Clamp current_x to prevent it from going out of bounds
                current_x = current_x.min(popup_area.x + popup_area.width - 1);
            }
            if let Some(animated_rgb_array) = animation.get_current_color() {
                let animated_rgb = crate::themes::Rgb(animated_rgb_array[0], animated_rgb_array[1], animated_rgb_array[2]);
                current_bg_color = crate::themes::rgb_to_color(&animated_rgb);
            }
            if animation.is_finished() {
                notification.animation = None;
                notification.animated_once = true;
            }
        }

        let render_area = Rect::new(current_x, current_y, popup_area.width, height);
        draw_notification(f, notification, theme, render_area, current_bg_color);
        y_offset += height;
    }

    // Remove notifications that have finished animating and timed out
    app_state.notification_manager.notifications_mut().retain(|n| {
        n.animation.is_some() || n.timeout.map_or(true, |t| n.created_at.elapsed() < t)
    });
}

fn draw_notification(
    f: &mut Frame,
    notification: &crate::tui::notification::notification::Notification,
    theme: &Theme,
    area: Rect,
    bg_color: Color,
) {
    let block = Block::default()
        .title(format!(
            "{} {}",
            notification.icon(),
            notification.title.as_str()
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(
            Style::default()
                .fg(notification.color(theme))
                .bg(bg_color),
        )
        .border_style(Style::default().bg(bg_color));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let content = Paragraph::new(notification.content.as_str())
        .wrap(ratatui::widgets::Wrap { trim: true })
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.text))
                .bg(bg_color),
        );

    f.render_widget(content, inner_area);
}