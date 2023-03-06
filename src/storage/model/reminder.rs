use super::DiscordWhere;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Reminder {
    pub who: u64,
    pub when: DateTime<Utc>,
    pub what: String,
    pub here: DiscordWhere,
}
