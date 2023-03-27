use poise::async_trait;
use sqlx::{Pool, Postgres, Database};
use tracing::{event, instrument, Level};
use anyhow::{Result, Context};
use sqlx::postgres::PgPoolOptions;

/// Create a database connection and run any pending migrations
#[instrument]
pub async fn setup(url: &str) -> Result<Pool<Postgres>> {
    event!(Level::DEBUG, "Connecting to the database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url).await.with_context(|| "Error connecting to database")?;

    let migrations = sqlx::migrate!();

    event!(Level::DEBUG, "Running any pending migrations");
    migrations.run(&pool).await.with_context(|| "Error applying migrations")?;

    Ok(pool)
}

pub struct Repo<'r, C>
    where C: Database
{
    pub(crate) pool: &'r Pool<C>,
}

#[async_trait]
/// Basic repository interface
pub trait Repository<E, I> {
    async fn insert(&self, entity: &E) -> Result<I>;
    async fn delete(&self, entity: E) -> Result<()>;
}