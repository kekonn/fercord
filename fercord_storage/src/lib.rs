//! All things storage for the bot.

extern crate tracing;

/// key-value store for session data and the like
pub mod kv;

/// All the storage models
pub mod model;

/// Database access
pub mod db;

/// fercord_storage prelude
pub mod prelude {
    pub use sqlx_oldapi::any::AnyPool;

    pub use crate::db;
    pub use crate::kv::{KVClient, Identifiable, KVIdentity};
    pub use crate::model as model;
}