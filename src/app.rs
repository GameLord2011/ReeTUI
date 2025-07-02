use crate::api::models::{BroadcastMessage, Channel};
use std::collections::HashMap;

/// Represents the overall application state.
#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub user_icon: Option<String>,
    pub current_channel: Option<Channel>,
    pub channels: Vec<Channel>,
    pub messages: HashMap<String, Vec<BroadcastMessage>>,
}

impl AppState {
    /// Creates a new, default instance of `AppState`.
    pub fn new() -> Self {
        AppState::default()
    }

    /// Sets the user's authentication token, username, and icon after a successful login/registration.
    pub fn set_user_auth(&mut self, token: String, username: String, icon: String) {
        self.auth_token = Some(token);
        self.username = Some(username);
        self.user_icon = Some(icon);
        println!(
            "App State: User authenticated: {}",
            self.username.as_ref().unwrap()
        );
    }

    /// Clears the user's authentication data, effectively logging them out.
    pub fn clear_user_auth(&mut self) {
        self.auth_token = None;
        self.username = None;
        self.user_icon = None;
        self.current_channel = None;
        self.channels.clear();
        self.messages.clear();
        println!("App State: User logged out.");
    }

    /// Sets the current active channel.
    pub fn set_current_channel(&mut self, channel: Channel) {
        self.current_channel = Some(channel.clone());
        self.messages
            .entry(channel.id.clone())
            .or_insert_with(Vec::new);
        println!("App State: Current channel set to: {}", channel.name);
    }

    /// Adds a new message to the application's message history,
    /// storing it under its respective channel ID.
    pub fn add_message(&mut self, message: BroadcastMessage) {
        let channel_messages = self
            .messages
            .entry(message.channel_id.clone())
            .or_insert_with(Vec::new);
        channel_messages.push(message);
    }

    pub fn get_messages_for_channel(&self, channel_id: &str) -> Option<&Vec<BroadcastMessage>> {
        self.messages.get(channel_id)
    }

    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.channels = channels;
        println!(
            "E: Updated channels list. Total channels: {}",
            self.channels.len()
        );
    }

    pub fn add_or_update_channel(&mut self, new_channel: Channel) {
        if let Some(pos) = self.channels.iter().position(|c| c.id == new_channel.id) {
            self.channels[pos] = new_channel;
            println!("DIBOG: {}", self.channels[pos].name);
        } else {
            self.channels.push(new_channel.clone());
            self.messages
                .entry(new_channel.id.clone())
                .or_insert_with(Vec::new);
            println!("DIBUG: {}", new_channel.name);
        }
    }

    pub fn remove_channel(&mut self, channel_id: &str) {
        self.channels.retain(|c| c.id != channel_id);
        if let Some(current) = &self.current_channel {
            if current.id == channel_id {
                self.current_channel = None;
                println!("DEBAG: Current channel removed and cleared.");
            }
        }
        self.messages.remove(channel_id);
        println!("DE the BUG: Removed channel with ID: {}", channel_id);
    }
}
