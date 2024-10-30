use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use poise::async_trait;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CacheHttp;
use serde::{Deserialize, Serialize};
use tracing::{debug_span, event, field, info, Level};

use fercord_storage::prelude::*;
use fercord_common::prelude::*;

//pub type Job = Box<dyn Fn(&Arc<JobArgs>) -> JobResult>;
pub(crate) type JobResult = anyhow::Result<()>;

#[async_trait]
pub trait Job {

    async fn run(&self, args: &JobArgs) -> JobResult;
}

pub struct JobArgs<'j> {
    pub kv_client: Arc<KVClient>,
    pub db_pool: Arc<AnyPool>,
    pub last_run_time: DateTime<Utc>,
    pub discord_client: Arc<dyn serenity::CacheHttp + 'j>,
    pub discord_config: DiscordConfig,
}

impl<'j> JobArgs<'j> {
    /// Create a new JobArgs struct from a `KVClient` and an sqlx Postgres pool.
    fn new(kv_client: &Arc<KVClient>, db_pool: &Arc<AnyPool>, last_run_time: DateTime<Utc>, discord_client: &Arc<impl serenity::CacheHttp + 'j>, discord_config: DiscordConfig) -> Self {
        Self { kv_client: kv_client.clone(), db_pool: db_pool.clone(), last_run_time, discord_client: discord_client.clone(), discord_config }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct JobState {
    pub last_run: chrono::DateTime<Utc>,
    job_shard_key: uuid::Uuid,
}

impl JobState {
    pub fn for_identity(shard_id: &uuid::Uuid) -> Self {
        Self { last_run: chrono::DateTime::<Utc>::default(), job_shard_key: *shard_id }
    }

    pub fn new(shard_id: &uuid::Uuid, time: DateTime<Utc>) -> Self {
        Self { last_run: time, job_shard_key: *shard_id }
    }
}

impl Identifiable for JobState {
    fn kv_key(&self) -> KVIdentity {
        format!("jobstate_{}", self.job_shard_key)
    }
}

pub(crate) async fn job_scheduler<'j>(
    app_config: &DiscordConfig,
    jobs: &Vec<Box<dyn Job>>,
    shard_key: &uuid::Uuid,
    discord_client: impl CacheHttp,
) -> Result<()> {
    let span = debug_span!("fercord.jobs.scheduler", shard_key = field::display(&shard_key));
    let _enter = span.enter();

    if jobs.is_empty() {
        info!("Job queue is empty. Skipping...");
        return Ok(());
    }

    event!(Level::DEBUG, "Setting up job scheduler db pool");
    let db_pool = {
        let pool = db
        ::setup(app_config.database_url.as_ref()).await
        .with_context(|| "Error setting up database connection")?;

        Arc::new(pool)
    };

    event!(Level::DEBUG, "Setting up job scheduler KV client");
    let kv_client = Arc::new(KVClient::new(app_config).with_context(|| "Error setting up KV client")?);

    let interval_dur = std::time::Duration::from_secs((app_config.job_interval_min * 60) as u64);
    let mut job_interval = tokio::time::interval_at(tokio::time::Instant::now(), interval_dur);
    job_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let client_arc = Arc::new(discord_client);

    loop {
         // get last run time and compare to interval, sleep for difference or run immediately
        event!(Level::TRACE, "Retrieving last run state");
        let last_job_state = get_last_runtime(shard_key, &kv_client).await?;

        let last_time_ran = last_job_state.map_or(Utc::now(), |s| s.last_run);

        let since_last_run = Utc::now() - last_time_ran;
        event!(Level::INFO, "Time since last run: {:?} s", &since_last_run.num_seconds());

        let mut failed_jobs = 0;
        let mut completed_jobs = 0;

        let job_args = Arc::new(JobArgs::new(&kv_client, &db_pool, last_time_ran, &client_arc, app_config.clone()));

        for job in jobs {

            
            if let Err(e) = job.run(&job_args).await {
                event!(Level::ERROR, "Encountered an error during a background job: {:?}", e);
                failed_jobs += 1;
            } else {
                completed_jobs += 1;
            }
        }
    
        info!("Attempted all jobs in this run. Completed: {} - Failed: {}", &completed_jobs, &failed_jobs);
    
        save_job_state(shard_key, &kv_client).await?;

        info!("Sleeping until next interval");
        _ = job_interval.tick().await;
    }
}

/// Save the job state for the given shard key and using the given KVClient.
async fn save_job_state(shard_key: &uuid::Uuid, kv_client: &Arc<KVClient>) -> Result<()> {
    let state = JobState::new(shard_key, Utc::now());
    event!(Level::DEBUG, "Saving completed run at {}", field::display(&state.last_run));

    kv_client.save_json(state).await
}


/// Retrieve the last known job state from the KV store (using `kv_client`).
///
/// ## Parameters
/// * `job_shard_key`: A `uuid::Uuid` that identifies this job runner.
/// * `kv_client`: The `KVClient` used for the connection to the kv server.
async fn get_last_runtime(
    job_shard_key: &uuid::Uuid,
    kv_client: &Arc<KVClient>
) -> Result<Option<JobState>> {
    let state_ident = JobState::for_identity(job_shard_key);
    let state_json = kv_client
        .get_json::<JobState>(&state_ident).await
        .with_context(|| format!("Error getting job state for shard {}", job_shard_key))?;

    Ok(state_json)
}
