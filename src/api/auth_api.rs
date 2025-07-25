use crate::api::error::AuthError;
use crate::api::models::AuthRequest;
use crate::api::models::RegisterRequest;
use crate::api::models::TokenResponse;
use reqwest::Client;
use reqwest::StatusCode;

const API_BASE_URL: &str = "https:back.reetui.hackclub.app";

pub async fn register(
    client: &Client,
    username: &str,
    password: &str,
    icon: &str,
) -> Result<TokenResponse, AuthError> {
    let endpoint = format!("{}/auth/register", API_BASE_URL); // funny
                                                              // meow
    let payload = RegisterRequest {
        username,
        password,
        icon,
    };
    let response = client.post(&endpoint).json(&payload).send().await?;
    let status = response.status();

    if status.is_success() {
        Ok(response.json::<TokenResponse>().await?)
    } else {
        if status == StatusCode::CONFLICT {
            Err(AuthError::UsernameTaken)
        } else {
            Err(AuthError::ServerError(status))
        }
    }
}

pub async fn login(
    client: &Client,
    username: &str,
    password: &str,
) -> Result<TokenResponse, AuthError> {
    let endpoint = format!("{}/auth/login", API_BASE_URL);
    let payload = AuthRequest { username, password };
    let response = client.post(&endpoint).json(&payload).send().await?;
    let status = response.status();

    if status.is_success() {
        let full_response_body = response.text().await?.to_string();
        match serde_json::from_str::<TokenResponse>(&full_response_body) {
            Ok(token_response) => Ok(token_response),
            Err(_e) => {
                // Removed error logging as per user request
                Err(AuthError::ServerError(status))
            }
        }
    } else {
        if status == StatusCode::UNAUTHORIZED {
            Err(AuthError::Unauthorized)
        } else {
            Err(AuthError::ServerError(status))
        }
    }
}
