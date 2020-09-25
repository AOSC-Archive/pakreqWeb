use crate::{auth, db, BAD_REQUEST, INTERNAL_ERROR};
use crate::{
    models::Oauth,
    rest::{BAD_REQUEST_RETURN, INTERNAL_ERR_RESPONSE, NOT_AUTHORIZED_RESPONSE},
};
use actix_identity::Identity;
use actix_session::Session;
use actix_web::{get, http, web, Error, HttpRequest, HttpResponse};
use anyhow::{anyhow, Result};
use awc::Client as awcClient;
use base64::STANDARD_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use log::info;
use oauth2::{
    basic::BasicErrorResponseType, basic::BasicTokenType, AuthorizationCode, Client, CsrfToken,
    EmptyExtraTokenFields, Scope, StandardErrorResponse, StandardTokenResponse,
};
use oauth2::{reqwest::async_http_client, TokenResponse};
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
pub struct OauthCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct TgProfileResponse {
    data: TgProfile,
}

#[derive(Deserialize)]
struct JWKEntry {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct JWK {
    keys: Vec<JWKEntry>,
}

#[derive(Deserialize, Debug)]
struct DexClaim {
    sub: String,
}

fn decode_subject(subject: &str) -> Result<String> {
    let decoded = base64::decode_config(subject, STANDARD_NO_PAD)?;
    if decoded.len() < 3 {
        return Err(anyhow!("Subject field is too short"));
    }
    let len = decoded[1] as usize;
    if len + 2 > decoded.len() {
        return Err(anyhow!("Subject field contains invalid length specifier"));
    }
    let subject = decoded[2..(len + 2)].to_owned();

    Ok(String::from_utf8(subject)?)
}

async fn validate_jwt_token(token: &str) -> Result<String> {
    let jwk_url = std::env::var("OAUTH_JWK_URL")?;
    let header = decode_header(token)?;
    let kid = header.kid.ok_or(anyhow!("`kid` is missing from header"))?;
    let mut resp = awcClient::default()
        .get(jwk_url)
        .send()
        .await
        .map_err(|_| anyhow!("Failed to send JWK request"))?;
    let jwk = resp.json::<JWK>().await?;
    for key in jwk.keys {
        if key.kid == kid {
            let claims = decode::<DexClaim>(
                token,
                &DecodingKey::from_rsa_components(&key.n, &key.e),
                &Validation::new(Algorithm::RS256),
            )?;
            info!("OAuth2 token verified");
            let username = decode_subject(&claims.claims.sub)?;
            return Ok(username);
        }
    }

    Err(anyhow!("Failed to verify token"))
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

#[get("/oauth/aosc")]
pub async fn oauth_aosc(
    pool: web::Data<PgPool>,
    id: Identity,
    query: web::Query<OauthCallback>,
    session: Session,
    oauth: web::Data<
        Client<
            StandardErrorResponse<BasicErrorResponseType>,
            StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
            BasicTokenType,
        >,
    >,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id.identity() {
        if let Some(csrf_token) = session.get::<String>("aosc")? {
            info!("OAuth2 challenge received");
            if query.state != csrf_token {
                info!("OAuth2 challenge failed: CSRF token mismatch.");
                return Ok(BAD_REQUEST!());
            }
            let token = oauth
                .exchange_code(AuthorizationCode::new(query.code.clone()))
                .request_async(async_http_client)
                .await
                .map_err(|_| {
                    HttpResponse::Unauthorized()
                        .header(http::header::CONTENT_TYPE, "text/html")
                        .finish()
                })?;
            info!("OAuth2 challenge verified by idp");
            let name = validate_jwt_token(token.access_token().secret())
                .await
                .map_err(|_| BAD_REQUEST!())?;
            let user = db::get_user_by_username(pool.as_ref(), &id)
                .await
                .map_err(|_| INTERNAL_ERROR!())?;
            db::add_oauth_info(
                pool.as_ref(),
                Oauth {
                    uid: user.id,
                    type_: "AOSC".to_string(),
                    oid: Some(name),
                    token: None,
                },
            )
            .await
            .map_err(|_| INTERNAL_ERROR!())?;
            info!("OAuth2 account added: {}", id);
            return Ok(HttpResponse::Found()
                .header(http::header::LOCATION, "/account")
                .finish());
        }
    }

    Ok(BAD_REQUEST!())
}

#[get("/oauth/aosc/new")]
pub async fn oauth_aosc_new(
    id: Identity,
    session: Session,
    oauth: web::Data<
        Client<
            StandardErrorResponse<BasicErrorResponseType>,
            StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
            BasicTokenType,
        >,
    >,
) -> Result<HttpResponse, Error> {
    if id.identity().is_none() {
        return Ok(BAD_REQUEST!());
    }
    let (auth_url, csrf_token) = oauth
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("openid".to_string()))
        .url();
    session.set("aosc", csrf_token.secret())?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, auth_url.to_string())
        .finish())
}
