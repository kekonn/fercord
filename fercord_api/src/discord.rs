use crate::model;
use actix_web::http::StatusCode;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use reqwest::{header, Client as HttpClient};
use serde::{Deserialize, Serialize};
use serenity::all::GuildInfo;
use std::collections::HashMap;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::sync::RwLock;
use thiserror::Error;
use tracing::{event, event_enabled, Level};

const DISCORD_API_URL: &str = "https://discord.com/api";

/// Type alias for access token expiration
pub type TokenExpiryTimestamp = DateTime<Utc>;

/// Client that connects to the Discord API.
#[derive(Debug)]
pub struct Client {
    store: RwLock<ClientStore>,
}

#[derive(Debug)]
struct ClientStore {
    access_token: String,
    refresh_token: String,
    /// UTC Timestamp indicating when the token expires
    expires_at: TokenExpiryTimestamp,
    client_id: NonZeroU64,
    client_secret: String,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Authentication error: {status}")]
    Authentication { status: u16, reason: String },
    #[error("(De)serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

impl actix_web::ResponseError for ClientError {
    #[tracing::instrument(name = "client.error", level = Level::TRACE)]
    fn status_code(&self) -> StatusCode {
        match &self {
            &ClientError::Http(e) => {
                if let Some(status_code) = e.status() {
                    event!(Level::TRACE, %e, "Source error is http client error with status code {:?}", status_code);
                    return StatusCode::from_u16(status_code.as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                }

                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Serialize)]
struct TokenExchangeRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Default for TokenExchangeRequest {
    fn default() -> Self {
        Self {
            code: "".into(),
            redirect_uri: "".into(),
            client_id: "".into(),
            client_secret: "".into(),
            grant_type: "authorization_code".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
}

impl Into<crate::model::DiscordSessionData> for Client {
    fn into(self) -> model::DiscordSessionData {
        let store = &self.store.read().unwrap();
        model::DiscordSessionData {
            expires_in: store.expires_at,
            access_token: store.access_token.clone(),
            refresh_token: store.refresh_token.clone(),
        }
    }
}


/// Constructor implementations
impl Client {
    /// Attempts to exchange the given authorization code for an access token.
    #[tracing::instrument(name = "client.init", skip_all)]
    pub async fn try_from_auth_code(
        code: impl Into<String>,
        client_id: NonZeroU64,
        client_secret: impl Into<String>,
    ) -> Result<Self, ClientError> {
        let client = reqwest::Client::new();
        let client_secret = client_secret.into();

        let body = TokenExchangeRequest {
            code: code.into(),
            redirect_uri: "https://tuxgamer.nessie-arctic.ts.net/discord_auth".into(),
            client_id: client_id.to_string(),
            client_secret: client_secret.clone(),
            ..Default::default()
        };

        let response = client
            .post(format!("{}/oauth2/token", DISCORD_API_URL))
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
                status: resp_status.as_u16(),
            });
        }

        let token_response = response
            .json::<AccessTokenResponse>()
            .await
            .map_err(ClientError::from)?;

        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        if event_enabled!(Level::TRACE) {
            event!(
                Level::TRACE,
                "Received a successful response that is valid until {}",
                &expires_at.to_rfc3339_opts(SecondsFormat::Secs, true)
            );
        }
        Ok(Self {
            store: RwLock::new(ClientStore {
                access_token: token_response.access_token,
                expires_at,
                refresh_token: token_response.refresh_token,
                client_id,
                client_secret,
            }),
        })
    }

    /// Create a `Client` from a discord JWT and client id and secret.
    pub fn from_access_token(
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
        expires_at: TokenExpiryTimestamp,
        client_id: NonZeroU64,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            store: RwLock::new(ClientStore {
                access_token: access_token.into(),
                refresh_token: refresh_token.into(),
                expires_at,
                client_secret: client_secret.into(),
                client_id,
            }),
        }
    }
}

/// self implementations
impl Client {
    #[tracing::instrument(name = "discord.client", skip(self))]
    pub async fn get_user_identity(&self) -> Result<model::AuthorizedIdentity, ClientError> {
        let client = self.create_client().await?;
        let response = client
            .get(format!("{}/oauth2/@me", DISCORD_API_URL))
            .send()
            .await
            .map_err(ClientError::from)?;

        event!(Level::TRACE, "{}: {}", response.url(), response.status());

        let resp_status = response.status();
        if !resp_status.is_success() {
            return Err(ClientError::Authentication {
                status: resp_status.as_u16(),
                reason: response.text().await?,
            });
        }

        let body_text = response.text().await?;

        process_response(body_text.as_str())
    }

    #[tracing::instrument(name = "discord.client", skip(self))]
    pub async fn get_managing_guilds(&self) -> Result<Vec<GuildInfo>, ClientError> {
        let client = self.create_client().await?;
        let response = client.get(format!("{}/users/@me/guilds", DISCORD_API_URL))
            .send().await.map_err(ClientError::from)?;
        
        event!(Level::TRACE, "{}: {}", response.url(), response.status());

        let response_body = response.text().await.map_err(ClientError::from)?;

        process_response(response_body.as_str())
    }

    #[tracing::instrument(name = "discord.client", skip(self))]
    pub async fn logout(&self) -> Result<(), ClientError> {
        let client = self.create_client().await?;

        let store = self.store.read().unwrap();

        let response = client
            .post(format!("{}/oauth2/token/revoke", DISCORD_API_URL))
            .form(&HashMap::from([
                ("token", store.refresh_token.as_str()),
                ("token_hint", "refresh_token"),
                ("client_id", store.client_id.to_string().as_str()),
                ("client_secret", store.client_secret.as_str()),
            ]))
            .send()
            .await?;

        response.error_for_status().map(|_| ()).map_err(ClientError::from)
    }
}

/// private self
impl Client {
    /// Creates a Serenity Http client.
    async fn create_client(&self) -> Result<HttpClient, ClientError> {
        let mut headers = header::HeaderMap::new();
        let store = self.store.read().unwrap();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", &store.access_token).parse().unwrap(),
        );
        headers.insert(
            header::USER_AGENT,
            format!("DiscordBot ({},{})", crate::REPO_URL, crate::VERSION)
                .parse()
                .unwrap(),
        );

        event!(Level::TRACE, "Creating HTTP client with headers: {:?}", headers);

        let client = reqwest::Client::builder()
            .https_only(true)
            .default_headers(headers)
            .build()
            .map_err(ClientError::from)?;

        if self.is_token_expired() {
            event!(Level::DEBUG, "Current token is expired. Refreshing token");
            // TODO: refresh token
            todo!()
        }

        Ok(client)
    }

    /// Check if the token is expired
    #[inline]
    fn is_token_expired(&self) -> bool {
        let now = Utc::now();
        let store = self.store.read().unwrap();
        let expiry = store.expires_at;
        event!(
            Level::TRACE,
            "Comparing token expiration ({}) against UTC now ({})",
            &expiry.to_rfc3339(),
            &now.to_rfc3339()
        );

        let time_remaining = expiry - now;
        time_remaining <= Duration::zero()
    }
}

/// Convert a response body into the given struct.
fn process_response<'a, T: serde::Deserialize<'a> + Debug>(
    body: &'a str,
) -> Result<T, ClientError> {
    let resp_json_res = serde_json::from_str::<T>(body);

    if resp_json_res.is_err() && event_enabled!(Level::DEBUG) {
        let error = resp_json_res
            .as_ref()
            .expect_err("There should be an error here");
        event!(Level::DEBUG, %body, "Error decoding JSON response: {}", error.to_string());
    }

    resp_json_res.map_err(ClientError::from)
}
