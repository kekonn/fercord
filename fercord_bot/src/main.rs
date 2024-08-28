use anyhow::{anyhow, Context};
use clap::Parser;
use llm::models::Llama;
use poise::serenity_prelude::{self as serenity, ActivityData};
use std::path::Path;
use tracing::*;

use fercord_storage::config::DiscordConfig;
use fercord_storage::db;
use fercord_storage::prelude::{AnyPool, KVClient};

use crate::cli::Commands;
use crate::discord::commands::{reminder, timezone};
use crate::healthchecks::perform_healthchecks;
use crate::job::{job_scheduler, Job};

mod job;
mod cli;
mod healthchecks;
mod discord;

pub struct ServerData {
    pub kv_client: KVClient,
    pub db_pool: AnyPool,
    pub config: DiscordConfig,
    pub model: Llama,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = cli::Args::parse();
    let config_file_path = args.config;

    event!(Level::DEBUG, %config_file_path, "Reading configuration");
    // Load application config
    let config = DiscordConfig::from_env_and_file(&config_file_path)?;

    if let Some(Commands::Healthcheck) = args.command {
        let checks_output = perform_healthchecks(&config).await?;
        println!("{}", checks_output);
        return Ok(());
    }

    // Db Setup
    event!(Level::DEBUG, "Database setup");

    let db_pool = db
        ::setup(config.database_url.as_ref()).await
        .with_context(|| "Error setting up database connection")?;

    // KV Setup
    event!(Level::DEBUG, "Connecting to KV Store");
    let kv_client = KVClient::new(&config).with_context(|| "Error building redis client")?;
    
    // LLM Setup
    event!(Level::DEBUG, "Setting up local LLM");
    let model_path = Path::new(&config.llm_path);
    if !model_path.exists() {
        return Err(anyhow!("Could not load LLM model at path '{}'", model_path.to_string_lossy()));
    }
    let model = Llama::load(model_path, Default::default(), llm::load_progress_callback_stdout).with_context(|| format!("Error loading LLM model at path: {}", &config.llm_path))?;

    // Discord setup
    event!(Level::DEBUG, "Discord client setup");

    let discord_config = config.clone();
    let framework = poise::Framework
        ::builder()
        .options(poise::FrameworkOptions {
            commands: vec![reminder(), timezone()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            ctx.reset_presence();
            ctx.set_activity(ActivityData::watching("all of you").into());

            Box::pin(async move {
                poise::builtins
                    ::register_globally(ctx, &framework.options().commands).await
                    .with_context(|| "Error creating Discord client")?;

                Ok(ServerData { kv_client, db_pool, config: discord_config, model })
            })
        })
        .build();

    // Set up background scheduling
    event!(Level::INFO, "Setting up background jobs");
    let shard_key = uuid::Uuid::new_v4();
    info!(%shard_key);

    let jobs: Vec<Box<dyn Job>> = vec![
        discord::jobs::reminders(),
        discord::jobs::reminders_cleanup()
    ];

    let token = config.discord_token.as_str();
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;
    let mut discord_client = serenity::ClientBuilder::new(token, intents).framework(framework).await?;
    let http_client = serenity::HttpBuilder::new(token).build();

    let (discord_result, scheduler_result) = tokio::join!(
        discord_client.start_autosharded(),
        job_scheduler(&config, &jobs, &shard_key, &http_client)
    );

    if let Err(scheduler_err) = scheduler_result {
        event!(Level::ERROR, "{:?}", &scheduler_err);
    }

    if let Err(discord_error) = discord_result {
        event!(Level::ERROR, "{:?}", &discord_error);
    }

    Ok(())
}
