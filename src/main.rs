mod config;
mod discord;
mod storage;
mod timed_job;

use anyhow::{anyhow, Context};
use discord::*;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Activity;
use tracing::*;

use crate::storage::kv::KVClient;

pub struct ServerData {
    pub kv_client: crate::storage::kv::KVClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    event!(Level::DEBUG, "Reading configuration");
    // Load application config
    let config = config::DiscordConfig::from_env_and_file(".config/config.toml")?;

    // Client setup
    event!(Level::DEBUG, "Discord client setup");
    let token = config.discord_token.as_str();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![reminder(), zuigt_ge_nog(), timezone()],
            ..Default::default()
        })
        .token(token)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                ctx.reset_presence().await;
                ctx.set_activity(Activity::watching("all of you")).await;

                // TODO: Figure out how to start and stop background worker

                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .with_context(|| "Error creating Discord client")?;

                let kv_client = KVClient::new(&config)?;

                Ok(ServerData { kv_client })
            })
        });

    match framework.run_autosharded().await {
        Err(e) => Err(anyhow!(e)),
        Ok(_) => Ok(()),
    }
}
