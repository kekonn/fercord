use chrono::Utc;
use tokio::time;
use tokio::time::MissedTickBehavior;
use tracing::{event, field, trace_span, Level};

use crate::discord::Context;

async fn start_worker<'a>(ctx: &Context<'a>) {
    let span = trace_span!("timed_job.worker", current_tick = field::Empty);
    let _enter = span.enter();

    let mut interval = time::interval(time::Duration::from_secs(60));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // TODO: Set up a channel to receive stop commands
    loop {
        interval.tick().await;
        span.record("current_tick", field::debug(Utc::now()));
        event!(Level::TRACE, "Tick expired. Waking up");

        todo!()
    }
}
