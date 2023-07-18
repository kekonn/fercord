use crate::ServerData;

pub type Context<'a> = poise::Context<'a, ServerData, anyhow::Error>;

pub mod commands;

#[cfg(test)]
mod tests;
