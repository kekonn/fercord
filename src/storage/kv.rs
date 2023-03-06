
use redis::{AsyncCommands, Client, ToRedisArgs};
use anyhow::{Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::{Send, Sync}, fmt::Debug};
use tracing::*;

use crate::config::DiscordConfig;

pub type KVIdentity = String;

/// A generic KV store client.
/// 
/// ## Creation
/// Create a new client by calling `KVClient::`
pub struct KVClient {
    client: Client
}

/// Marks an object as identifyable to the KV Client.
pub trait Identifiable {
    fn kv_key(&self) -> KVIdentity;
}

impl KVClient {
    /// Create a new `KVClient` from a `&DiscordConfig`.
    pub fn new(config: &DiscordConfig) -> Result<Self> {
        let client = redis::Client::open(config.database_url.as_ref())?;

        Ok(Self { client })
    }

    /// Save a value to the KV store.
    pub async fn save<T>(&self, record: T) -> Result<()> 
        where T: Identifiable + Serialize + Send + Sync + ToRedisArgs + Debug
    {
        let span = trace_span!("storage.kv_client", record = field::Empty, save_key = field::Empty);
        let _enter = span.enter();
        span.record("record", field::debug(&record));
        event!(parent: &span, Level::TRACE, "Saving a record to the KV store");

        let con = &mut self.client.get_async_connection().await?;

        let save_key = &record.kv_key();
        span.record("save_key", field::debug(&save_key));

        match con.set(save_key, record).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(?e, "Error saving valing to kv store");

                Err(anyhow!(e))
            },
        }
    }

    /// Save complex objects as json in redis.
    pub async fn save_json<T>(&self, record: T) -> Result<()> 
        where T: Identifiable + Serialize + Send + Sync + Debug
    {
        let span = trace_span!("storage.kv_client", record = field::Empty, save_key = field::Empty);
        let _enter = span.enter();
        span.record("record", field::debug(&record));
        event!(Level::TRACE, "Saving a record to the KV store in json mode");

        let json = serde_json::to_string(&record)?;

        let con = &mut self.client.get_async_connection().await?;

        let save_key = &record.kv_key();
        span.record("save_key", field::debug(&save_key));

        match con.set(save_key, json).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(?e, "Error saving valing to kv store");

                Err(anyhow!(e))
            },
        }
    }

    /// Retrieve a complex record for the given key.
    pub async fn get_json<T>(&self, record: &T) -> Result<T> 
        where T: DeserializeOwned + Send + Sync + Debug + Identifiable
    {
        let span = trace_span!("storage.kv_client", record = field::Empty, object_key = field::Empty);
        let _enter = span.enter();

        let key = record.kv_key();
        span.record("object_key", &key);

        event!(Level::TRACE, "Retrieving a record from the kv store");

        let con = &mut self.client.get_async_connection().await?;

        let json: String = con.get(key).await?;
        
        let record = serde_json::from_str(json.as_str())?;

        span.record("record", field::debug(&record));

        Ok(record)
    }   
}
