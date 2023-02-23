/// All things database.

use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};

use crate::config::DiscordConfig;

/// Create a database connection from a `DiscordConfig`.
#[tracing::instrument]
pub async fn create_connection(config: &DiscordConfig) -> Result<DatabaseConnection> {
    let connection = Database::connect(&config.database_url).await?;

    Ok(connection)
}