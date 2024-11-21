use clap::{arg, command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to config.toml
    #[arg(short, long, value_hint = clap::ValueHint::FilePath, env, default_value(".config/config.toml"))]
    pub config: String,
    /// Current timezone
    #[arg(env("TZ"), default_value("Etc/Utc"))]
    pub timezone: String,
    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialOrd, PartialEq)]
pub enum Commands {
    /// Simply performs health checks. Only supported by the bot. Ignored by all the rest
    Healthcheck,
}
