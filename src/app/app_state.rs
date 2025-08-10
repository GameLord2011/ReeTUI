use serde::{Deserialize, Serialize};
use crate::api::models::{BroadcastMessage, Channel};
use crate::app::{PopupState, TuiPage};
use crate::themes::{Theme, ThemeName, ThemesConfig};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum DebugView {
    Overview,
    WebSocket,
    Logs,
    AppState,
    Messages,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum ChatFocusedPane {
    ChannelList,
    Messages,
    Input,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub user_icon: Option<String>,
    pub channels: Vec<Channel>,
    pub current_channel: Option<Channel>,
    pub messages: HashMap<String, VecDeque<BroadcastMessage>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub rendered_messages: HashMap<String, HashMap<String, Vec<ratatui::text::Line<'static>>>>,
    pub needs_re_render: HashMap<String, HashMap<String, bool>>,
    pub last_chat_view_height: usize,
    pub total_chat_buffer_length: usize,
    pub message_scroll_offset: usize,
    pub themes: HashMap<ThemeName, Theme>,
    pub current_theme: Theme,
    pub settings_main_selection: usize,
    pub settings_focused_pane: crate::tui::settings::state::FocusedPane,
    pub quit_confirmation_state: crate::tui::settings::state::QuitConfirmationState,
    pub quit_selection: usize,
    pub show_settings: bool,
    pub popup_state: crate::app::PopupState,
    pub active_users: Vec<String>,
    pub mention_query: String,
    pub selected_mention_index: usize,
    pub emoji_query: String,
    pub selected_emoji_index: usize,
    pub cursor_position: usize,
    pub download_progress: u8,
    pub debug_json_content: String,
    pub notification_manager: crate::tui::notification::NotificationManager,
        pub should_exit_app: bool,
    pub next_page: Option<TuiPage>,
    pub channel_history_state: HashMap<String, (u64, bool, bool)>,
    #[serde(skip)]
    pub active_animations: std::collections::HashMap<
        String,
        std::sync::Arc<tokio::sync::Mutex<crate::tui::chat::gif_renderer::GifAnimationState>>,
    >,
    pub chat_width: u16,
    pub chat_focused_pane: ChatFocusedPane,
    pub websocket_latencies: Vec<(String, Duration)>,
    pub current_debug_view: DebugView,
    pub log_content: String,
    pub log_scroll_offset: usize,
    pub fps: f64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
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
            popup_state: PopupState::default(),
            message_scroll_offset: 0,
            current_theme: crate::themes::ThemesConfig::get_all_themes()
                .unwrap()
                .remove(&crate::themes::ThemeName::CatppuccinMocha)
                .unwrap(),
            themes: ThemesConfig::get_all_themes().unwrap(),
            active_users: Vec::new(),
            selected_mention_index: 0,
            selected_emoji_index: 0,
            mention_query: String::new(),
            emoji_query: String::new(),
            cursor_position: 0,
            notification_manager: crate::tui::notification::NotificationManager::default(),
            download_progress: 0,
            debug_json_content: String::new(),
            last_chat_view_height: 10,
            total_chat_buffer_length: 0,
                        should_exit_app: false,
            next_page: None,
            settings_main_selection: 0,
            settings_focused_pane: crate::tui::settings::state::FocusedPane::Left,
            quit_confirmation_state: crate::tui::settings::state::QuitConfirmationState::Inactive,
            quit_selection: 0,
            show_settings: false,
            active_animations: HashMap::new(),
            needs_re_render: HashMap::new(),
            chat_width: 80,
            chat_focused_pane: ChatFocusedPane::Input,
            websocket_latencies: Vec::new(),
            current_debug_view: DebugView::Overview,
            log_content: String::new(),
            log_scroll_offset: 0,
            fps: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0,
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

    pub fn get_current_theme(&self) -> &Theme {
        &self.current_theme
    }

    

    pub async fn clear_user_auth(&mut self) {
        self.auth_token = None;
        self.username = None;
        self.user_icon = None;
        self.current_channel = None;
        self.channels.clear();
        self.messages.clear();
        self.rendered_messages.clear();
        self.channel_history_state.clear();
                self.popup_state = PopupState::default();
        self.next_page = None;
        
        self.message_scroll_offset = 0;
        self.current_theme = self.themes.get(&ThemeName::Default).unwrap().clone();
        self.needs_re_render.clear();
    }
    pub fn set_current_channel(&mut self, channel: Channel) {
        let channel_id = channel.id.clone();
        self.current_channel = Some(channel);
        self.messages.entry(channel_id.clone()).or_default();
        self.rendered_messages
            .entry(channel_id.clone())
            .or_default();
        self.channel_history_state
            .entry(channel_id.clone())
            .or_insert((0, true, false));
        self.needs_re_render.entry(channel_id.clone()).or_default();
        self.message_scroll_offset = 0;
        // Mark all messages in the new channel for re-rendering
        if let Some(messages) = self.messages.get(&channel_id) {
            let needs_re_render_for_channel =
                self.needs_re_render.entry(channel_id.clone()).or_default();
            for msg in messages.iter() {
                let message_id = msg
                    .file_id
                    .clone()
                    .unwrap_or_else(|| msg.timestamp.to_string());
                needs_re_render_for_channel.insert(message_id, true);
            }
        }
    }

    pub fn add_message(&mut self, message: BroadcastMessage) {
        let channel_id = message.channel_id.clone();
        let message_id = message
            .file_id
            .clone()
            .unwrap_or_else(|| message.timestamp.to_string());
        let channel_messages = self.messages.entry(channel_id.clone()).or_default();

        // Check if the previous message needs re-rendering for grouping
        if let Some(last_msg) = channel_messages.back() {
            if last_msg.user == message.user
                && (message.timestamp - last_msg.timestamp).abs() < 60
                && last_msg.file_id.is_none()
                && !last_msg.is_image.unwrap_or(false)
                && message.file_id.is_none()
                && !message.is_image.unwrap_or(false)
            {
                let last_message_id = last_msg
                    .file_id
                    .clone()
                    .unwrap_or_else(|| last_msg.timestamp.to_string());
                self.needs_re_render
                    .entry(channel_id.clone())
                    .or_default()
                    .insert(last_message_id, true);
            }
        }

        channel_messages.push_back(message);
        self.rendered_messages
            .entry(channel_id.clone())
            .or_default()
            .remove(&message_id);
        self.message_scroll_offset = 0;
        self.needs_re_render
            .entry(channel_id)
            .or_default()
            .insert(message_id, true);
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
        let needs_re_render_for_channel = self
            .needs_re_render
            .entry(channel_id.to_string())
            .or_default();
        let rendered_messages_for_channel = self
            .rendered_messages
            .entry(channel_id.to_string())
            .or_default();

        for msg in history.into_iter().rev() {
            let message_id = msg
                .file_id
                .clone()
                .unwrap_or_else(|| msg.timestamp.to_string());
            channel_messages.push_front(msg);
            needs_re_render_for_channel.insert(message_id.clone(), true);
            rendered_messages_for_channel.remove(&message_id);
        }

        if let Some(state) = self.channel_history_state.get_mut(channel_id) {
            state.0 += 50;
        }
        // Mark all messages in the channel for re-rendering after history prepend
        let needs_re_render_for_channel = self
            .needs_re_render
            .entry(channel_id.to_string())
            .or_default();
        if let Some(messages) = self.messages.get(channel_id) {
            for msg in messages.iter() {
                let message_id = msg
                    .file_id
                    .clone()
                    .unwrap_or_else(|| msg.timestamp.to_string());
                needs_re_render_for_channel.insert(message_id, true);
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_messages_for_channel(
        &self,
        channel_id: &str,
    ) -> Option<&VecDeque<BroadcastMessage>> {
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
        self.needs_re_render.remove(channel_id);
    }

    pub fn scroll_messages_up(&mut self, scroll_amount: usize) {
        let max_offset = self
            .total_chat_buffer_length
            .saturating_sub(self.last_chat_view_height)
            .max(0);
        self.message_scroll_offset = (self.message_scroll_offset + scroll_amount).min(max_offset);
    }

    pub fn scroll_messages_down(&mut self, scroll_amount: usize) {
        self.message_scroll_offset = self.message_scroll_offset.saturating_sub(scroll_amount);
    }

    

    pub fn find_message_mut(&mut self, message_id: &str) -> Option<&mut BroadcastMessage> {
        for (_channel_id, messages) in self.messages.iter_mut() {
            for message in messages.iter_mut() {
                if let Some(file_id) = &message.file_id {
                    if file_id == message_id {
                        return Some(message);
                    }
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn update_message(&mut self, updated_message: BroadcastMessage) {
        let message_id = updated_message
            .file_id
            .clone()
            .unwrap_or_else(|| updated_message.timestamp.to_string());
        if let Some(channel_messages) = self.messages.get_mut(&updated_message.channel_id) {
            for (i, msg) in channel_messages.iter().enumerate() {
                // 1. prefer file_id if both present
                if msg.file_id.is_some() && updated_message.file_id.is_some() {
                    if msg.file_id == updated_message.file_id {
                        channel_messages[i] = updated_message;
                        self.rendered_messages
                            .entry(channel_messages[i].channel_id.clone())
                            .or_default()
                            .remove(&message_id);
                        self.needs_re_render
                            .entry(channel_messages[i].channel_id.clone())
                            .or_default()
                            .insert(message_id, true);
                        return;
                    } else {
                        continue;
                    }
                // 2. else, match by timestamp if both present
                } else if msg.timestamp == updated_message.timestamp {
                    channel_messages[i] = updated_message;
                    self.rendered_messages
                        .entry(channel_messages[i].channel_id.clone())
                        .or_default()
                        .remove(&message_id);
                    self.needs_re_render
                        .entry(channel_messages[i].channel_id.clone())
                        .or_default()
                        .insert(message_id, true);
                    return;
                // 3. elsif( jk ), match by file_name if both present and neither file_id nor timestamp is present
                } else if msg.file_name.is_some()
                    && updated_message.file_name.is_some()
                    && msg.file_name == updated_message.file_name
                {
                    channel_messages[i] = updated_message;
                    self.rendered_messages
                        .entry(channel_messages[i].channel_id.clone())
                        .or_default()
                        .remove(&message_id);
                    self.needs_re_render
                        .entry(channel_messages[i].channel_id.clone())
                        .or_default()
                        .insert(message_id, true);
                    return;
                }
            }
        }
        self.needs_re_render
            .entry(updated_message.channel_id)
            .or_default()
            .insert(message_id, true);
    }
}
