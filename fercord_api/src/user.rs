use crate::{discord, model, tags};
use actix_web::{
    get, web,
    Responder, Result,
};

#[utoipa::path(
    tag = tags::USER,
    responses(
        (status = 200, description = "All user data Discord shares with the API", body = model::AuthorizedIdentity, content_type = mime::APPLICATION_JSON.to_string()),
        (status = 401, description = "Please log in first"),
        (status = 500, description = "An unexpected error occurred", body = super::ApiError, content_type = mime::APPLICATION_JSON.to_string())
    )
)]
#[get("/identify")]
async fn identify(discord_client: discord::Client) -> Result<impl Responder> {

    let user_data = discord_client.get_user_identity().await?;

    Ok(web::Json(user_data))
}

#[utoipa::path(
    tag = tags::USER,
    responses(
        (status = 200, description = "All guilds (servers) the logged on user belongs to", body = Vec<crate::model::Guild>, content_type = mime::APPLICATION_JSON.to_string()),
        (status = 401, description = "Please log in first"),
        (status = 500, description = "An unexpected error occurred", body = super::ApiError, content_type = mime::APPLICATION_JSON.to_string())
    )
)]
#[get("/guilds")]
async fn guilds(discord_client: discord::Client) -> Result<impl Responder> {

    let guilds = discord_client.get_managing_guilds().await?;

    Ok(web::Json(guilds))
}