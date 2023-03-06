use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Reminder {
    pub who: u64,
    pub when: DateTime<Utc>,
    pub what: String,
}