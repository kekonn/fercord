use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Reminder {
    pub who: u64,
    pub when: DateTime<Utc>,
    pub what: String,
    pub server: u64,
    pub channel: u64,
}
