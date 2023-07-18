use std::sync::Arc;

use poise::async_trait;
use tracing::{ event, Level, debug_span, field };

use crate::{ job::{ JobArgs, Job , JobResult }, storage::{ model::reminder::{*, self} }};


pub(crate) struct RemindersJob;

#[async_trait]
impl Job for RemindersJob {
    
    async fn run(&self, args: &Arc<JobArgs>) -> JobResult {
        let span = debug_span!("fercord.jobs.reminders", last_run_time = field::display(&args.last_run_time));
        let _enter = span.enter();
        
        let repo: ReminderRepo = Reminder::repository(&args.db_pool);
        let expired_reminders = repo.get_reminders_since(&args.last_run_time).await?;

        event!(Level::DEBUG, "Found {} reminders since {}", &expired_reminders.len(), &args.last_run_time);

        for reminder in expired_reminders {
            // get discord client and send reminders
            
            todo!()
        }

        Ok(())
    }
}