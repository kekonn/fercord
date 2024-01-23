use actix_web::http::header::ContentType;
use actix_web::{HttpServer, App, web, HttpRequest, Responder, HttpResponse, Result};
use fercord_storage::config::DiscordConfig;
use fercord_storage::kv::KVClient;
use fercord_storage::db::{setup, Pool};

use crate::model::HealthCheck;

mod model;

async fn health_check(_req: HttpRequest, db: web::Data<Pool>, kv: web::Data<KVClient>) -> Result<impl Responder> {
    let db = db.acquire().await?;
    kv.connection_check().await?;
    
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