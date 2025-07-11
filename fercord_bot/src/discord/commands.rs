use std::num::NonZeroU8;

use anyhow::{anyhow, Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use chrono_tz::{Tz, TZ_VARIANTS};
use interim::{parse_date_string, Dialect};
use poise::serenity_prelude as serenity;
use rand::prelude::*;
use tracing::{debug, event, field, trace_span, warn, Level};

use crate::discord::Context;
use fercord_storage::prelude::{guild_timezone::GuildTimezone, *};

const FROM_NOW: &str = "from now";
const AT: &str = "at";

/// Register slash commands
#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<()> {
    let span = trace_span!("fercord.discord.register",);
    let _enter = span.enter();

    poise::builtins::register_application_commands_buttons(ctx)
        .await
        .context("Error registering slash commands")?;

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
        span.record("guild_id", field::debug(&guild_id));

        let guild_timezone = GuildTimezone {
            guild_id: guild_id.get(),
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

/// Roll dice
///
/// * count: The amount of dice to roll
/// * sides: the sides on the dice
#[poise::command(slash_command)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "How many dice do you want to roll?"]
    #[min = 1]
    #[max = 255]
    count: NonZeroU8,
    #[description = "How many sides does the dice have?"]
    #[min = 2]
    #[max = 255]
    sides: NonZeroU8,
) -> Result<()> {
    let span = trace_span!(
        "fercord.discord.roll",
        guild_id = field::Empty,
        die_command = "{count}d{sides}"
    );
    let _enter = span.enter();
    event!(Level::TRACE, "Received roll command for {count}d{sides}");
    let guild_id = ctx.guild_id().context("Error retrieving guild id")?;
    span.record("guild_id", field::debug(&guild_id));

    ctx.defer_ephemeral().await?;

    let mut rng = rand::rng();

    let mut rolls: Vec<usize> = Vec::with_capacity(count.get() as usize);
    rolls.resize(count.get().into(), Default::default());
    rolls.fill_with(move || rng.random_range(1..=(sides.get() as usize)));

    let roll_total = rolls.into_iter().sum::<usize>();
    event!(
        Level::TRACE,
        "Rolled {count}d{sides} for a total value of {roll_total}"
    );

    ctx.say(format!(
        "You rolled {count}d{sides} for a total value of {roll_total}"
    ))
    .await?;

    Ok(())
}

/// Create a reminder
///
/// * when: When to remind you
/// * what: What to remind you of
#[poise::command(slash_command)]
pub async fn reminder(
    ctx: Context<'_>,
    #[description = "When should I remind you?"] when: String,
    #[description = "What should I remind you of?"] what: String,
) -> Result<()> {
    let span = trace_span!(
        "fercord.discord.reminder",
        guild_id = field::Empty,
        guild_timezone = field::Empty
    );
    let _enter = span.enter();
    event!(Level::TRACE, ?when, ?what, "Received reminder command");
    let now = Utc::now();

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

    if let Ok(parsed_datetime) = parse_human_time(
        &when,
        guild_timezone,
        Some(now.with_timezone(&guild_timezone)),
    ) {
        event!(
            Level::TRACE,
            ?parsed_datetime,
            "Parsed '{when}' into {parsed_datetime:#?}"
        );

        let user_data = ctx.framework().user_data().await;
        let min_duration = user_data.config.job_interval_min as i64;
        let interval = parsed_datetime.signed_duration_since(now);
        if interval.num_minutes() < min_duration {
            return Err(anyhow!(
                "The minimum amount of time for a reminder is {} minute.",
                min_duration
            ));
        }

        let reminder = Reminder {
            id: 0, // will be ignored on insert
            server: ctx.guild_id().unwrap().into(),
            channel: ctx.channel_id().into(),
            who: ctx.author().id.into(),
            when: parsed_datetime.with_timezone(&Utc),
            what: what.clone(),
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
        guild_id: guild_id.get(),
        timezone: String::new(),
    };

    let guild_timezone = client.get_json(&kv_identity).await?;

    if guild_timezone.is_none() {
        return Ok(Tz::UTC);
    }

    guild_timezone
        .unwrap()
        .timezone
        .parse::<Tz>()
        .map_err(|e| anyhow!(e))
}

pub(crate) fn parse_human_time<Tz>(
    when: impl Into<String>,
    tz: Tz,
    now: Option<DateTime<Tz>>,
) -> Result<DateTime<Tz>>
where
    Tz: chrono::TimeZone,
{
    let span = trace_span!(
        "discord.parse_human_time",
        raw_input = field::Empty,
        clean_input = field::Empty,
        parsed_datetime = field::Empty,
        now = field::debug(&now),
    );
    let _enter = span.enter();

    let raw_input: String = when.into();
    event!(Level::TRACE, "Parsing '{raw_input}' to a date/time");
    span.record("raw_input", field::debug(&raw_input));

    let now = now.unwrap_or(Utc::now().with_timezone(&tz));

    let cleaned_input = clean_input(raw_input);
    span.record("cleaned_input", field::debug(&cleaned_input));
    event!(Level::TRACE, ?cleaned_input, "Cleaned up raw input");

    if let Ok(parsed_datetime) = parse_date_string(&cleaned_input, now.fixed_offset(), Dialect::Uk)
    {
        event!(
            Level::TRACE,
            "Parsed '{cleaned_input}' into '{parsed_datetime:#?}'"
        );
        span.record("parsed_datetime", field::debug(&parsed_datetime));

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
pub(crate) fn clean_input(natural_input: String) -> String {
    let mut pre_clean = natural_input.trim().to_lowercase();

    if pre_clean.starts_with("in") {
        pre_clean = pre_clean.drain(2..).collect();
    }

    if let Some(now_pos) = pre_clean.find(FROM_NOW) {
        let from_now_len = FROM_NOW.len();

        pre_clean.drain(now_pos..now_pos + from_now_len);
    }

    if let Some(at_pos) = pre_clean.find(AT) {
        let at_len = AT.len();

        pre_clean.drain(at_pos..at_pos + at_len);
    }

    pre_clean.trim().to_string()
}

/// Autocomplete renderer for the timezones list.
#[allow(clippy::unnecessary_to_owned)]
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
    let lower_pattern = pattern.to_lowercase();
    TZ_VARIANTS
        .iter()
        .filter_map(move |tz| {
            tz.name()
                .to_lowercase()
                .find(&lower_pattern)
                .map(|_| tz.name())
        })
        .map(|tz| tz.to_string())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn can_find_timezone() -> Result<()> {
        let mixed_timezones = vec!["eUrOpe/Brussel", "europe/brussels", "EUROPE/BRUSSELS"];
        let expected = String::from("Europe/Brussels");
        for search in mixed_timezones {
            let found: Vec<String> = filter_timezones(search).collect();

            if let Some(found_tz) = found.first() {
                assert_eq!(
                    expected, *found_tz,
                    "Found timezone does not match expected {}",
                    expected
                );
            } else {
                return Err(anyhow!("Could not find timezone matching {}", expected));
            }
        }

        Ok(())
    }
}
