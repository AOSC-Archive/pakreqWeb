use crate::{auth, db};
use actix_web::{web, Error};
use actix_web::{http, HttpRequest, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::path::{Iter, PathBuf};
use sqlx::PgPool;

pub const BAD_REQUEST_RETURN: &'static str = r#"{"success": false, "message": "Bad Request"}"#;
pub const INTERNAL_ERR_RESPONSE: &'static str = r#"{"success": false, "message": "Internal error"}"#;
pub const NOT_AUTHORIZED_RESPONSE: &'static str = r#"{"success": false, "message": "Not authorized"}"#;

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    sub: String,
    nbf: DateTime<Utc>,
    exp: DateTime<Utc>,
    iat: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    success: bool,
    token: String,
}

#[macro_export]
macro_rules! BAD_REQUEST {
    () => {
        HttpResponse::BadRequest()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(BAD_REQUEST_RETURN);
    };
}

#[macro_export]
macro_rules! INTERNAL_ERROR {
    () => {
        HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(INTERNAL_ERR_RESPONSE);
    };
}

#[macro_export]
macro_rules! NOT_AUTHORIZED {
    () => {
        HttpResponse::Unauthorized()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(NOT_AUTHORIZED_RESPONSE);
    };
}

#[macro_export]
macro_rules! OK {
    ($r:ident) => {
        HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body($r);
    };
}

pub async fn rest_dispatch(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let path = req.match_info().query("endpoint").parse::<PathBuf>();
    if path.is_err() {
        return Ok(BAD_REQUEST!());
    }
    let path = path.unwrap();
    let mut components = path.iter();
    if let Some(component) = components.next() {
        return {
            match &*component.to_string_lossy() {
                "requests" => rest_requests(pool, components).await,
                "request" => rest_request_detail(pool, components).await,
                "login" => rest_login(pool, &req).await,
                _ => Ok(BAD_REQUEST!()),
            }
        };
    }

    Ok(BAD_REQUEST!())
}

#[inline]
async fn rest_request_detail(
    pool: web::Data<PgPool>,
    mut components: Iter<'_>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get_ref();

    let request_id = components.next();
    if let Some(request_id) = request_id {
        let request_id =
            str::parse::<i64>(&request_id.to_string_lossy()).map_err(|_| INTERNAL_ERROR!())?;
        let detail = db::get_request_detail_by_id(&conn, request_id)
            .await
            .map_err(|_| INTERNAL_ERROR!())?;
        let result = to_string(&detail).map_err(|_| INTERNAL_ERROR!())?;
        return Ok(OK!(result));
    }

    Ok(BAD_REQUEST!())
}

#[inline]
async fn rest_requests(
    pool: web::Data<PgPool>,
    _components: Iter<'_>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get_ref();
    let requests = db::get_open_requests_json(&conn)
        .await
        .map_err(|_| INTERNAL_ERROR!())?;

    Ok(OK!(requests))
}

#[inline]
async fn rest_login(pool: web::Data<PgPool>, req: &HttpRequest) -> Result<HttpResponse, Error> {
    let headers = req.headers();
    let username = headers.get("x-username");
    let password = headers.get("x-password");
    let username_str: &str;
    if let Some(username) = username {
        username_str = username.to_str().map_err(|_| NOT_AUTHORIZED!())?;
    } else {
        return Ok(NOT_AUTHORIZED!());
    }
    if let Some(password) = password {
        let password = password.to_str().map_err(|_| NOT_AUTHORIZED!())?;
        let is_valid_password = auth::check_password(pool, username_str.to_string(), password)
            .await
            .map_err(|_| INTERNAL_ERROR!())?;
        if is_valid_password {
            let token = issue_jwt_token(username_str)
                .await
                .map_err(|_| INTERNAL_ERROR!())?;
            let result = to_string(&TokenResponse {
                success: true,
                token,
            })
            .map_err(|_| INTERNAL_ERROR!())?;
            return Ok(OK!(result));
        }
    }

    Ok(NOT_AUTHORIZED!())
}

// utility functions
#[inline]
async fn issue_jwt_token(username: &str) -> Result<String, Error> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");
    let expiration = Duration::days(1);
    let claim = UserClaims {
        iat: Utc::now(),
        nbf: Utc::now(),
        sub: username.to_owned(),
        exp: Utc::now() + expiration,
    };
    let token = web::block(move || {
        encode(
            &Header::default(),
            &claim,
            &EncodingKey::from_secret(secret.as_ref()),
        )
    })
    .await?;

    Ok(token)
}

async fn validate_jwt_token(token: String) -> Result<String, Error> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");
    let token_data = web::block(move || {
        decode::<UserClaims>(
            &token,
            &DecodingKey::from_secret(secret.as_ref()),
            &Validation::default(),
        )
    })
    .await?;

    Ok(token_data.claims.sub)
}
