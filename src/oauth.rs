use crate::{auth, db, BAD_REQUEST};
use crate::rest::{BAD_REQUEST_RETURN, INTERNAL_ERR_RESPONSE, NOT_AUTHORIZED_RESPONSE};
use actix_web::{web, Error};
use actix_web::{http, HttpRequest, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
struct TgProfile {
    auth_date: DateTime<Utc>,
    id: u64,
    hash: String,
    username: Option<String>,
    photo_url: Option<String>,
    first_name: String,
    last_name: Option<String>,
}

#[derive(Deserialize)]
struct TgProfileResponse {
    data: TgProfile,
}

pub async fn oauth_telegram(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    // TODO: make this a config
    // if let Ok(secret) = std::env::var("TG_BOT_SECRET") {

    // }
    Ok(BAD_REQUEST!())
}

pub async fn oauth_aosc(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    Ok(BAD_REQUEST!())
}
