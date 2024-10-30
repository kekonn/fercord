use crate::model::SessionData;
use chrono::{Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serenity::all::GuildInfo;
use serenity::http::Http;
use thiserror::Error;
use tracing::{event, event_enabled, Level};

const DISCORD_BASE_URL: &str = "https://discord.com/";

/// Client that connects to the Discord API.
pub struct Client {
    client: Http,
    expires_in: usize,
    refresh_token: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Authentication error: {status}")]
    Authentication { status: String, reason: String },
    #[error("(De)serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::prelude::SerenityError),
}

#[derive(Debug, Serialize)]
struct TokenExchangeRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
}

impl TokenExchangeRequest {
    pub fn new(
        code: String,
        redirect_uri: String,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            code,
            redirect_uri,
            client_id,
            client_secret,
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
    #[tracing::instrument(name = "api.discord.auth", skip_all)]
    pub async fn try_from_auth_code(
        code: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<Self, ClientError> {
        let client = reqwest::Client::new();
        let client_secret = client_secret.into();
        let client_id = client_id.into();

        let body = TokenExchangeRequest::new(
            code.into(),
            "https://tuxgamer.nessie-arctic.ts.net/discord_auth".into(),
            client_id.clone(),
            client_secret.clone(),
        );

        let response = client
            .post(format!("{}{}", DISCORD_BASE_URL, "api/oauth2/token"))
            .form(&body)
            .send()
            .await
            .map_err(ClientError::from)?;

        if event_enabled!(Level::TRACE) {
            event!(
                Level::TRACE,
                "Auth response status code: {}",
                &response.status()
            );
            event!(Level::TRACE, "Response headers: {:?}", &response.headers());
        }

        let resp_status = response.status();

        if !resp_status.is_success() {
            event!(
                Level::ERROR,
                "Discord returned following status code during authentication: {}",
                resp_status
            );

            let resp_text = response.text().await.expect("Error reading response body");

            if event_enabled!(Level::TRACE) {
                event!(Level::TRACE, "Discord response: {}", &resp_text);
            }

            return Err(ClientError::Authentication {
                reason: resp_text,
                status: resp_status.to_string(),
            });
        }

        let token_response = response
            .json::<AccessTokenResponse>()
            .await
            .map_err(ClientError::from)?;

        if event_enabled!(Level::TRACE) {
            let valid_until = Utc::now() + Duration::seconds(token_response.expires_in as i64);
            event!(
                Level::TRACE,
                "Received a successful response that is valid until {}",
                valid_until.to_rfc3339_opts(SecondsFormat::Secs, true)
            );
        }
        Ok(Self {
            client: Http::new(token_response.access_token.as_str()),
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        })
    }

    pub fn from_access_token(
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
        expires_in: usize,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            client: Http::new(access_token.into().as_str()),
            refresh_token: refresh_token.into(),
            expires_in,
            client_secret: client_secret.into(),
            client_id: client_id.into(),
        }
    }
}

/// self implementations
impl Client {
    pub async fn get_managing_guilds(&self) -> Result<Vec<GuildInfo>, ClientError> {
        Ok(self
            .client
            .get_guilds(None, None)
            .await
            .map_err(ClientError::from)?
            .into_iter()
            .filter(|g| g.permissions.iter().any(|p| p.manage_guild()))
            .collect::<Vec<GuildInfo>>())
    }
}
