use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Default, ToSchema)]
pub struct HealthCheck {
    pub database: bool,
    pub kv: bool,
}

/// The response we receive from Discord after the OAuth redirect.
#[derive(Debug, Deserialize)]
pub struct DiscordOAuthResponse {
    pub code: String,
}

/// Discord session data returned when authorizing the access token.
#[derive(Debug, Deserialize, Serialize)]
pub struct DiscordSessionData {
    pub access_token: String,
    pub expires_in: super::discord::TokenExpiryTimestamp,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AuthorizedIdentity {
    pub scopes: Vec<String>,
    pub user: AuthorizedUser,
    pub expires: chrono::DateTime<chrono::Utc>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AuthorizedApplication {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub hook: bool,
    pub verify_key: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AuthorizedUser {
    pub id: String,
    pub username: String,
    pub avatar: String,
    pub discriminator: String,
    pub global_name: String,
    pub public_flags: u32,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct Guild {
    /// The unique Id of the guild.
    ///
    /// Can be used to calculate creation date.
    pub id: String,
    /// The name of the guild.
    pub name: String,
    /// The hash of the icon of the guild.
    ///
    /// This can be used to generate a URL to the guild's icon image.
    pub icon: Option<String>,
    /// Indicator of whether the current user is the owner.
    pub owner: bool,
    /// The permissions that the current user has.
    pub permissions: u64,
    /// See [`Guild::features`].
    pub features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    pub fn default_healthcheck_fails() -> Result<()> {
        let health_check = HealthCheck::default();

        assert!(!health_check.kv);
        assert!(!health_check.database);

        Ok(())
    }
}
