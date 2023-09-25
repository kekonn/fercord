mod config;
mod discord;
mod storage;
mod job;

use anyhow::Context;
use discord::commands::{timezone, zuigt_ge_nog, reminder};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Activity;
use sqlx::AnyPool;
use tracing::*;

use crate::{storage::{db, kv::KVClient}, job::{Job, job_scheduler}};
use crate::config::DiscordConfig;

pub struct ServerData {
    pub kv_client: KVClient,
    pub db_pool: AnyPool,
    pub config: DiscordConfig
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_file_path = {
        if let Ok(config_path) = std::env::var("CONFIG") {
            config_path
        } else {
            ".config/config.toml".to_owned()
        }
    };

    event!(Level::DEBUG, %config_file_path, "Reading configuration");
    // Load application config
    let config = config::DiscordConfig::from_env_and_file(&config_file_path)?;

    // Db Setup
    event!(Level::DEBUG, "Database setup");
    
    let db_pool = db
        ::setup(config.database_url.as_ref()).await
        .with_context(|| "Error setting up database connection")?;

    // KV Setup
    event!(Level::DEBUG, "Connecting to KV Store");
    let kv_client = KVClient::new(&config).with_context(|| "Error building redis client")?;

    // Discord setup
    event!(Level::DEBUG, "Discord client setup");
    let token = config.discord_token.as_str();

    let discord_config = config.clone();
    let framework = poise::Framework
        ::builder()
        .options(poise::FrameworkOptions {
            commands: vec![reminder(), zuigt_ge_nog(), timezone()],
            ..Default::default()
        })
        .token(token)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT
        )
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                ctx.reset_presence().await;
                ctx.set_activity(Activity::watching("all of you")).await;

                poise::builtins
                    ::register_globally(ctx, &framework.options().commands).await
                    .with_context(|| "Error creating Discord client")?;

                Ok(ServerData { kv_client, db_pool, config: discord_config })
            })
        })
        .build().await?;

    // Set up background scheduling
    event!(Level::INFO, "Setting up background jobs");
    let shard_key = uuid::Uuid::new_v4();
    info!(%shard_key);

    let jobs: Vec<Box<dyn Job>> = vec![discord::jobs::reminders()];
    let discord_client = framework.client().cache_and_http.clone();

    let (discord_result, scheduler_result) = tokio::join!(
        framework.start_autosharded(), 
        job_scheduler(&config, &jobs, &shard_key, &discord_client)
    );

    if let Err(scheduler_err) = scheduler_result {
        event!(Level::ERROR, "{:?}", &scheduler_err);
    }

    if let Err(discord_error) = discord_result {
        event!(Level::ERROR, "{:?}", &discord_error);
    }

    Ok(())    
}