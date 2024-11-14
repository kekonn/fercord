use actix_session::SessionExt;
use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use fercord_common::prelude::*;
use std::future::Future;
use std::pin::Pin;
use tracing::{event, Level};

use crate::discord::Client;
use crate::model;
use crate::ApiError;

impl FromRequest for Client {

    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            event!(Level::TRACE, "Constructing Discord client from request");
            let config = req.app_data::<web::Data<DiscordConfig>>()
                .ok_or(ApiError::ServerError("Error loading config data from session.".into()))?;

            if !config.is_valid_api_config() {
                return Err(ApiError::ServerError("Configuration is not valid for use with server.".into()));
            }

            let session_data = req.get_session()
                .get::<model::DiscordSessionData>(super::SESSION_DATA_KEY)
                .map_err(|_| ApiError::ServerError("Error retrieving session data".into()))?
                .ok_or(ApiError::Unauthorized)?;

            let client = Client::from_access_token(
                session_data.access_token,
                session_data.refresh_token,
                session_data.expires_in,
                config.client_id.unwrap(),
                config.client_secret.as_ref().unwrap().to_string(),
            );

            Ok(client)
        })

    }
}