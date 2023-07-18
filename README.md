# fercord
A discord bot written in Rust, for personal use.

## Configuration

```toml
discord_token = "your-bot-token"
database_url = "postgres://fercord:fercord@localhost/fercord"
redis_url = "redis://localhost/"
job_interval_min = 1
shard_key = "c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"
```

* **discord_token**: Your bot token
* **databse_url**: the url to the database. Currently we only support postgres
* **redis_url**: the url to the redis instance used to store runtime configuration
* **job_interval_min**: the interval (in minutes) that the scheduler leaves between runs
* **shard_key**: a UUID that should be unique per bot instance that is connecting to the same key-value store