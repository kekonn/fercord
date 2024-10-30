
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, Result, web, get, post};
use actix_web::http::header::{ContentType, TryIntoHeaderValue};
use actix_web::cookie::Key;
use actix_session::{Session, SessionMiddleware, storage::RedisSessionStore};

use serde::{Deserialize, Serialize};
use serenity::all::StatusCode;
use thiserror::Error;
use tracing::{event, Level};

use fercord_storage::{prelude::*, db::Pool};
use fercord_common::prelude::*;

use crate::model::HealthCheck;

mod model;
mod discord;

const SESSION_DATA_KEY: &str = "session_data";

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Received an OAuth token of invalid length")]
    OAuthTokenInvalidLength,
    #[error("An error occurred during token exchange: {}", reason)]
    OAuthTokenExchangeError {
        reason: String
    },
}

impl actix_web::ResponseError for ApiError {
    fn status_code(&self) -> serenity::all::StatusCode {
        match self {
            Self::OAuthTokenInvalidLength => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let mut res = HttpResponse::new(self.status_code());

        let buf = web::BytesMut::from(serde_json::to_vec_pretty(&self).expect("Error serializing ApiError to json").as_slice());

        let mime = mime::TEXT_PLAIN_UTF_8.try_into_value().unwrap();
        res.headers_mut().insert(actix_web::http::header::CONTENT_TYPE, mime);

        res.set_body(actix_web::body::BoxBody::new(buf))
    }
}

async fn health_check(_req: HttpRequest, db: web::Data<Pool>, kv: web::Data<KVClient>) -> Result<impl Responder> {
    let db_res = db.acquire().await;
    let kv_res = kv.connection_check().await;
    
    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(HealthCheck {
            database: db_res.is_ok(),
            kv: kv_res.is_ok(),
        }))
}

#[get("/discord_auth")]
async fn discord_auth(code: web::Query<model::DiscordOAuthResponse>, session: Session) -> Result<impl Responder> {
    let oauth_response = code.0;
    event!(Level::TRACE, "Received Discord OAuth code: {}", &oauth_response.auth_code);
    let discord_token = oauth_response.auth_code;

    if discord_token.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError::OAuthTokenInvalidLength));
    }

    let new_session: model::SessionData = discord::Client::try_from_auth_code(&discord_token).await.map_err(|e| ApiError::OAuthTokenExchangeError { reason: e.to_string() })?.into();
    if let Some(session_data) = session.get::<model::SessionData>(SESSION_DATA_KEY).map_err(|e| ApiError::OAuthTokenExchangeError { reason: e.to_string() })? {
        event!(Level::TRACE, "Session contained existing session data");

        if new_session.expires_in > session_data.expires_in {
            event!(Level::DEBUG, "Newer token is more up to date, overwriting old sessions data...");
            session.insert(SESSION_DATA_KEY, new_session)?;
        } else {
            event!(Level::DEBUG, "New session data seems to be older. Retaining old session data");
        }
    } else {
        event!(Level::DEBUG, "No session data existed yet. Storing session data");
        session.insert(SESSION_DATA_KEY, new_session)?;
    }

    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = DiscordConfig::from_env_and_file("config.toml").expect("Error loading Fercord API config");
    let kv = KVClient::new(&config).expect("Error constructing KV client");
    let db = db::setup(&config.database_url).await.expect("Error constructing db pool");
    let session_key = Key::from(config.session_key.as_bytes());
    let redis_session_store = RedisSessionStore::new(&config.redis_url).await.expect("Error creating redis session store");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(kv.clone()))
            .app_data(web::Data::new(db.clone()))
            .wrap(SessionMiddleware::new(redis_session_store.clone(), session_key.clone()))
            .route("/healthz", web::get().to(health_check))
    }).bind(("127.0.0.1", 8080))?
    .run()
    .await
}