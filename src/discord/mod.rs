use crate::ServerData;

pub type Context<'a> = poise::Context<'a, ServerData, anyhow::Error>;

pub mod commands;
pub mod jobs;

#[cfg(test)]
mod tests;
