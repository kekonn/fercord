use crate::storage::db::{Repo, Repository};

use chrono::{DateTime, Utc};
use poise::async_trait;
use sqlx::{Postgres, Row, Pool, FromRow};
use anyhow::{Result, Context};
use tracing::{event, Level};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Reminder {
    pub id: i64,
    pub who: u64,
    pub when: DateTime<Utc>,
    pub what: String,
    pub server: u64,
    pub channel: u64,
}

pub type ReminderRepo<'r> = Repo<'r, Postgres>;

impl<'r> ReminderRepo<'r> {

    /// Get all reminders between the given moment and now.
    pub async fn get_reminders_since(&self, moment: &DateTime<Utc>) -> Result<Vec<Reminder>> {
        let now = Utc::now();
        event!(Level::TRACE, "Getting all reminders between {} and {}", &moment, &now);

        let query = sqlx::query(r#"SELECT who, "when", what, "server", channel
        FROM public.reminders
        WHERE "when" between $1 and $2
        "#)
            .bind(moment)
            .bind(now)
            .fetch_all(self.pool).await?;

        let reminders = query.iter().map(|r| ReminderEntity::from_row(r).map_or(None, |r| Some(r)))
            .filter_map(|o| o.map_or(None, |f| Reminder::try_from(f).map_or(None, |f| Some(f))));

        Ok(reminders.collect())
    }
}

#[async_trait]
impl<'r> Repository<Reminder, i64> for ReminderRepo<'r> {

    /// Inserts a reminder into the database and returns the id of the inserted record upon success.
    async fn insert(&self, entity: &Reminder) -> Result<i64> {
        event!(Level::TRACE, "Adding or updating entity {:?}", entity);
        let db_ent = ReminderEntity::from(entity);

        let trans = self.pool.begin().await?;

        let query = sqlx::query(r#"INSERT INTO public.reminders
        (who, "when", what, "server", channel)
        VALUES($1, $2, $3, $4, $5) RETURNING id;"#)
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

        sqlx::query("DELETE FROM public.reminders WHERE id=$1;")
            .bind(entity.id)
            .execute(self.pool).await.with_context(|| "Error deleting reminder")?;

        trans.commit().await?;

        Ok(())
    }

    /// Get a reminder by id.
    async fn get(&self, id: i64) -> Result<Option<Reminder>> {
        event!(Level::TRACE, "Retrieving Reminder with id {}", id);

        if let Some(query) = sqlx::query_as::<Postgres, ReminderEntity>("SELECT * FROM public.reminders WHERE id = $1")
                .bind(id).fetch_optional(self.pool).await.with_context(|| "Error getting Reminder with id")? {
             Ok(Some(query.try_into().with_context(|| "Error converting entity to reminder")?))
        } else {
            Ok(None)
        }
    }
}

impl Reminder {

    /// Create a `Reminder` repository that connects to the database with the borrowed pool.
    pub fn repository(pool: &Pool<Postgres>) -> ReminderRepo {
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