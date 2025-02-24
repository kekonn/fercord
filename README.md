# fercord
A discord bot written in Rust, for personal use.

## Configuration
### When running locally/directly

You can specify the location of the config file by setting the `CONFIG` environment variable (i.e.: `CONFIG=$XDG_CONFIG_HOME/fercord/config.toml`) or if not specified we look in `.config/config.toml` in the current working directory. 

Example `config.toml`:

```toml
discord_token = "your-bot-token"
database_url = "sqlite://fercord.db"
redis_url = "redis://localhost/"
job_interval_min = 1
shard_key = "c69b7bb6-0ca4-40da-8bad-26d9d4d2fb50"
```

* **discord_token**: Your bot token
* **databse_url**: the url to the database. Currently we only support postgres
* **redis_url**: the url to the redis instance used to store runtime configuration
* **job_interval_min**: the interval (in minutes) that the scheduler leaves between runs
* **shard_key**: a UUID that should be unique per bot instance that is connecting to the same key-value store

### API Config
The following items should be added to the configuration file if the file is to be used with the web API:

```toml
session_key = "base64 value"
client_id = 948517362313863198
```

* **session_key**: A random base64'ed value that is at least 64 characters long (after base64 encode). This value is used as the base secret for session encryption.
* **client_id**: Go to the [Discord Developer Portal](https://discord.com/developers/applications), select your application and go to the OAuth 2 settings. 
Copy the client id found there to this setting (no quotes).

You will also need to set the following environment variable: `FERCORD_CLIENT_SECRET`. This secret can be found on the same page as the client id.

### Configuration from environment variables

Every variable mentioned above can be overriden from the environment. The correct environment variable prefix is "FERCORD_".

To override your discord token you would set the environment variable `FERCORD_DISCORD_TOKEN` to your token.
Settings set through environment variables take precedence over configuration set via a config file.

## Docker

The container has a built-in `config.toml` stored at `/config/config.toml`. The only setting there is job_interval_min (set to 1).
If you want to build your own docker image, you can override the location fercord looks for the config file by setting the `CONFIG` environment variable in the Dockerfile.

This means the following environment variables HAVE to be specified in order for the container to be able to function:

* FERCORD_DISCORD_TOKEN
* FERCORD_DATABASE_URL
* FERCORD_REDIS_URL
* FERCORD_SHARD_KEY
* FERCORD_SESSION_KEY
* FERCORD_CLIENT_ID

If you want a different job interval, you can specify it through `FERCORD_JOB_INTERVAL_MIN`.

The sqlite database is placed in the `/data` directory and called `fercord.db`. The container exposes `/data` as a volume, so it will persist between updates etc.

### RUST_LOG

The default value for `RUST_LOG` in the container is `info,sqlx::query=warn`. You can override this, but if you choose to, please copy the value for `sqlx::query`.

Any log level lower than that will output the queries that sqlx runs, which might be a security issue.

## Web API

### API docs

Visit `/docs` on the root of the API and you will get the API docs.