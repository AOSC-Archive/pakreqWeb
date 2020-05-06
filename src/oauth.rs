use crate::{auth, db, models, DbPool, BAD_REQUEST};
use crate::rest::{BAD_REQUEST_RETURN, INTERNAL_ERR_RESPONSE, NOT_AUTHORIZED_RESPONSE};
use actix_web::{error, web, Error, Responder};
use actix_web::{http, http::StatusCode, HttpRequest, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::path::{Iter, PathBuf};

#[derive(Deserialize)]
struct TgProfile {
    auth_date: DateTime<Utc>,
    id: u64,
    hash: String,
    username: String,
    #[serde(default)]
    photo_url: String,
    first_name: String,
    #[serde(default)]
    last_name: String,
}

#[derive(Deserialize)]
struct TgProfileResponse {
    data: TgProfile,
}

pub async fn oauth_telegram(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    // TODO: make this a config
    if let Ok(secret) = std::env::var("TG_BOT_SECRET") {

    }
    Ok(BAD_REQUEST!())
}

pub async fn oauth_aosc(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    Ok(BAD_REQUEST!())
}
