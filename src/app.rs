use crate::api::models::{BroadcastMessage, Channel};
use crate::tui::themes::ThemeName;
use ratatui::text::Line;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupType {
    Quit, // Do not remove this variant, it is used in other parts of the code.
    Settings,
    CreateChannel,
    SetTheme,
    Deconnection,
    Help,
    Mentions,
    Emojis,
    FileManager,
    DownloadProgress,
    DebugJson,
    None,
    Notification,
}

#[derive(Debug, Clone, Copy)]
pub struct PopupState {
    pub show: bool,
    pub popup_type: PopupType,
}

impl Default for PopupState {
    fn default() -> Self {
        Self {
            show: false,
            popup_type: PopupType::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub user_icon: Option<String>,
    pub current_channel: Option<Channel>,
    pub channels: Vec<Channel>,
    pub messages: HashMap<String, VecDeque<BroadcastMessage>>,
    pub rendered_messages: HashMap<String, Vec<Line<'static>>>,
    pub channel_history_state: HashMap<String, (usize, bool)>,
    pub animation_frame_index: usize,
    pub last_frame_time: Instant,
    pub popup_state: PopupState,
    pub message_scroll_offset: usize,
    pub current_theme: ThemeName,
    pub selected_setting_index: usize,
    pub active_users: Vec<String>,
    pub selected_mention_index: usize,
    pub selected_emoji_index: usize,
    pub mention_query: String,
    pub emoji_query: String,
    pub cursor_position: usize,
    pub notification: Option<Notification>,
    pub download_progress: u8,
    pub debug_json_content: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth_token: None,
            username: None,
            user_icon: None,
            current_channel: None,
            channels: Vec::new(),
            messages: HashMap::new(),
            rendered_messages: HashMap::new(),
            channel_history_state: HashMap::new(),
            animation_frame_index: 0,
            last_frame_time: Instant::now(),
            popup_state: PopupState::default(),
            message_scroll_offset: 0,
            current_theme: ThemeName::Default,
            selected_setting_index: 0,
            active_users: Vec::new(),
            selected_mention_index: 0,
            selected_emoji_index: 0,
            mention_query: String::new(),
            emoji_query: String::new(),
            cursor_position: 0,
            notification: None,
            download_progress: 0,
            debug_json_content: String::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState::default()
    }

    pub fn set_user_auth(&mut self, token: String, username: String, icon: String) {
        self.auth_token = Some(token);
        self.username = Some(username);
        self.user_icon = Some(icon);
    }

    pub fn set_notification(&mut self, title: String, message: String, notification_type: NotificationType) {
        self.notification = Some(Notification {
            title,
            message,
            notification_type,
            created_at: Instant::now(),
        });
    }

    pub fn clear_notification(&mut self) {
        self.notification = None;
    }

    pub fn clear_user_auth(&mut self) {
        self.auth_token = None;
        self.username = None;
        self.user_icon = None;
        self.current_channel = None;
        self.channels.clear();
        self.messages.clear();
        self.rendered_messages.clear();
        self.channel_history_state.clear();
        self.animation_frame_index = 0;
        self.last_frame_time = Instant::now();
        self.popup_state = PopupState::default();
        self.notification = None; // Clear notification on logout
        self.message_scroll_offset = 0;
        self.current_theme = ThemeName::Default;
    }

    pub fn set_current_channel(&mut self, channel: Channel) {
        let channel_id = channel.id.clone();
        self.current_channel = Some(channel);
        self.messages.entry(channel_id.clone()).or_default();
        self.rendered_messages
            .entry(channel_id.clone())
            .or_default();
        self.channel_history_state
            .entry(channel_id)
            .or_insert((0, true));
        self.message_scroll_offset = 0;
    }

    pub fn add_message(&mut self, message: BroadcastMessage) {
        let channel_messages = self.messages.entry(message.channel_id.clone()).or_default();
        channel_messages.push_back(message);
        self.message_scroll_offset = 0;
    }

    pub fn prepend_history(&mut self, channel_id: &str, history: Vec<BroadcastMessage>) {
        if history.is_empty() {
            if let Some(state) = self.channel_history_state.get_mut(channel_id) {
                state.1 = false;
            }
            return;
        }

        let channel_messages = self.messages.entry(channel_id.to_string()).or_default();
        for msg in history.into_iter().rev() {
            channel_messages.push_front(msg);
        }

        if let Some(state) = self.channel_history_state.get_mut(channel_id) {
            state.0 += 50;
        }
    }

    pub fn get_messages_for_channel(
        &self,
        channel_id: &str,
    ) -> Option<&VecDeque<BroadcastMessage>> {
        self.messages.get(channel_id)
    }

    pub fn add_or_update_channel(&mut self, new_channel: Channel) {
        if let Some(pos) = self.channels.iter().position(|c| c.id == new_channel.id) {
            self.channels[pos] = new_channel;
        } else {
            self.channels.push(new_channel.clone());
            self.messages.entry(new_channel.id.clone()).or_default();
        }
    }

    pub fn remove_channel(&mut self, channel_id: &str) {
        self.channels.retain(|c| c.id != channel_id);
        if let Some(current) = &self.current_channel {
            if current.id == channel_id {
                self.current_channel = None;
            }
        }
        self.messages.remove(channel_id);
        self.rendered_messages.remove(channel_id);
        self.channel_history_state.remove(channel_id);
    }

    pub fn scroll_messages_up(&mut self) {
        self.message_scroll_offset = self.message_scroll_offset.saturating_add(1);
    }

    pub fn scroll_messages_down(&mut self) {
        self.message_scroll_offset = self.message_scroll_offset.saturating_sub(1);
    }

    pub fn clear_expired_notification(&mut self) {
        if let Some(notification) = &self.notification {
            if notification.created_at.elapsed() > std::time::Duration::from_secs(5) {
                self.notification = None;
            }
        }
    }
}
