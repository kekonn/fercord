use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub database: bool,
    pub kv: bool,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            database: true,
            kv: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DiscordOAuthResponse {
    pub code: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionData {
    pub access_token: String,
    pub expires_in: usize,
    pub refresh_token: String,
}
