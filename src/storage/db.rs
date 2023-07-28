use poise::async_trait;
use sqlx::{AnyPool, any::AnyPoolOptions};
use tracing::{event, Level};
use anyhow::{Result, Context};

/// Create a database connection and run any pending migrations
pub async fn setup(url: &str) -> Result<AnyPool> {
    event!(Level::DEBUG, "Connecting to the database");
    let pool = AnyPoolOptions::new()
        .max_connections(2)
        .connect(url).await.with_context(|| "Error connecting to database")?;

    let migrations = sqlx::migrate!();

    event!(Level::DEBUG, "Running any pending migrations");
    migrations.run(&pool).await.with_context(|| "Error applying migrations")?;

    Ok(pool)
}

pub struct Repo<'r>
{
    pub(crate) pool: &'r AnyPool,
}

#[async_trait]
/// Basic repository interface
pub trait Repository<E, I> 
    where I: Sized
{
    async fn insert(&self, entity: &E) -> Result<I>;
    async fn delete(&self, entity: E) -> Result<()>;
    async fn get(&self, id: I) -> Result<Option<E>>;
}

