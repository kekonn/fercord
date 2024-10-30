use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use tracing::{event, Level};

use fercord_common::prelude::*;
use fercord_storage::prelude::*;

use crate::healthchecks::CheckType::Database;

#[derive(Serialize, Debug)]
pub struct HealthCheck {
    pub time: i64,
    pub check_type: CheckType,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub enum CheckType {
    Redis,
    Database,
}

pub async fn perform_healthchecks(config: &DiscordConfig) -> Result<String> {
    event!(Level::DEBUG, "Performing health checks");

    let mut checks: Vec<HealthCheck> = Vec::<HealthCheck>::with_capacity(2);

    let db_check_start = Utc::now();
    event!(Level::TRACE, %db_check_start, "Starting DB Health check");
    let db_result = db::setup(config.database_url.as_ref()).await;

    if let Ok(pool) = db_result {
        let conn = pool.acquire().await;
        let db_check_end = Utc::now();

        checks.push(HealthCheck {
            check_type: Database,
            time: (db_check_end - db_check_start).num_milliseconds(),
            success: conn.is_ok(),
        });
    } else {
        let db_check_end = Utc::now();

        checks.push(HealthCheck {
            check_type: Database,
            time: (db_check_end - db_check_start).num_milliseconds(),
            success: db_result.is_ok(),
        });
    }

    event!(Level::TRACE, "Finished DB Health check");

    let kv_check_start = Utc::now();
    event!(Level::TRACE, %kv_check_start, "Starting KV Health check");
    let kv_result = KVClient::new(config);

    if let Ok(kv_client) = kv_result {
        let conn_check = kv_client.connection_check().await;
        let kv_check_end = Utc::now();

        event!(Level::TRACE, %kv_check_end, "Finished KV health check");
        checks.push(HealthCheck {
            success: conn_check.is_ok(),
            check_type: CheckType::Redis,
            time: (kv_check_end - kv_check_start).num_milliseconds(),
        })
    } else {
        let kv_check_end = Utc::now();

        event!(Level::TRACE, %kv_check_end, "Finished KV health check");
        checks.push(HealthCheck {
            success: false,
            check_type: CheckType::Redis,
            time: (kv_check_end - kv_check_start).num_milliseconds(),
        })
    }

    let result = serde_json::to_string_pretty(&checks)
        .with_context(|| "Error serializing health check json")?;
    Ok(result)
}
