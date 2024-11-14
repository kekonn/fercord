//! Everything used to handle the application configuration.
//!
//! Create a new configuration as follows:
//! ```rust
//! use fercord_common::prelude::*;
//! let config = DiscordConfig::from_env().unwrap();
//! // or when you want to use a file and only overwrite from env
//! let config = DiscordConfig::from_env_and_file("../.config/config.toml").unwrap();
//! ```

use std::num::NonZeroU64;

use tracing::{event, Level};

pub use figment::Error;
use figment::{providers::{Env, Format, Toml}, Figment};

/// The application configuration.
///
/// You can use [from_env()](#from_env) or [from_env_and_file(path: &str)](#from_env_and_file) to create a configuration.
///
/// Settings:
/// * `discord_token`: `String`
/// * `database_url`: `String`
/// * `redis_url`: `String`
/// * `job_interval_min`: `u32`
/// * `session_key`: `String`
/// * `client_id`: `NonZeroU64`
/// * `client_secret`: `String`
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct DiscordConfig {
    /// The Discord API token.
    pub discord_token: String,
    /// The url for the database.
    ///
    /// Usual format: `postgres://usernane:pw@server/db`
    pub database_url: String,
    /// Url to indicate the redis instance to use.
    pub redis_url: String,
    /// Job interval in minutes
    pub job_interval_min: u32,
    /// The unique shard key that defines this bot server.
    ///
    /// Used when multiple servers share the same key-value store.
    pub shard_key: uuid::Uuid,
    /// Base64 encoded string of the secret session key. Key must be at least 64 bytes in length.
    ///
    /// This is used by the API.
    pub session_key: Option<String>,
    /// Discord OAuth Client Id.
    ///
    /// This is used by the API
    pub client_id: Option<NonZeroU64>,
    /// Discord OAuth client secret. For security reasons this can only be set from the environment.
    ///
    /// This is used by the API.
    pub client_secret: Option<String>,
}

const ENV_PREFIX: &str = "FERCORD_";

impl DiscordConfig {
    /// Create a configuration just from environment variables.
    ///  
    /// This will read all variables prefixed with `FERCORD_` and try to serialize them into a `DiscordConfig`.
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env() -> Result<Self, Error> {
        let figment = Figment::new()
            .merge(Env::prefixed(ENV_PREFIX));

        figment.extract()
    }

    /// Create a configuration from the environment variables and the indicated file.
    ///
    /// The file is prioritised. You can use the environment variables to overwrite certain file values.
    ///
    /// For more info about how the environment variables are read, see [from_env()](#from_env).
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env_and_file(path: &str) -> Result<Self, Error> {
        event!(
            Level::DEBUG,
            "Building configuration from environment and file {}",
            path
        );

        let file_figment = Figment::new()
            .merge(Toml::file_exact(path));

        if file_figment.extract_inner::<String>("client_secret").is_ok() {
            return Err(Error::from("Setting client secret is not allowed from a config file"));
        }

        let env_figment = Figment::new()
            .merge(Env::prefixed(ENV_PREFIX));


        file_figment.merge(env_figment).extract()
    }

    /// Checks if all the required fields are set in the configuration that the API server
    pub fn is_valid_api_config(&self) -> bool {
        let client_secret = self.client_secret.clone().is_some_and(|s| !s.is_empty());
        let session_key = self.session_key.clone().is_some_and(|s| s.len() >= 64);
        let client_id = self.client_id.clone().is_some();

        client_secret && session_key && client_id
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self::from_env().unwrap()
    }
}

#[cfg(test)]
mod tests {
    //! Tests are run with the working directory set to the work space, not the directory of the source file.

    use super::*;

    const TEST_CONFIG_EXPOSED: &str = r#"
            discord_token = "111"
            database_url = "sqlite://:memory:"
            redis_url = "redis://localhost"
            job_interval_min = 1
            shard_key = "c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"
            session_key = "1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw=="
            client_secret = "exposedsecret"
            client_id = 948517362313863198
            "#;

    const TEST_CONFIG: &str = r#"
            discord_token = "111"
            database_url = "sqlite://:memory:"
            redis_url = "redis://localhost"
            job_interval_min = 1
            shard_key = "c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"
            session_key = "1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw=="
            client_id = 948517362313863198
            "#;

    #[test]
    fn error_when_secret_in_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("config.toml", TEST_CONFIG_EXPOSED)?;

            DiscordConfig::from_env_and_file("config.toml").expect_err("This should have thrown an error");

            Ok(())
        })
    }

    #[test]
    fn can_deserialize_toml() {

        figment::Jail::expect_with(|jail| {
            jail.create_file("config.toml", TEST_CONFIG)?;

            let expected = DiscordConfig {
                discord_token: "111".into(),
                database_url: "sqlite://:memory:".into(),
                redis_url: "redis://localhost".into(),
                job_interval_min: 1,
                shard_key: uuid::uuid!("c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"),
                session_key: Some("1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw==".into()),
                client_secret: None,
                client_id: Some(NonZeroU64::new(948517362313863198).unwrap()),
            };

            let config = DiscordConfig::from_env_and_file("config.toml")?;
            assert_eq!(config, expected);

            Ok(())
        });
    }

    #[test]
    fn env_overwrites_file() {
        // Arrange
        figment::Jail::expect_with(|jail| {
            jail.create_file("config.toml", TEST_CONFIG)?;

            jail.set_env(format!("{}{}", ENV_PREFIX, "CLIENT_SECRET"), "supersecret");
            jail.set_env(format!("{}{}", ENV_PREFIX, "DISCORD_TOKEN"), r#""222""#);

            let expected = DiscordConfig {
                discord_token: "222".into(),
                database_url: "sqlite://:memory:".into(),
                redis_url: "redis://localhost".into(),
                job_interval_min: 1,
                shard_key: uuid::uuid!("c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"),
                session_key: Some("1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw==".into()),
                client_secret: Some("supersecret".into()),
                client_id: Some(NonZeroU64::new(948517362313863198).unwrap()),
            };

            let config = DiscordConfig::from_env_and_file("config.toml")?;
            assert_eq!(expected, config);

            Ok(())
        });
    }
}
