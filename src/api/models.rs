use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct AuthRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Serialize)]
pub struct RegisterRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub icon: &'a str,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub icon: String,
}

#[derive(Serialize)]
pub struct ChannelCommand<'a> {
    pub channel_id: &'a str,
    pub content: &'a str,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastMessage {
    #[serde(default)]
    pub client_id: Option<String>,
    pub user: String,
    pub icon: String,
    pub content: String,
    pub timestamp: i64,
    pub channel_id: String,
    #[serde(default)]
    pub channel_name: String,
    #[serde(default)]
    pub channel_icon: String,
    #[serde(default = "default_message_type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size_mb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_image: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_progress: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gif_frames: Option<Vec<(String, Duration)>>,
}

fn default_message_type() -> String {
    "text".to_string()
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ChannelBroadcast {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryResponse {
    pub history: Vec<BroadcastMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notification {
    pub target_user: String,
    pub message: String,
    pub channel_id: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ActiveUsersResponse {
    pub active_users: Vec<String>,
}
