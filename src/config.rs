//! Everything used to handle the application configuration.
//!
//! Create a new configuration as follows:
//! ```rust
//! let config = DiscordConfig::from_env().unwrap();
//! // or when you want to use a file and only overwrite from env
//! let config = DiscordConfig::from_env_and_file("./config.toml").unwrap();
//! ```

use anyhow::{Context, Result};

/// The application configuration.
///
/// You can use [from_env()](#from_env) or [from_env_and_file(path: &str)](#from_end_and_path) to create a configuration.
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
}

const ENV_PREFIX: &str = "FERCORD";

impl DiscordConfig {
    /// Create a configuration just from environment variables.
    ///  
    /// This will read all variables prefixed with `FERIS_` and try to serialize them into a `DiscordConfig`.
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env() -> Result<Self> {
        let builder =
            config::Config::builder().add_source(config::Environment::with_prefix(ENV_PREFIX));

        let config = builder
            .build()
            .with_context(|| "Error building Fercord configuration")?;

        config
            .try_deserialize::<DiscordConfig>()
            .with_context(|| "Error deserializing configuration")
    }

    /// Create a configuration from the environment variables and the indicated file.
    ///
    /// The file is prioritised and you can use the environment variables to overwrite certain file values.
    ///
    /// For more info about how the environment variables are read, see [from_env()](#from_env).
    #[allow(dead_code)]
    #[tracing::instrument]
    pub fn from_env_and_file(path: &str) -> Result<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix(ENV_PREFIX));

        let config = builder
            .build()
            .with_context(|| "Error building Fercord configuration")?;

        config
            .try_deserialize::<DiscordConfig>()
            .with_context(|| "Error deserializing configuration")
    }

    /// Create a configuration from the given file.
    ///
    /// Only here to test the file loading without environment influence.
    #[cfg(test)]
    fn from_file(path: &str) -> Result<Self> {
        let builder = config::Config::builder().add_source(config::File::with_name(path));

        let config = builder
            .build()
            .with_context(|| "Error building Fercord configuration")?;

        config
            .try_deserialize::<DiscordConfig>()
            .with_context(|| "Error deserializing configuration")
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self::from_env()
            .with_context(|| "Error creating config from environment")
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    //! Tests are run with the working directory set to the work space, not the directory of the source file.

    use super::*;
    use std::env;

    #[test]
    fn can_deserialize_toml() {
        let expected = DiscordConfig {
            discord_token: "111".into(),
            database_url: "sqlite://:memory:".into(),
            redis_url: "redis://localhost".into()
        };

        let config = DiscordConfig::from_file(".testdata/basic_config.toml").unwrap();

        assert_eq!(expected, config);
    }

    #[test]
    fn env_overwrites_file() {
        // Arrange
        env_setup();

        let expected = DiscordConfig {
            discord_token: "222".into(),
            database_url: "sqlite://:memory:".into(),
            redis_url: "redis://localhost".into()
        };

        // Act
        let config = DiscordConfig::from_env_and_file(".testdata/basic_config.toml").unwrap();

        // Assert
        assert_eq!(expected, config);

        env_teardown();
    }

    fn env_setup() {
        env::set_var(format!("{}_DISCORD_TOKEN", ENV_PREFIX), "222");
    }

    fn env_teardown() {
        env::remove_var(format!("{}_DISCORD_TOKEN", ENV_PREFIX));
    }
}
