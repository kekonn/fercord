use anyhow::{Context, Result};
use poise::async_trait;
use sqlx::{any::AnyPoolOptions, AnyPool};
#[cfg(feature = "sqlite")]
use sqlx::{migrate::MigrateDatabase, Sqlite};
use tracing::{event, Level};

/// Create a database connection and run any pending migrations
#[cfg(not(feature = "sqlite"))]
pub async fn setup(url: &str) -> Result<AnyPool> {
    event!(Level::DEBUG, "Connecting to the database");
    let pool = AnyPoolOptions::new()
        .max_connections(2)
        .connect(url).await.with_context(|| "Error connecting to database")?;

    run_migrations(&pool).await?;

    Ok(pool)
}

#[cfg(feature = "sqlite")]
pub async fn setup(url: &str) -> Result<AnyPool> {
    event!(Level::DEBUG, "Checking if database exists");

    if !Sqlite::database_exists(url).await? {
        event!(Level::DEBUG, "Could not find database, creating a new one");
        Sqlite::create_database(url).await.with_context(|| format!("Error creating sqlite database {}", url))?;
    } else {
        event!(Level::DEBUG, "Databse {} already exists", url);
    }

    event!(Level::DEBUG, "Connecting to the database");
    let pool = AnyPoolOptions::new()
        .max_connections(2)
        .connect(url).await.with_context(|| "Error connecting to database")?;

    run_migrations(&pool).await?;

    Ok(pool)
}

#[cfg(feature = "sqlite")]
async fn run_migrations(pool: &AnyPool) -> Result<()> {
    let migrations = sqlx::migrate!("migrations/sqlite");

    event!(Level::DEBUG, "Running any pending migrations");
    migrations.run(pool).await.with_context(|| "Error applying migrations")?;

    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_migrations(pool: &AnyPool) -> Result<()> {
    let migrations = sqlx::migrate!("migrations/postgres");

    event!(Level::DEBUG, "Running any pending migrations");
    migrations.run(pool).await.with_context(|| "Error applying migrations")?;

    Ok(())
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

