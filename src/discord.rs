use serenity::framework::standard::macros::group;
use serenity::prelude::*;
use serenity::async_trait;

#[group]
pub struct General;

pub struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {

}