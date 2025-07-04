use crate::api::models::{BroadcastMessage, Channel};
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

/// Enum to represent the type of pop-up to display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupType {
    Quit,
    Settings,
    CreateChannel, // New pop-up type for creating channels
    None,          // No pop-up active
}

/// Struct to manage the state of a pop-up.
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
    pub messages: HashMap<String, Vec<BroadcastMessage>>,
    pub animation_frame_index: usize,
    pub last_frame_time: Instant,
    pub popup_state: PopupState,       // New field for pop-up management
    pub error_message: Option<String>, // New field for error messages
    pub error_display_until: Option<Instant>, // New field for error display duration
    pub message_scroll_offset: usize,  // New field for message scrolling
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
            // Default values for animation fields
            animation_frame_index: 0,
            last_frame_time: Instant::now(), // Initialize with the current time
            popup_state: PopupState::default(), // Initialize pop-up state
            error_message: None,             // No error initially
            error_display_until: None,       // No error display time initially
            message_scroll_offset: 0,        // Initialize scroll offset
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

    pub fn clear_user_auth(&mut self) {
        self.auth_token = None;
        self.username = None;
        self.user_icon = None;
        self.current_channel = None;
        self.channels.clear();
        self.messages.clear();
        self.animation_frame_index = 0;
        self.last_frame_time = Instant::now();
        self.popup_state = PopupState::default(); // Reset pop-up state on clear
        self.error_message = None; // Clear error message
        self.error_display_until = None; // Clear error display time
        self.message_scroll_offset = 0; // Reset scroll offset
    }

    pub fn set_current_channel(&mut self, channel: Channel) {
        self.current_channel = Some(channel.clone());
        self.messages
            .entry(channel.id.clone())
            .or_insert_with(Vec::new);
        self.message_scroll_offset = 0; // Reset scroll on channel change
    }

    pub fn add_message(&mut self, message: BroadcastMessage) {
        let channel_messages = self
            .messages
            .entry(message.channel_id.clone())
            .or_insert_with(Vec::new);
        channel_messages.push(message);
        // Auto-scroll to the bottom when a new message arrives
        self.message_scroll_offset = 0;
    }

    pub fn get_messages_for_channel(&self, channel_id: &str) -> Option<&Vec<BroadcastMessage>> {
        self.messages.get(channel_id)
    }

    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.channels = channels;
    }

    pub fn add_or_update_channel(&mut self, new_channel: Channel) {
        if let Some(pos) = self.channels.iter().position(|c| c.id == new_channel.id) {
            self.channels[pos] = new_channel;
        } else {
            self.channels.push(new_channel.clone());
            self.messages
                .entry(new_channel.id.clone())
                .or_insert_with(Vec::new);
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
    }

    /// Sets an error message to be displayed for a short duration.
    pub fn set_error_message(&mut self, message: String, duration_ms: u64) {
        self.error_message = Some(message);
        self.error_display_until = Some(Instant::now() + Duration::from_millis(duration_ms));
    }

    /// Clears the error message if its display duration has passed.
    pub fn clear_expired_error(&mut self) {
        if let Some(display_until) = self.error_display_until {
            if Instant::now() >= display_until {
                self.error_message = None;
                self.error_display_until = None;
            }
        }
    }

    /// Scrolls messages up by one line.
    pub fn scroll_messages_up(&mut self) {
        self.message_scroll_offset = self.message_scroll_offset.saturating_add(1);
    }

    /// Scrolls messages down by one line.
    pub fn scroll_messages_down(&mut self) {
        self.message_scroll_offset = self.message_scroll_offset.saturating_sub(1);
    }
}
