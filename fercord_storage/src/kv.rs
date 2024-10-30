use std::{
    fmt::Debug,
    marker::{Send, Sync},
};
use std::time::Duration;

use anyhow::{anyhow, Result};
use redis::{AsyncCommands, Client, ConnectionLike, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};
use tracing::*;

use fercord_common::config::DiscordConfig;

pub type KVIdentity = String;

/// A generic KV store client.
///
/// ## Creation
/// Create a new client by calling `KVClient::`
#[derive(Debug, Clone)]
pub struct KVClient {
    client: Client,
}

/// Marks an object as identifyable to the KV Client.
pub trait Identifiable {
    fn kv_key(&self) -> KVIdentity;
}

impl KVClient {
    /// Create a new `KVClient` from a `&DiscordConfig`.
    #[instrument(level = "trace")]
    pub fn new(config: &DiscordConfig) -> Result<Self> {
        let client = redis::Client::open(config.redis_url.as_ref())?;

        Ok(Self { client })
    }

    /// Save a value to the KV store.
    pub async fn save<T>(&self, record: T) -> Result<()>
    where
        T: Identifiable + Serialize + Send + Sync + ToRedisArgs + Debug,
    {
        let span = trace_span!(
            "storage.kv_client",
            record = field::Empty,
            save_key = field::Empty
        );
        let _enter = span.enter();
        span.record("record", field::debug(&record));
        event!(
            parent: &span,
            Level::TRACE,
            "Saving a record to the KV store"
        );

        let con = &mut self.client.get_multiplexed_async_connection().await?;

        let save_key = &record.kv_key();
        span.record("save_key", field::debug(&save_key));

        match con.set(save_key, record).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(?e, "Error saving valing to kv store");

                Err(anyhow!(e))
            }
        }
    }

    /// Save complex objects as json in redis.
    pub async fn save_json<T>(&self, record: T) -> Result<()>
    where
        T: Identifiable + Serialize + Send + Sync + Debug,
    {
        let span = trace_span!(
            "storage.kv_client",
            record = field::Empty,
            save_key = field::Empty
        );
        let _enter = span.enter();
        span.record("record", field::debug(&record));
        event!(Level::TRACE, "Saving a record to the KV store in json mode");

        let json = serde_json::to_string(&record)?;

        let con = &mut self.client.get_multiplexed_async_connection().await?;

        let save_key = &record.kv_key();
        span.record("save_key", field::debug(&save_key));

        match con.set(save_key, json).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(?e, "Error saving valing to kv store");

                Err(anyhow!(e))
            }
        }
    }

    /// Retrieve a complex record for the given key.
    pub async fn get_json<T>(&self, record: &T) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync + Debug + Identifiable,
    {
        let span = trace_span!(
            "storage.kv_client",
            record = field::Empty,
            object_key = field::Empty
        );
        let _enter = span.enter();

        let key = record.kv_key();
        span.record("object_key", &key);

        event!(Level::TRACE, "Retrieving a record from the kv store");

        let con = &mut self.client.get_multiplexed_async_connection().await?;

        if !con.exists(&key).await? {
            return Ok(None);
        }

        let json: String = con.get(key).await?;

        let record = serde_json::from_str(json.as_str())?;

        span.record("record", field::debug(&record));

        Ok(Some(record))
    }

    /// Perform a connection check.
    /// If we can obtain an open connection in 15 seconds, we return `Ok()`.
    pub async fn connection_check(&self) -> Result<()> {
        let connection_result = self.client.get_connection_with_timeout(Duration::from_secs(15));

        match connection_result {
            Ok(conn) => {
                if conn.is_open() {
                    Ok(())
                } else {
                    Err(anyhow!("Could not open connection"))
                }
            },
            Err(e) => Err(anyhow!(e))
        }
    }
}