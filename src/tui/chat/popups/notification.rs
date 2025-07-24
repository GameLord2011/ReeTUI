use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, NotificationType},
    tui::themes::{get_theme, rgb_to_color},
};

pub fn draw_notification_popup(f: &mut Frame, app_state: &AppState) {
    if let Some(notification) = &app_state.notification {
        let theme = get_theme(app_state.current_theme);
        let popup_area = {
            let parent_area = f.area();
            let popup_width = 40;
            let popup_height = 5;
            Rect::new(
                parent_area.width.saturating_sub(popup_width + 1),
                1,
                popup_width,
                popup_height,
            )
        };

        f.render_widget(Clear, popup_area);

        let (border_color, _) = match notification.notification_type {
            NotificationType::Info => (theme.accent, theme.accent),
            NotificationType::Warning => (theme.button, theme.button),
            NotificationType::Error => (theme.error, theme.error),
            NotificationType::Success => (theme.border_focus, theme.border_focus),
        };

        let popup_block = Block::default()
            .title(format!(" {} ", notification.title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&border_color)));

        let text = Text::from(vec![
            Line::from(Span::styled(
                &notification.message,
                Style::default().fg(rgb_to_color(&theme.text)),
            )),
        ]);

        let paragraph = Paragraph::new(text)
            .block(popup_block)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }
}
