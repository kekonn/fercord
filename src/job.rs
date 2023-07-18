use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

use anyhow::{ Result, Context };
use chrono::{Utc, DateTime, Duration};
use serde::{ Deserialize, Serialize };
use sqlx::Pool;
use tracing::{ span, event, field, Span, Level };
use tokio_stream::Stream;

use crate::config::DiscordConfig;
use crate::storage::kv::*;
use crate::storage::db;

pub type Job = dyn Fn(&Arc<JobArgs>) -> JobFuture;

pub type JobFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

#[derive(Debug)]
pub struct JobArgs {
    pub kv_client: Arc<KVClient>,
    pub db_pool: Arc<Pool<sqlx::Postgres>>,
}

impl JobArgs {
    /// Create a new JobArgs struct from a `KVClient` and an sqlx Postgres pool.
    fn new(kv_client: &Arc<KVClient>, db_pool: &Arc<Pool<sqlx::Postgres>>) -> Self {
        Self { kv_client: kv_client.clone(), db_pool: db_pool.clone() }
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

pub struct JobStream;

impl Stream for JobStream {
    type Item = JobFuture;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

pub async fn job_scheduler(
    app_config: &DiscordConfig,
    jobs: &Vec<Box<Job>>
) -> Result<()> {
    let span = Span::current();

    event!(Level::DEBUG, "Setting up job scheduler db pool");
    let db_pool = db
        ::setup(app_config.database_url.as_ref()).await
        .with_context(|| "Error setting up database connection")?;
    let db_pool = Arc::new(db_pool);

    event!(Level::DEBUG, "Setting up job scheduler KV client");
    let kv_client = Arc::new(KVClient::new(app_config).with_context(|| "Error setting up KV client")?);

    let job_args = Arc::new(JobArgs::new(&kv_client, &db_pool));

    let shard_key = uuid::Uuid::new_v4();
    // get last run time and compare to interval, sleep for difference or run immediately
    event!(Level::TRACE, "Retrieving last run state");
    let last_job_state = get_last_runtime(&shard_key, &job_args.kv_client).await?;

    let last_time_ran = last_job_state.map_or(DateTime::<Utc>::MIN_UTC, |s| s.last_run);
    span.record("last_run_time", field::display(&last_time_ran));

    let since_last_run = Utc::now() - last_time_ran;
    event!(Level::INFO, "Time since last run: {:?}", &since_last_run);

    let job_interval = Duration::minutes(app_config.job_interval_min.into());
    span.record("interval", field::display(&job_interval));

    let mut failed_jobs = 0;
    let mut completed_jobs = 0;

    for job in jobs {

        if let Err(e) = job(&job_args).await {
            event!(Level::ERROR, "Encountered an error during a background job: {:?}", e);
            failed_jobs += 1;
        } else {
            completed_jobs += 1;
        }

        span.record("failed_jobs", field::display(&failed_jobs));
        span.record("completed_jobs", field::display(&completed_jobs));
    }

    event!(Level::INFO, "All jobs in this run attemted");

    save_job_state(&shard_key, &job_args.kv_client).await?;

    Ok(())
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