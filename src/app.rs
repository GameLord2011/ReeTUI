use crate::api::models::{BroadcastMessage, Channel};
use crate::tui::themes::ThemeName;
use ratatui::text::Line;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
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
    pub duration: Duration,
}

impl Notification {
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupType {
    Quit,
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
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub user_icon: Option<String>,
    pub current_channel: Option<Channel>,
    pub channels: Vec<Channel>,
    pub messages: HashMap<String, VecDeque<BroadcastMessage>>,
    pub rendered_messages: HashMap<String, Vec<Line<'static>>>,
    pub channel_history_state: HashMap<String, (usize, bool)>,
    pub active_animations: HashMap<String, Arc<Mutex<crate::tui::chat::gif_renderer::GifAnimationState>>>, // Replaces old animation fields
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
    pub terminal_width: u16,
    pub last_chat_view_height: usize, // funny
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
            active_animations: HashMap::new(), // New
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
            terminal_width: 0,
            last_chat_view_height: 10, // default to 10, will be set by UI
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

    pub fn set_notification(
        &mut self,
        title: String,
        message: String,
        notification_type: NotificationType,
        duration_in_seconds: u64,
    ) {
        self.notification = Some(Notification {
            title,
            message,
            notification_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(duration_in_seconds),
        });
        self.popup_state.show = true;
        self.popup_state.popup_type = PopupType::Notification;
    }

    pub fn set_download_progress_popup(&mut self, progress: u8) {
        self.download_progress = progress;
        self.popup_state.show = true;
        self.popup_state.popup_type = PopupType::DownloadProgress;
    }

    #[allow(dead_code)]
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
    // Clean up GIF animation threads
for (_file_id, animation_arc) in self.active_animations.iter_mut() {
    if let Ok(mut animation) = animation_arc.lock() {
        animation.running = false;
        if let Some(handle) = animation.thread_handle.take() {
            let _ = handle.join();
        }
    }
}    self.active_animations.clear(); // New
    self.popup_state = PopupState::default();
    self.notification = None;
    self.message_scroll_offset = 0;
    self.current_theme = ThemeName::Default;
}
    pub fn set_current_channel(&mut self, channel: Channel) {
        let channel_id = channel.id.clone();
        self.current_channel = Some(channel);
        self.messages.entry(channel_id.clone()).or_default();
        self.rendered_messages.entry(channel_id.clone()).or_default();
        self.channel_history_state
            .entry(channel_id)
            .or_insert((0, true));
        self.message_scroll_offset = 0;
    }

    pub fn add_message(&mut self, message: BroadcastMessage) {
        let channel_id = message.channel_id.clone();
        let channel_messages = self.messages.entry(channel_id.clone()).or_default();
        channel_messages.push_back(message);
        self.rendered_messages.remove(&channel_id);
        self.message_scroll_offset = 0;
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_messages_for_channel(&self, channel_id: &str) -> Option<&VecDeque<BroadcastMessage>> {
        self.messages.get(channel_id)
    }

    #[allow(dead_code)]
    pub fn add_or_update_channel(&mut self, new_channel: Channel) {
        if let Some(pos) = self.channels.iter().position(|c| c.id == new_channel.id) {
            self.channels[pos] = new_channel;
        } else {
            self.channels.push(new_channel.clone());
            self.messages.entry(new_channel.id.clone()).or_default();
        }
    }

    #[allow(dead_code)]
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

    pub fn scroll_messages_up(&mut self, message_count: usize, view_height: usize) {
        let max_offset = message_count.saturating_sub(view_height).max(0);
        if self.message_scroll_offset < max_offset {
            self.message_scroll_offset += 1;
        }
        if self.message_scroll_offset > max_offset {
            self.message_scroll_offset = max_offset;
        }
        log::debug!("scroll up: offset={}, max_offset={}, message_count={}, view_height={}", self.message_scroll_offset, max_offset, message_count, view_height);
    }

    pub fn scroll_messages_down(&mut self) {
        if self.message_scroll_offset > 0 {
            self.message_scroll_offset -= 1;
        }
        log::debug!("scroll down: offset={}", self.message_scroll_offset);
    }

    #[allow(dead_code)]
    pub fn clear_expired_notification(&mut self) {
        if let Some(notification) = &self.notification {
            if notification.created_at.elapsed() > std::time::Duration::from_secs(5) {
                self.notification = None;
            }
        }
    }

    #[allow(dead_code)]
    pub fn update_message(&mut self, updated_message: BroadcastMessage) {
        if let Some(channel_messages) = self.messages.get_mut(&updated_message.channel_id) {
            // Try to match by file_id and timestamp
            for (i, msg) in channel_messages.iter().enumerate() {
                log::debug!("Checking message for update: idx={} file_id={:?} timestamp={} file_name={:?}", i, msg.file_id, msg.timestamp, msg.file_name);
                // 1. Prefer file_id if both present
                if msg.file_id.is_some() && updated_message.file_id.is_some() {
                    if msg.file_id == updated_message.file_id {
                        log::debug!("Matched by file_id at idx={}!", i);
                        channel_messages[i] = updated_message;
                        self.rendered_messages.remove(&channel_messages[i].channel_id);
                        return;
                    } else {
                        continue;
                    }
                // 2. Else, match by timestamp if both present
                } else if msg.timestamp == updated_message.timestamp {
                    log::debug!("Matched by timestamp at idx={}!", i);
                    channel_messages[i] = updated_message;
                    self.rendered_messages.remove(&channel_messages[i].channel_id);
                    return;
                // 3. Else, match by file_name if both present and neither file_id nor timestamp is present
                } else if msg.file_name.is_some() && updated_message.file_name.is_some() && msg.file_name == updated_message.file_name {
                    log::debug!("Matched by file_name at idx={} (no file_id/timestamp)!", i);
                    channel_messages[i] = updated_message;
                    self.rendered_messages.remove(&channel_messages[i].channel_id);
                    return;
                }
            }
            log::warn!("No matching message found to update: file_id={:?} timestamp={:?} file_name={:?}", updated_message.file_id, updated_message.timestamp, updated_message.file_name);
        }
    }
}