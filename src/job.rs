use std::sync::Arc;

use anyhow::{ Result, Context };
use chrono::{ Utc, DateTime };
use poise::async_trait;
use poise::serenity_prelude as serenity;
use serde::{ Deserialize, Serialize };
use sqlx::Pool;
use tracing::{ span, info, event, field, debug_span, Level };

use crate::config::DiscordConfig;
use crate::storage::kv::*;
use crate::storage::db;

//pub type Job = Box<dyn Fn(&Arc<JobArgs>) -> JobResult>;
pub(crate) type JobResult = anyhow::Result<()>;

#[async_trait]
pub trait Job {

    async fn run(&self, args: &Arc<JobArgs>) -> JobResult;
}

pub struct JobArgs {
    pub kv_client: Arc<KVClient>,
    pub db_pool: Arc<Pool<sqlx::Postgres>>,
    pub last_run_time: DateTime<Utc>,
    pub discord_client: Arc<serenity::CacheAndHttp>,
}

impl JobArgs {
    /// Create a new JobArgs struct from a `KVClient` and an sqlx Postgres pool.
    fn new(kv_client: &Arc<KVClient>, db_pool: &Arc<Pool<sqlx::Postgres>>, last_run_time: DateTime<Utc>, discord_client: &Arc<serenity::CacheAndHttp>) -> Self {
        Self { kv_client: kv_client.clone(), db_pool: db_pool.clone(), last_run_time, discord_client: discord_client.clone() }
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

pub(crate) async fn job_scheduler(
    app_config: &DiscordConfig,
    jobs: &Vec<Box<dyn Job>>,
    shard_key: &uuid::Uuid,
    discord_client: &Arc<serenity::CacheAndHttp>,
) -> Result<()> {
    let span = debug_span!("fercord.jobs.scheduler", last_run_time = field::Empty, interval_mins = &app_config.job_interval_min, failed_jobs = 0, completed_jobs = 0, job_count = field::display(&jobs.len()), shard_key = field::display(&shard_key));
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

    loop {
         // get last run time and compare to interval, sleep for difference or run immediately
        event!(Level::TRACE, "Retrieving last run state");
        let last_job_state = get_last_runtime(shard_key, &kv_client).await?;

        let last_time_ran = last_job_state.map_or(DateTime::<Utc>::MIN_UTC, |s| s.last_run);
        span.record("last_run_time", field::display(&last_time_ran));

        let since_last_run = Utc::now() - last_time_ran;
        event!(Level::INFO, "Time since last run: {:?} min.", &since_last_run.num_minutes());

        let mut failed_jobs = 0;
        let mut completed_jobs = 0;

        for job in jobs {

            let job_args = Arc::new(JobArgs::new(&kv_client, &db_pool, last_time_ran, &discord_client));
            
            if let Err(e) = job.run(&job_args).await {
                event!(Level::ERROR, "Encountered an error during a background job: {:?}", e);
                failed_jobs += 1;
            } else {
                completed_jobs += 1;
            }
    
            span.record("failed_jobs", field::display(&failed_jobs));
            span.record("completed_jobs", field::display(&completed_jobs));
        }
    
        info!("Attempted all jobs in this run");
    
        save_job_state(shard_key, &kv_client).await?;

        info!("Sleeping until next interval");
        _ = job_interval.tick().await;
    }
}

/// Save the job state for the given shard key and using the given KVClient.
async fn save_job_state(shard_key: &uuid::Uuid, kv_client: &Arc<KVClient>) -> Result<()> {
    let span = span!(Level::DEBUG, "fercord.jobs.save_state", shard_key = field::display(shard_key));
    let _enter = span.enter();

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
    let span = span!(Level::TRACE, "fercord.jobs", job_shard_key = field::display(&job_shard_key));
    let _enter = span.enter();

    let state_ident = JobState::for_identity(job_shard_key);
    let state_json = kv_client
        .get_json::<JobState>(&state_ident).await
        .with_context(|| format!("Error getting job state for shard {}", job_shard_key))?;

    Ok(state_json)
}
