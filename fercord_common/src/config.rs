//! Everything used to handle the application configuration.
//!
//! Create a new configuration as follows:
//! ```rust
//! use fercord_common::prelude::*;
//! let config = DiscordConfig::from_env().unwrap();
//! // or when you want to use a file and only overwrite from env
//! let config = DiscordConfig::from_env_and_file("../.config/config.toml").unwrap();
//! ```

use tracing::{event, Level};

pub use config::ConfigError;

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
    pub client_id: Option<String>,
    /// Discord OAuth client secret. For security reasons this can only be set from the environment.
    ///
    /// This is used by the API.
    pub client_secret: Option<String>,
}

const ENV_PREFIX: &str = "FERCORD";

impl DiscordConfig {
    /// Create a configuration just from environment variables.
    ///  
    /// This will read all variables prefixed with `FERCORD_` and try to serialize them into a `DiscordConfig`.
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env() -> Result<Self, ConfigError> {
        let builder =
            config::Config::builder().add_source(config::Environment::with_prefix(ENV_PREFIX));

        let config = builder.build()?;

        config.try_deserialize::<DiscordConfig>()
    }

    /// Create a configuration from the environment variables and the indicated file.
    ///
    /// The file is prioritised. You can use the environment variables to overwrite certain file values.
    ///
    /// For more info about how the environment variables are read, see [from_env()](#from_env).
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env_and_file(path: &str) -> Result<Self, ConfigError> {
        event!(
            Level::DEBUG,
            "Building configuration from environment and file {}",
            path
        );

        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix(ENV_PREFIX));

        let config = builder.build()?;

        config.try_deserialize::<DiscordConfig>()
    }

    /// Checks if all the required fields are set in the configuration that the API server
    pub fn is_valid_api_config(&self) -> bool {
        let client_secret = self.client_secret.clone().is_some_and(|s| s.len() > 0);
        let session_key = self.session_key.clone().is_some_and(|s| s.len() >= 64);
        let client_id = self.client_id.clone().is_some_and(|s| s.len() >= 18);

        client_secret && session_key && client_id
    }

    /// Create a configuration from the given file.
    ///
    /// Only here to test the file loading without environment influence.
    #[cfg(test)]
    fn from_file(path: &str) -> Result<Self, ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .set_override::<&str, Option<String>>("client_secret", None)?;

        let config = builder.build()?;

        config.try_deserialize::<DiscordConfig>()
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

    use std::env;

    use super::*;

    const TEST_CONFIG_PATH: &str = "../.testdata/basic_config.toml";

    #[test]
    fn can_deserialize_toml() {
        let expected = DiscordConfig {
            discord_token: "111".into(),
            database_url: "sqlite://:memory:".into(),
            redis_url: "redis://localhost".into(),
            job_interval_min: 1,
            shard_key: uuid::uuid!("c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"),
            session_key: Some("1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw==".into()),
            client_secret: None,
            client_id: Some("948517362313863198".into())
        };

        let config = DiscordConfig::from_file(TEST_CONFIG_PATH).unwrap();

        assert_eq!(expected, config);
    }

    #[test]
    fn env_overwrites_file() {
        // Arrange
        env_setup();

        let expected = DiscordConfig {
            discord_token: "222".into(),
            database_url: "sqlite://:memory:".into(),
            redis_url: "redis://localhost".into(),
            job_interval_min: 1,
            shard_key: uuid::uuid!("c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"),
            session_key: Some("1hYw2n0+t8SDo+gqy+Q3x2SJ4u/Y6e6QPrMHExaQTHETOD8tlUsR2Cq66H0a2QuGBK7L1TIDhAupc3rHCbiehw==".into()),
            client_secret: Some("supersecret".into()),
            client_id: Some("948517362313863198".into())
        };

        // Act
        let config = DiscordConfig::from_env_and_file(TEST_CONFIG_PATH).unwrap();

        // Assert
        assert_eq!(expected, config);

        env_teardown();
    }

    fn env_setup() {
        env::set_var(format!("{}_DISCORD_TOKEN", ENV_PREFIX), "222");
        env::set_var(format!("{}_CLIENT_SECRET", ENV_PREFIX), "supersecret");
    }

    fn env_teardown() {
        env::remove_var(format!("{}_DISCORD_TOKEN", ENV_PREFIX));
        env::remove_var(format!("{}_CLIENT_SECRET", ENV_PREFIX));
    }
}
