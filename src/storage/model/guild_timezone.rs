use serde::{Serialize, Deserialize};
use chrono_tz::Tz;
use anyhow::{Result, anyhow};

use crate::storage::kv::{KVIdentity, Identifiable};

/// Contains the timezone set for a certain guild.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct GuildTimezone {
    pub guild_id: u64,
    pub timezone: String,
}

impl Identifiable for GuildTimezone {
    fn kv_key(&self) -> KVIdentity {
        format!("guild_timezone_{}", &self.guild_id)
    }
}

impl TryFrom<GuildTimezone> for chrono_tz::Tz {
    type Error = anyhow::Error;

    fn try_from(value: GuildTimezone) -> Result<Self, Self::Error> {
        let timezone: Tz = value.timezone.parse::<Tz>().map_err(|e| anyhow!(e))?;

        Ok(timezone)
    }
}