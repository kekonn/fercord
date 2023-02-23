mod config;
mod discord;
mod storage;

use anyhow::{ Context, anyhow };
use poise::serenity_prelude as serenity;

use tracing::*;
use discord::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    event!(Level::DEBUG, "Reading configuration");
    // Load application config
    let config = config::DiscordConfig::from_env_and_file(".config/config.toml")?;

    // Database setup
    event!(Level::DEBUG, "Setting up database");
    let _db = storage::db::create_connection(&config).await?;

    // Client setup
    event!(Level::DEBUG, "Discord client setup");
    let token = config.discord_token.as_str();
    let framework = poise::Framework
        ::builder()
        .options(poise::FrameworkOptions {
            commands: vec![reminder(),zuigt_ge_nog()],
            ..Default::default()
        })
        .token(token)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT
        )
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins
                    ::register_globally(ctx, &framework.options().commands).await
                    .with_context(|| "Error creating Discord client")?;

                Ok(config.clone())
            })
        });

    match framework.run_autosharded().await {
        Err(e) => Err(anyhow!(e)),
        Ok(_) => Ok(()),
    }
}