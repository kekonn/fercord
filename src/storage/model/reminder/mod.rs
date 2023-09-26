#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

use crate::storage::db::{Repo, Repository};

#[cfg(feature = "sqlite")]
use crate::storage::model::reminder::sqlite::*;


#[cfg(feature = "postgres")]
use crate::storage::model::reminder::postgres::*;

use chrono::{DateTime, NaiveDateTime, Utc};
use poise::async_trait;
use sqlx::{Row, FromRow, AnyPool, Any};
use anyhow::{Result, Context};
use sqlx::any::AnyRow;
use tracing::{event, Level, trace};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Reminder {
    pub id: i64,
    pub who: u64,
    pub when: DateTime<Utc>,
    pub what: String,
    pub server: u64,
    pub channel: u64,
}

pub type ReminderRepo<'r> = Repo<'r>;

impl<'r> ReminderRepo<'r> {

    /// Get all reminders between the given moment and now.
    pub async fn get_reminders_since(&self, moment: &DateTime<Utc>) -> Result<Vec<Reminder>> {
        let now = Utc::now();
        event!(Level::TRACE, "Getting all reminders between {} and {}", &moment, &now);

        let query = sqlx::query(REMINDERS_BETWEEN_QUERY)
            .bind(moment)
            .bind(now)
            .fetch_all(self.pool).await?;

        trace!("Found {} reminders", query.len());

        let reminders: Vec<Reminder> = query_to_entity(query);

        trace!("{} records remaining after mapping", reminders.len());

        Ok(reminders)
    }

    /// Get all reminders before the given moment
    pub async fn get_reminders_before(&self, moment: &DateTime<Utc>) -> Result<Vec<Reminder>> {
        event!(Level::TRACE, "Getting all reminders before {}", &moment);

        let query = sqlx::query(REMINDERS_BETWEEN_QUERY)
            .bind(NaiveDateTime::UNIX_EPOCH)
            .bind(moment)
            .fetch_all(self.pool).await.with_context(|| "Error fetching expired reminders")?;

        Ok(query_to_entity(query))
    }

    /// Bulk delete reminders
    pub async fn delete_reminders(&self, reminders: Vec<Reminder>) -> Result<()> {
        event!(Level::TRACE, "Deleting {} reminders", &reminders.len());

        if reminders.is_empty() {
            return Ok(());
        }

        let reminder_ids = reminders.iter().fold(String::from(""), |s, r| s + r.id.to_string().as_str() + ",");
        let reminder_ids = reminder_ids.trim_end_matches(',');

        let trans = self.pool.begin().await.with_context(|| "Error starting transaction")?;

        let query = sqlx::query(&BATCH_DELETE_QUERY.replace('?', reminder_ids))
            .execute(self.pool).await.with_context(|| "Error deleting reminders")?;

        event!(Level::DEBUG, "Deleted {} reminders", query.rows_affected());

        trans.commit().await.with_context(|| "Error committing transaction")
    }
}

/// Converts an iterator over `AnyRow`s into a vector of `Reminder`s
fn query_to_entity(query: Vec<AnyRow>) -> Vec<Reminder> {
    query.iter().map(|r| ReminderEntity::from_row(r).ok())
        .filter_map(|o| o.and_then(|f| Reminder::try_from(f).ok())).collect()
}

#[async_trait]
impl<'r> Repository<Reminder, i64> for ReminderRepo<'r> {

    /// Inserts a reminder into the database and returns the id of the inserted record upon success.
    async fn insert(&self, entity: &Reminder) -> Result<i64> {
        event!(Level::TRACE, "Adding or updating entity {:?}", entity);
        let db_ent = ReminderEntity::from(entity);

        let trans = self.pool.begin().await?;

        let query = sqlx::query(INSERT_QUERY)
            .bind(db_ent.who)
            .bind(db_ent.when)
            .bind(db_ent.what)
            .bind(db_ent.server)
            .bind(db_ent.channel)
            .fetch_one(self.pool).await.with_context(|| "Error saving or updating entity")?;

        trans.commit().await?;
        Ok(query.try_get(0)?)
    }

    /// Deletes the given reminder from the database.
    async fn delete(&self, entity: Reminder) -> Result<()> {
        event!(Level::TRACE, "Deleting entity {:?}", &entity);

        let trans = self.pool.begin().await?;

        sqlx::query(DELETE_QUERY)
            .bind(entity.id)
            .execute(self.pool).await.with_context(|| "Error deleting reminder")?;

        trans.commit().await?;

        Ok(())
    }

    /// Get a reminder by id.
    async fn get(&self, id: i64) -> Result<Option<Reminder>> {
        event!(Level::TRACE, "Retrieving Reminder with id {}", id);

        if let Some(query) = sqlx::query_as::<Any, ReminderEntity>(GET_ONE_QUERY)
                .bind(id)
                .fetch_optional(self.pool).await.with_context(|| "Error getting Reminder with id")? {
                    Ok(Some(query.try_into().with_context(|| "Error converting entity to reminder")?))
        } else {
            Ok(None)
        }
    }

}

impl Reminder {

    /// Create a `Reminder` repository that connects to the database with the borrowed pool.
    pub fn repository(pool: &AnyPool) -> ReminderRepo {
        Repo { pool }
    }
}

/// Because a lot of our types are not supported by databases
#[derive(Debug, sqlx::FromRow)]
struct ReminderEntity {
    pub id: i64,
    pub who: String,
    pub when: DateTime<Utc>,
    pub what: String,
    pub server: String,
    pub channel: String,
}

impl From<&Reminder> for ReminderEntity {
    fn from(value: &Reminder) -> Self {
        Self { id: value.id, who: value.who.to_string(), when: value.when, what: value.what.clone(), server: value.server.to_string(), channel: value.channel.to_string() }
    }
}

impl TryFrom<ReminderEntity> for Reminder {
    type Error = anyhow::Error;

    fn try_from(value: ReminderEntity) -> Result<Self> {
        Ok(Self { id: value.id, who: value.who.parse()?, when: value.when, what: value.what, server: value.server.parse()?, channel: value.channel.parse()? })
    }
}