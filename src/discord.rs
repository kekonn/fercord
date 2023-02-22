use std::{ops::Mul, fmt::Display};

use poise::serenity_prelude as serenity;
use anyhow::Result;
use tracing::*;
use date_time_parser::{DateParser, TimeParser};
use chrono::{prelude::*, Duration};

use crate::config::DiscordConfig;

type Context<'a> = poise::Context<'a, DiscordConfig, anyhow::Error>;

#[derive(Debug)]
struct FormattableDuration {
    duration: Duration
}

/// Check if the bot is still alive
#[poise::command(slash_command)]
pub async fn zuigt_ge_nog(ctx: Context<'_>) -> Result<()> {
    let span = trace_span!("fercord.discord.zuigt_ge_nog");
    let _enter = span.enter();

    event!(parent: &span, Level::TRACE, "Received zuigt_ge_nog command");

    ctx.say("Beter dan uw ma!").await?;

    event!(parent: &span, Level::TRACE, "Replied successfully");
    Ok(())
}

/// Create a reminder
/// 
/// * when: When to remind you
/// * what: What to remind you of
#[poise::command(slash_command)]
pub async fn reminder(
    ctx: Context<'_>,
    when: String,
    what: String
) -> Result<()> {
    let span = trace_span!("fercord.discord.reminder");
    let _enter = span.enter();
    event!(parent: &span, Level::TRACE, ?when, ?what, "Received reminder command");
    
    ctx.defer_ephemeral().await?;

    let parsed_time = TimeParser::parse_relative(&when);
    let parsed_date = DateParser::parse(&when);

    if let Some(parsed_datetime) = span.in_scope(|| merge_parsed_time_data(parsed_date, parsed_time)) {
        event!(parent: &span, Level::TRACE, ?parsed_datetime, "Parsed '{when}' into {parsed_datetime:#?}");

        let duration = Utc::now().naive_utc().signed_duration_since(parsed_datetime);
        let formatted_duration: FormattableDuration = duration.into();
        
        ctx.say(format!("Got it! I will remind you in {formatted_duration}")).await?;
    } else {
        ctx.say(format!("What the hell am I supposed to make of {when}?!")).await?;
    }

    Ok(())
}


fn merge_parsed_time_data(date: Option<NaiveDate>, time: Option<NaiveTime>) -> Option<NaiveDateTime> {
    event!(Level::TRACE, "Merging {date:?} and {time:?}");

    match (date, time) {
        (Some(parsed_date), Some(parsed_time)) => Some(NaiveDateTime::new(parsed_date, parsed_time)),
        (None, Some(parsed_time)) => Some(NaiveDateTime::new(Utc::now().date_naive(), parsed_time)),
        (Some(parsed_date), None) => Some(NaiveDateTime::new(parsed_date, Utc::now().naive_utc().time())),
        (None, None) => None
    }
}

impl Display for FormattableDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = &self.duration;

        let (days, hours, minutes) = (duration.num_days(), duration.num_hours(), duration.num_minutes());
        
        match (days > 0, hours > 0, minutes > 0) {
            (true, true, true) => write!(f, "{days} day(s), {hours} hour(s) and {minutes} minute(s)"),
            (false, true, true) => write!(f, "{hours} hour(s) and {minutes} minute(s)"),
            (false, false, true) => write!(f, "{minutes} minute(s)"),
            _ => write!(f, "now")
        }
    }
}

impl From<Duration> for FormattableDuration {
    fn from(val: Duration) -> Self {
        FormattableDuration { duration: val }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}