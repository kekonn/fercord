use anyhow::{anyhow, Result};
use chrono::prelude::*;
use chrono_english::*;
use chrono_tz::{Tz, TZ_VARIANTS};
use poise::serenity_prelude as serenity;
use tracing::*;

use crate::storage::kv::KVClient;
use crate::storage::model::guild_timezone::GuildTimezone;
use crate::ServerData;
use crate::storage::model::reminder::Reminder;
use crate::storage::db::Repository;

pub type Context<'a> = poise::Context<'a, ServerData, anyhow::Error>;

const FROM_NOW: &str = "from now";

/// Check if the bot is still alive
#[poise::command(slash_command)]
pub async fn zuigt_ge_nog(ctx: Context<'_>) -> Result<()> {
    let span = trace_span!("fercord.discord.zuigt_ge_nog");
    let _enter = span.enter();

    event!(Level::TRACE, "Received zuigt_ge_nog command");

    ctx.say(format!("Vacuuming at {:?}", std::time::SystemTime::now()))
        .await?;

    event!(Level::TRACE, "Replied successfully");
    Ok(())
}

/// Set the timezone for this server (used by time related commands).
#[poise::command(slash_command)]
pub async fn timezone(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_timezone"]
    #[description = "The IANA name of the timezone. Type the first 3 letters of the timezone to autocomplete."]
    timezone: String,
) -> Result<()> {
    let span = trace_span!(
        "fercord.discord.timezone",
        timezone = &timezone,
        guild_id = field::Empty
    );
    let _enter = span.enter();

    event!(parent: &span, Level::TRACE, "Received timezone command");

    ctx.defer_ephemeral().await?;

    if let Some(guild_id) = ctx.guild_id() {
        span.record("guild_id", tracing::field::debug(&guild_id));

        let guild_timezone = GuildTimezone {
            guild_id: guild_id.0,
            timezone: timezone.clone(),
        };

        debug!(?guild_timezone, "Setting timezone for guild");
        let kv_client = &ctx.data().kv_client;

        match kv_client.save_json(guild_timezone).await {
            Ok(_) => {
                event!(
                    parent: &span,
                    Level::DEBUG,
                    "Successfully set the timezone for this server"
                );
                ctx.say(format!("Set timezone {} for the server.", &timezone))
                    .await?;

                Ok(())
            }
            Err(e) => {
                warn!(?e, "Error setting the timezone for the server");

                Err(anyhow!(e))
            }
        }
    } else {
        event!(
            parent: &span,
            Level::WARN,
            "Could not determine the guild id"
        );

        Err(anyhow!("Could not determine the guild id"))
    }
}

/// Autocomplete renderer for the timezones list.
async fn autocomplete_timezone<'a>(
    _ctx: Context<'_>,
    partial: &str,
) -> impl Iterator<Item = String> {
    let timezones: Vec<String> = filter_timezones(partial).collect();
    match timezones.len() {
        1..=100 => timezones.into_iter(),
        101.. => timezones.chunks(100).next().unwrap().to_vec().into_iter(),
        _ => vec![String::from(
            "Please type the first 3 characters of your timezone to start a search",
        )]
        .into_iter(),
    }
}

fn filter_timezones(pattern: &str) -> impl Iterator<Item = String> + '_ {
    TZ_VARIANTS
        .iter()
        .filter_map(move |tz| tz.name().find(pattern).map(|_| tz.name()))
        .map(|tz| tz.to_string())
}

/// Create a reminder
///
/// * when: When to remind you
/// * what: What to remind you of
#[poise::command(slash_command)]
pub async fn reminder(ctx: Context<'_>, when: String, what: String) -> Result<()> {
    let span = trace_span!(
        "fercord.discord.reminder",
        guild_id = field::Empty,
        guild_timezone = field::Empty
    );
    let _enter = span.enter();
    event!(Level::TRACE, ?when, ?what, "Received reminder command");

    ctx.defer_ephemeral().await?;

    let kv_client = &ctx.data().kv_client;
    let guild_timezone = match ctx.guild_id() {
        Some(guild_id) => {
            span.record("guild_id", field::display(&guild_id));
            event!(Level::TRACE, "Retrieving timezone for guild");

            if let Ok(guild_timezone) = get_guild_timezone(kv_client, &guild_id).await {
                span.record("guild_timezone", field::debug(&guild_timezone));
                event!(
                    Level::DEBUG,
                    "Found specific timezone {:?} for guild {}",
                    guild_timezone,
                    guild_id
                );

                guild_timezone
            } else {
                Tz::UTC
            }
        }
        None => {
            event!(Level::DEBUG, "Could not find guild timezone");

            Tz::UTC
        }
    };

    if let Ok(parsed_datetime) = parse_human_time(&when, guild_timezone) {
        event!(
            Level::TRACE,
            ?parsed_datetime,
            "Parsed '{when}' into {parsed_datetime:#?}"
        );

        let reminder = Reminder {
            id: 0, // will be ignored on insert
            server: ctx.guild_id().unwrap().0,
            channel: ctx.channel_id().0,
            who: ctx.author().id.0,
            when: parsed_datetime.with_timezone(&Utc),
            what: what.clone()
        };

        let repo = Reminder::repository(&ctx.data().db_pool);
        let id = repo.insert(&reminder).await?;

        event!(Level::TRACE, "Saved event with id {}", id);

        ctx.say(format!(
            "Got it! I will remind you at {} about {}",
            parsed_datetime.format("%d/%m/%Y %H:%M"),
            what
        ))
        .await?;
    } else {
        ctx.say(format!("What the hell am I supposed to make of {when}?!"))
            .await?;
    }

    Ok(())
}

async fn get_guild_timezone(client: &KVClient, guild_id: &serenity::GuildId) -> Result<Tz> {
    let kv_identity = GuildTimezone {
        guild_id: guild_id.0,
        timezone: String::new(),
    };

    let guild_timezone = client.get_json(&kv_identity).await?;

    if guild_timezone.is_none() {
        return Ok(Tz::UTC);
    }

    guild_timezone.unwrap().timezone.parse::<Tz>().map_err(|e| anyhow!(e))
}

fn parse_human_time<T>(when: impl Into<String>, tz: T) -> Result<DateTime<T>>
where
    T: TimeZone,
{
    let span = trace_span!(
        "discord.parse_human_time",
        raw_input = field::Empty,
        clean_input = field::Empty,
        parsed_datetime = field::Empty
    );
    let _enter = span.enter();

    let raw_input: String = when.into();
    event!(Level::TRACE, "Parsing '{raw_input}' to a date/time");
    span.record("raw_input", field::debug(&raw_input));

    let cleaned_input = clean_input(raw_input);
    span.record("cleaned_input", field::debug(&cleaned_input));
    event!(Level::TRACE, ?cleaned_input, "Cleaned up raw input");

    if let Ok(parsed_datetime) = parse_date_string(&cleaned_input, Utc::now(), Dialect::Us) {
        event!(
            Level::TRACE,
            "Parsed '{cleaned_input}' into '{parsed_datetime:#?}'"
        );
        span.record("parsed_datetime", field::display(&parsed_datetime));

        Ok(parsed_datetime.with_timezone(&tz))
    } else {
        event!(
            Level::TRACE,
            "Failed to parse raw input to a useful DateTime"
        );

        Err(anyhow!("Failed to parse raw input to a useful DateTime"))
    }
}

#[tracing::instrument]
fn clean_input(natural_input: String) -> String {
    let mut pre_clean = natural_input.trim().to_lowercase();

    if pre_clean.starts_with("in") {
        pre_clean = pre_clean.drain(2..).collect();
    }

    if let Some(now_pos) = pre_clean.find(FROM_NOW) {
        let from_now_len = FROM_NOW.len();

        pre_clean.drain(now_pos..now_pos + from_now_len);
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
        let parsed = super::parse_human_time(input, Utc);

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
