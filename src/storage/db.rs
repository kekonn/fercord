use sqlx::{Pool, Postgres};
use tracing::{Event, event, instrument, Level};
use anyhow::{Result, Context};
use sqlx::postgres::PgPoolOptions;

/// Create a database connection and run any pending migrations
#[instrument]
pub async fn setup(url: &str) -> Result<Pool<Postgres>> {
    event!(Level::DEBUG, "Connecting to the database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url).await.with_context(|| "Error connecting to database")?;

    event!(Level::DEBUG, "Running any pending migrations");
    let migrations = sqlx::migrate!();

    sqlx::migrate!().run(&pool).await.with_context(|| "Error applying migrations")?;

    Ok(pool)
}

