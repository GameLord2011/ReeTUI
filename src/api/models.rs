use serde::{Deserialize, Serialize};

// its more undertsanble without comments
// gonna comments about it
// no comment about

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

#[derive(Deserialize, Debug, Clone)]
pub struct BroadcastMessage {
    pub user: String,
    pub icon: String,
    pub content: String,
    pub timestamp: i64,
    pub channel_id: String,
    #[serde(default = "default_message_type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
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
}

fn default_message_type() -> String {
    "text".to_string()
}

#[derive(Deserialize, Debug, Clone)]
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
pub struct ActiveUsersResponse {
    pub active_users: Vec<String>,
}
