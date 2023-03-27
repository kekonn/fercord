use crate::storage::db::{Repo, Repository};

use chrono::{DateTime, Utc};
use poise::async_trait;
use sqlx::{Postgres, Row, Pool};
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

#[async_trait]
impl<'r> Repository<Reminder, i64> for ReminderRepo<'r> {
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

    async fn delete (&self, entity: Reminder) -> Result<()> {
        event!(Level::TRACE, "Deleting entity {:?}", &entity);

        let trans = self.pool.begin().await?;

        sqlx::query("DELETE FROM public.reminders WHERE id=$1;")
            .bind(entity.id)
            .execute(self.pool).await.with_context(|| "Error deleting reminder")?;

        trans.commit().await?;

        Ok(())
    }
}

impl Reminder {

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