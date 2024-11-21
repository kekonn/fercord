use actix_session::{storage::RedisSessionStore, Session, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::http::header::TryIntoHeaderValue;
use actix_web::http::{header, StatusCode};
use actix_web::{
    get, middleware, web, App, HttpRequest, HttpResponse,
    HttpServer, Responder, Result,
};
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use thiserror::Error;
use tracing::{event, Level};
use utoipa::{OpenApi, ToSchema};
use utoipa_actix_web::{scope, AppExt};
use utoipa_scalar::{Scalar, Servable};

use fercord_common::{cli, prelude::*};
use fercord_storage::{db::Pool, prelude::*};

use crate::model::HealthCheck;

mod discord;
mod model;
mod util;
mod user;

pub(crate) const SESSION_DATA_KEY: &str = "session_data";
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

mod tags {
    pub const AUTHENTICATION: &str = "Authentication";
    pub const UTILITY: &str = "Utility";
    pub const USER: &str = "User";
}

#[derive(OpenApi)]
#[openapi(
    info(description = "The Fercord management backend and REST api", title = "Fercord REST Api"),
)]
struct ApiDoc;

#[derive(Error, Debug, Serialize, Deserialize, ToSchema)]
pub enum ApiError {
    #[error("Received an OAuth token of invalid length")]
    OAuthTokenInvalidLength,
    #[error("An error occurred during token exchange: {0}")]
    OAuthTokenExchangeError(String),
    #[error("Please visit /discord_auth to authenticate.")]
    Unauthorized,
    #[error("An unexpected error occurred: {0}")]
    ServerError(String),
}

impl actix_web::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::OAuthTokenInvalidLength => StatusCode::BAD_REQUEST,
            &Self::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let mut res = HttpResponse::new(self.status_code());

        let buf = web::BytesMut::from(
            serde_json::to_vec_pretty(&self)
                .expect("Error serializing ApiError to json")
                .as_slice(),
        );

        let mime = mime::APPLICATION_JSON.try_into_value().unwrap();
        res.headers_mut()
            .insert(header::CONTENT_TYPE, mime);

        res.set_body(actix_web::body::BoxBody::new(buf))
    }
}

#[utoipa::path(get,
    tag = tags::UTILITY,
    responses(
        (status = OK, description = "Retrieved the health check data successfully", body = HealthCheck)
    )
)]
#[get("/healthz")]
#[tracing::instrument(name = "api.health")]
async fn health_check(
    _req: HttpRequest,
    db: web::Data<Pool>,
    kv: web::Data<KVClient>,
) -> Result<impl Responder> {
    let db_res = db.acquire().await;
    let kv_res = kv.connection_check().await;

    Ok(HttpResponse::Ok().json(HealthCheck {
        database: db_res.is_ok(),
        kv: kv_res.is_ok(),
    }))
}

#[utoipa::path(
    tag = tags::AUTHENTICATION,
    responses(
        (status = 200, description = "User has successfully authenticated with Fercord"),
        (status = 500, description = "An error occurred during the authentication", body = ApiError, content_type = mime::APPLICATION_JSON.to_string())
    )
)]
#[get("/discord_auth")]
#[tracing::instrument(name = "api.discord.auth", level = "trace", skip_all)]
async fn discord_auth(
    oauth_response: web::Query<model::DiscordOAuthResponse>,
    session: Session,
    config: web::Data<DiscordConfig>,
) -> Result<impl Responder> {
    let oauth_response = oauth_response.0;
    let discord_token = oauth_response.code;

    if discord_token.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError::OAuthTokenInvalidLength));
    }

    let config = config.get_ref().clone();

    let new_session: model::DiscordSessionData = discord::Client::try_from_auth_code(
        &discord_token,
        config
            .client_id
            .expect("client_id is required when running the API"),
        &config
            .client_secret
            .expect("client_secret is required when running the API"),
    )
    .await
    .map_err(|e| ApiError::OAuthTokenExchangeError(e.to_string()))?
    .into();
    if let Some(session_data) = session
        .get::<model::DiscordSessionData>(SESSION_DATA_KEY)
        .map_err(|e| ApiError::OAuthTokenExchangeError(e.to_string()))?
    {
        event!(Level::TRACE, "Session contained existing session data");

        if new_session.expires_in <= session_data.expires_in {
            event!(
                Level::DEBUG,
                "Newer token is more up to date, overwriting old sessions data..."
            );
            session.insert(SESSION_DATA_KEY, new_session)?;
        } else {
            event!(
                Level::DEBUG,
                "New session data seems to be older. Retaining old session data"
            );
        }
    } else {
        event!(
            Level::DEBUG,
            "No session data existed yet. Storing session data"
        );
        session.insert(SESSION_DATA_KEY, new_session)?;
    }

    Ok(HttpResponse::TemporaryRedirect()
        .insert_header((header::LOCATION, "/"))
        .finish())
}

#[utoipa::path(
    tag = tags::AUTHENTICATION,
    responses(
        (status = 200, description = "Logs out the current user")
    )
)]
#[get("/logout")]
async fn logout(session: Session, discord_client: discord::Client) -> Result<impl Responder> {

    let logout_result = discord_client.logout().await;
    session.purge();

    logout_result?;

    Ok(HttpResponse::Ok())
}



#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = cli::Args::parse();
    let config_file_path = args.config;

    event!(Level::DEBUG, %config_file_path, "Reading configuration");
    // Load application config
    let config = DiscordConfig::from_env_and_file(&config_file_path)
        .expect("Error loading Fercord API config");
    let config_valid = config.is_valid_api_config();
    if !config_valid {
        event!(
            Level::ERROR,
            "The configuration is missing required fields for use with the API service"
        );
        bail!("The configuration is missing required fields for use with the API service")
    }

    let kv = KVClient::new(&config).expect("Error constructing KV client");
    let db = db::setup(&config.database_url.clone())
        .await
        .expect("Error constructing db pool");
    let session_key = Key::from(
        config
            .session_key
            .as_ref()
            .expect("session_key is required in config when running the API")
            .as_bytes(),
    );
    let redis_session_store = RedisSessionStore::new(&config.redis_url)
        .await
        .expect("Error creating redis session store");

    HttpServer::new(move || {
        let (app, _) = App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(kv.clone()))
            .app_data(web::Data::new(db.clone()))
            .map(|app| {
                app.wrap(SessionMiddleware::new(
                    redis_session_store.clone(),
                    session_key.clone(),
                ))
            })
            .map(|app| app.wrap(middleware::Compress::default()))
            .service(discord_auth)
            .service(logout)
            .service(health_check)
            .service(
                scope::scope("/api")
                    .service(
                        scope::scope("/v1")
                        .service(
                            scope::scope("/user")
                                .service(user::identify)
                                .service(user::guilds)
                        )
                    )
            )
            .openapi_service(|api| {
                Scalar::with_url("/docs", api)
            })
            .split_for_parts();

        app.service(web::redirect("/", "/docs"))
    })
    .bind((Ipv4Addr::UNSPECIFIED, 8888))?
    .run()
    .await
    .map_err(|e| anyhow!(e))
}