use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{app::app_state::AppState, themes::Theme};

const NOTIFICATION_LIMIT: usize = 5;

pub fn draw_notifications(f: &mut Frame, app_state: &AppState) {
    let notifications = app_state.notification_manager.notifications();
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

    let num_notifications = notifications.len();
    let mut notifications_to_display = if num_notifications > NOTIFICATION_LIMIT {
        notifications[num_notifications - NOTIFICATION_LIMIT..].to_vec()
    } else {
        notifications.to_vec()
    };
    notifications_to_display.reverse();

    let mut total_height = 0;
    let mut heights = Vec::new();
    for n in &notifications_to_display {
        let content_height = (n.content.len() as u16 / (notification_width.saturating_sub(2))).max(1);
        let height = 2 + content_height;
        heights.push(height);
        total_height += height;
    }

    if num_notifications > NOTIFICATION_LIMIT {
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

    for (i, notification) in notifications_to_display.iter().enumerate() {
        let height = heights[i];
        let area = Rect::new(popup_area.x, y_offset, popup_area.width, height);
        draw_notification(f, notification, theme, area);
        y_offset += height;
    }

    if num_notifications > NOTIFICATION_LIMIT {
        let remaining = num_notifications - NOTIFICATION_LIMIT;
        let area = Rect::new(popup_area.x, y_offset, popup_area.width, 3);
        let text = format!("+ {} more...", remaining);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(
                Style::default()
                    .fg(crate::themes::rgb_to_color(&theme.colors.text))
                    .bg(crate::themes::rgb_to_color(&theme.colors.background)),
            );

        let p = Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(block);

        f.render_widget(p, area);
    }
}

fn draw_notification(
    f: &mut Frame,
    notification: &crate::tui::notification::notification::Notification,
    theme: &Theme,
    area: Rect,
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
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        );

    let content = Paragraph::new(notification.content.as_str())
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .style(
            Style::default()
                .fg(crate::themes::rgb_to_color(&theme.colors.text))
                .bg(crate::themes::rgb_to_color(&theme.colors.background)),
        );

    f.render_widget(content, area);
}
