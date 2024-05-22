use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, Result, web};
use actix_web::http::header::ContentType;
use anyhow::anyhow;

use fercord_storage::config::DiscordConfig;
use fercord_storage::db::{Pool, setup};
use fercord_storage::kv::KVClient;

use crate::model::HealthCheck;

mod model;

#[derive(Debug)]
struct ApiError {
    err: anyhow::Error
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ApiError: {}", self.err)
    }
}

impl actix_web::error::ResponseError for ApiError {
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError { err }
    }
}

async fn health_check(_req: HttpRequest, db: web::Data<Pool>, kv: web::Data<KVClient>) -> Result<impl Responder> {
    db.acquire().await.map_err(|err| ApiError::from(anyhow!(err)))?;
    kv.connection_check().await.map_err(ApiError::from)?;
    
    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(HealthCheck::default()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let config = DiscordConfig::from_env_and_file("config.toml").expect("Error loading Fercord API config");
    let kv = KVClient::new(&config).expect("Error constructing KV client");
    let db = setup(&config.database_url).await.expect("Error constructing db pool");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(kv.clone()))
            .app_data(web::Data::new(db.clone()))
            .route("/healthz", web::get().to(health_check))
    }).bind(("127.0.0.1", 8080))?
    .run()
    .await
    
}