use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::tui::animation::Animation;

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NotificationType {
    Success,
    Warning,
    Error,
    Info,
    Loading,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: usize,
    pub title: String,
    pub content: String,
    pub notification_type: NotificationType,
    pub timeout: Option<Duration>,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    pub current_animation_frame_index: Option<usize>,
    #[serde(skip)]
    pub animation: Option<Animation>,
    #[serde(skip)]
    pub animated_once: bool,
}

impl Notification {
    pub fn new(
        id: usize,
        title: String,
        content: String,
        notification_type: NotificationType,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            id,
            title,
            content,
            notification_type,
            timeout,
            created_at: Instant::now(),
            current_animation_frame_index: None,
            animation: None, // Initialize animation to None
            animated_once: false,
        }
    }

    pub fn icon(&self) -> &str {
        match self.notification_type {
            NotificationType::Success => "",
            NotificationType::Warning => "",
            NotificationType::Error => "",
            NotificationType::Info => "",
            NotificationType::Loading => {
                SPINNER_FRAMES[self.current_animation_frame_index.unwrap_or(0)]
            }
        }
    }

    pub fn color(&self, theme: &crate::themes::Theme) -> Color {
        match self.notification_type {
            NotificationType::Success => crate::themes::rgb_to_color(&theme.colors.success_color),
            NotificationType::Warning => crate::themes::rgb_to_color(&theme.colors.warning_color),
            NotificationType::Error => crate::themes::rgb_to_color(&theme.colors.error),
            NotificationType::Info => crate::themes::rgb_to_color(&theme.colors.info_color),
            NotificationType::Loading => crate::themes::rgb_to_color(&theme.colors.loading_color),
        }
    }

    pub fn height(&self, max_width: u16) -> u16 {
        let content_height = (self.content.len() as u16 / (max_width.saturating_sub(2))).max(1);
        2 + content_height // 2 for borders/title, plus content lines
    }
}

pub struct LoadingNotification {
    id: usize,
    app_state: Arc<tokio::sync::Mutex<crate::app::app_state::AppState>>,
}

impl LoadingNotification {
    pub fn new(
        id: usize,
        app_state: Arc<tokio::sync::Mutex<crate::app::app_state::AppState>>,
    ) -> Self {
        Self { id, app_state }
    }

    pub async fn remove(self) {
        let mut app_state_guard = self.app_state.lock().await;
        app_state_guard.notification_manager.remove(self.id);
    }

    pub async fn replace(self, notification: Notification) {
        let mut app_state_guard = self.app_state.lock().await;
        app_state_guard
            .notification_manager
            .replace(self.id, notification);
    }

    pub async fn update_content(&mut self, content: String) {
        let mut app_state_guard = self.app_state.lock().await;
        app_state_guard
            .notification_manager
            .update_content(self.id, content);
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
    next_id: usize,
}

impl NotificationManager {
    pub async fn add(
        &mut self,
        title: String,
        content: String,
        notification_type: NotificationType,
        timeout: Option<Duration>,
        app_state: Arc<tokio::sync::Mutex<crate::app::app_state::AppState>>,
    ) -> Option<LoadingNotification> {
        let id = self.next_id;
        self.next_id += 1;
        let mut notification = Notification::new(id, title, content, notification_type, timeout);

        if notification.notification_type == NotificationType::Loading {
            notification.current_animation_frame_index = Some(0);
        }

        self.notifications.push(notification.clone());

        if notification.notification_type == NotificationType::Loading {
            Some(LoadingNotification::new(id, app_state))
        } else {
            None
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let mut i = 0;
        while i < self.notifications.len() {
            let n = &mut self.notifications[i];
            if n.notification_type == NotificationType::Loading {
                n.current_animation_frame_index =
                    Some((n.current_animation_frame_index.unwrap_or(0) + 1) % SPINNER_FRAMES.len());
                i += 1;
                continue;
            }
            if n.timeout
                .map_or(true, |timeout| now.duration_since(n.created_at) < timeout)
            {
                i += 1;
            } else {
                self.notifications.remove(i);
            }
        }
    }

    pub fn remove(&mut self, id: usize) {
        self.notifications.retain(|n| n.id != id);
    }

    pub fn replace(&mut self, id: usize, mut new_notification: Notification) {
        if let Some(index) = self.notifications.iter().position(|n| n.id == id) {
            new_notification.id = id;
            self.notifications[index] = new_notification;
        }
    }

    pub fn update_content(&mut self, id: usize, content: String) {
        if let Some(notification) = self.notifications.iter_mut().find(|n| n.id == id) {
            notification.content = content;
        }
    }

    pub fn notifications(&self) -> &Vec<Notification> {
        &self.notifications
    }

    pub fn notifications_mut(&mut self) -> &mut Vec<Notification> {
        &mut self.notifications
    }
}