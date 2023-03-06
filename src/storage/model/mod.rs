//! All models for the bot

/// Reminder model
pub mod reminder;

/// Store guild timezones as setting data
pub mod guild_timezone;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct DiscordWhere {
    pub server: u64,
    pub channel: u64,
}

impl DiscordWhere {
    pub fn new(server: u64, channel: u64) -> Self {
        Self { server, channel }
    }
}
