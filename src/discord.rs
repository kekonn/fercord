
use anyhow::{Result, anyhow};
use chrono::{prelude::*};
use chrono_english::*;
use poise::serenity_prelude as serenity;
use tracing::*;

use crate::config::DiscordConfig;

type Context<'a> = poise::Context<'a, DiscordConfig, anyhow::Error>;

const FROM_NOW: &str = "from now";

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
pub async fn reminder(ctx: Context<'_>, when: String, what: String) -> Result<()> {
    let span = trace_span!("fercord.discord.reminder");
    let _enter = span.enter();
    event!(
        parent: &span,
        Level::TRACE,
        ?when,
        ?what,
        "Received reminder command"
    );

    ctx.defer_ephemeral().await?;

    if let Ok(parsed_datetime) = span.in_scope(|| parse_human_time(&when)) {
        event!(
            parent: &span,
            Level::TRACE,
            ?parsed_datetime,
            "Parsed '{when}' into {parsed_datetime:#?}"
        );

        ctx.say(format!("Got it! I will remind you at {parsed_datetime}"))
            .await?;
    } else {
        ctx.say(format!("What the hell am I supposed to make of {when}?!"))
            .await?;
    }

    Ok(())
}

fn parse_human_time(when: impl Into<String>) -> Result<DateTime<Utc>> {
    let span = Span::current();
    let raw_input:String = when.into();
    event!(Level::TRACE, "Parsing '{raw_input}' to a date/time"); 
    span.record("raw_input", tracing::field::debug(&raw_input));

    let cleaned_input = clean_input(raw_input);
    span.record("cleaned_input", tracing::field::debug(&cleaned_input));
    event!(Level::TRACE, ?cleaned_input, "Cleaned up raw input");

    if let Ok(parsed_datetime) = parse_date_string(&cleaned_input, Utc::now(), Dialect::Us) {
        event!(Level::TRACE, "Parsed '{cleaned_input}' into '{parsed_datetime:#?}'");
        span.record("parsed_datetime", tracing::field::display(&parsed_datetime));

        Ok(parsed_datetime)
    } else {
        event!(Level::TRACE, "Failed to parse raw input to a usefull DateTime");

        Err(anyhow!("Failed to parse raw input to a usefull DateTime"))
    }
}

fn clean_input(natural_input: String) -> String {
    let mut pre_clean = natural_input.trim().to_lowercase();

    if pre_clean.starts_with("in") {
        pre_clean = pre_clean.drain(2..).collect();
    }

    if let Some(now_pos) = pre_clean.find(FROM_NOW) {
        let from_now_len = FROM_NOW.len();

        pre_clean.drain(now_pos..now_pos+from_now_len);
    }

    pre_clean.trim().to_string()
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use tracing_test::traced_test;

    /// `"5 minutes"`
    const EXPECTED: &str = "5 minutes";

    #[traced_test]
    #[test]
    fn parsed_moment_is_future() {
        // Arrange
        let input = "in 5 minutes";
        let now = Utc::now();

        // Act
        let parsed = super::parse_human_time(input);

        // Assert
        debug_assert!(parsed.is_ok(), "We did not get a successful parse");
        let unwrapped = parsed.unwrap();
        assert!(
            unwrapped > now,
            "Parsed time appears to be in the past: unwrapped={unwrapped} now={now}"
        );

        let difference = unwrapped - now;
        assert_eq!(difference.num_minutes(), 5);
    }

    #[test]
    fn clean_input_cleans_in_syntax() {
        let input = "in 5 minutes";

        let result = super::clean_input(input.into());

        assert_eq!(EXPECTED, result);
    }

    #[test]
    fn clean_input_cleans_from_syntax() {
        let input = "5 minutes from now";

        let result = super::clean_input(input.into());

        assert_eq!(EXPECTED, result);
    }

    #[test]
    fn clean_input_cleans_combined_syntax() {
        let input = "in 5 minutes from now";

        let result = super::clean_input(input.into());

        assert_eq!(EXPECTED, result);
    }
}
