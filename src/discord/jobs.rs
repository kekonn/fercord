use std::sync::Arc;

use poise::async_trait;
use tracing::{ event, Level, debug_span, field };
use poise::serenity_prelude as serenity;

use crate::{ job::{ JobArgs, Job , JobResult }, storage::model::reminder::{*}};


struct RemindersJob;

#[async_trait]
impl Job for RemindersJob {
    
    async fn run(&self, args: &Arc<JobArgs>) -> JobResult {
        let span = debug_span!("fercord.jobs.reminders", reminder_id = field::Empty);
        let _enter = span.enter();
        
        let repo: ReminderRepo = Reminder::repository(&args.db_pool);
        let expired_reminders = repo.get_reminders_since(&args.last_run_time).await?;

        event!(Level::DEBUG, "Found {} reminders since {}", &expired_reminders.len(), &args.last_run_time);

        let discord_client = &args.discord_client.http;

        for reminder in expired_reminders {
            // get discord client and send reminders
            span.record("reminder_id", field::display(reminder.id));

            let typing = discord_client.start_typing(reminder.channel)?;
            let user = discord_client.get_user(reminder.who).await?;
            
            let channel = serenity::ChannelId(reminder.channel);
            if let Err(error) = channel.send_message(&discord_client, |m| {
                m.content(format!("{} I was supposed to remind you of {}", serenity::Mention::from(user.id), reminder.what))
            }).await {
                event!(Level::ERROR, %error, "Error sending reminder {}", &reminder.id);
            }

            typing.stop();
        }

        Ok(())
    }
}

pub fn reminders() -> Box<dyn Job> {
    Box::new(RemindersJob {})
}