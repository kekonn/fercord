use serde::{Deserialize, Serialize};
use serenity::http::Http;
use anyhow::{anyhow, Context, Result};
use serenity::all::GuildInfo;
use crate::model::SessionData;

const DISCORD_BASE_URL: &str = "https://discord.com/";

/// Client that connects to the Discord API.
pub struct Client {
    client: Http,
    expires_in: usize,
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct TokenExchangeRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
}

impl TokenExchangeRequest {
    pub fn new(code: String, redirect_uri: String) -> Self {
        Self {
            code,
            redirect_uri,
            grant_type: "authorization_code".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
    pub refresh_token: String,
    pub scope: String,
}

impl Into<crate::model::SessionData> for Client {
    fn into(self) -> SessionData {
        SessionData {
            expires_in: self.expires_in,
            access_token: self.client.token().into(),
            refresh_token: self.refresh_token,
        }
    }
}

/// Constructor implementations
impl Client {
    /// Attempts to exchange the given authorization code for an access token.
    pub async fn try_from_auth_code(code: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::new();
        let response = client.post(format!("{}{}", DISCORD_BASE_URL, "api/oauth2/token"))
            .form(&TokenExchangeRequest::new(code.into(), "https://webhook.site/276aac7a-574b-4346-8a4a-0d32a612cc6d".into()))
            .send().await?;

        if response.status().is_success() {
            let token_response = response.json::<AccessTokenResponse>().await.with_context(|| "Error deserializing discord token response")?;
            return Ok(Self {
                client: Http::new(token_response.access_token.as_str()),
                expires_in: token_response.expires_in,
                refresh_token: token_response.refresh_token,
            });
        }

        Err(anyhow!(response.status()))
    }

    pub fn from_access_token(access_token: impl Into<String>, refresh_token: impl Into<String>, expires_in: usize) -> Self {
        Self {
            client: Http::new(access_token.into().as_str()),
            refresh_token: refresh_token.into(),
            expires_in,
        }
    }
}

/// self implementations
impl Client {
    pub async fn get_managing_guilds(&self) -> Result<Vec<GuildInfo>> {
        Ok(self.client.get_guilds(None, None).await?
            .into_iter().filter(|g| g.permissions.iter().any(|p| p.manage_guild()))
            .collect::<Vec<GuildInfo>>())
    }
}