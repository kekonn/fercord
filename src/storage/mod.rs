//! All things storage for the bot.

extern crate tracing;

/// key-value store for session data and the like
pub mod kv;

/// All the storage models
pub mod model;

/// Database access
pub mod db;
