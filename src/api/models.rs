use serde::{Deserialize, Serialize};

// for login request
#[derive(Serialize)]
pub struct AuthRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
}
// hello bruh, where is this
// for register reqwest
#[derive(Serialize)]
pub struct RegisterRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub icon: &'a str,
}

// for auth responses
#[derive(Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub icon: String,
}

// for sending a message to a channel
#[derive(Serialize)]
pub struct ChannelCommand<'a> {
    pub channel_id: &'a str,
    pub content: &'a str,
}

// for receiving a broadcasted message from a channel
#[derive(Deserialize, Debug, Clone)]
pub struct BroadcastMessage {
    pub user: String,
    pub icon: String,
    pub content: String,
    pub timestamp: i64,
    pub channel_id: String,
}

// for receiving freaking channel update
#[derive(Deserialize, Debug, Clone)]
pub struct ChannelBroadcast {
    pub id: String,
    pub name: String,
    pub icon: String,
}

// Definition for a Channel, used for managing channel lists in the client
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
