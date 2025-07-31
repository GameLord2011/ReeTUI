use reqwest::StatusCode;
use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    UsernameTaken,
    Unauthorized,
    RequestFailed(reqwest::Error),
    ServerError(StatusCode),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::UsernameTaken => write!(f, "This username is already taken."),
            AuthError::Unauthorized => write!(f, "Invalid username or password."),
            AuthError::RequestFailed(e) => write!(f, "Request failed: {}", e),
            AuthError::ServerError(sc) => write!(f, "Server returned an error: {}", sc),
        }
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(err: reqwest::Error) -> Self {
        AuthError::RequestFailed(err)
    }
}
