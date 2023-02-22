mod config;
mod discord;

use anyhow::Context;
use serenity::framework::StandardFramework;
use serenity::prelude::*;

use discord::*;
use tracing::*;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let current_span = subscriber.current_span();

    tracing::subscriber::set_global_default(subscriber)?;

    if let Some(span_metadata) = current_span.metadata() {
        println!("{:?}", span_metadata);
    }
    

    debug!("Reading configuration");
    // Load application config
    let config = config::DiscordConfig::from_env_and_file(".config/config.toml")?;

    // Client setup
    let token = config.discord_token.as_str();
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new()
        .configure(|c| c)
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(token, intents)
        .event_handler(DiscordHandler)
        .framework(framework)
        .await
        .with_context(|| "Error creating discord client")
        .unwrap();
    
    client.start().await.with_context(|| "Error starting Discord client")?;

    Ok(())
}
