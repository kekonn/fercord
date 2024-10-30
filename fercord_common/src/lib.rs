/**
Fercord_common
*/
pub mod cli;
pub mod config;

/// Fercord common prelude
pub mod prelude {
    pub use clap::Parser;

    pub use crate::cli::Args;
    pub use crate::cli::Commands;
    pub use crate::config::DiscordConfig;
}
