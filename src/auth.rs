use crate::{db, DbPool};
use actix_identity::Identity;
use actix_web::{error, web, Error, Responder};
use actix_web::{http, http::StatusCode, HttpResponse};
use argonautica;
use diesel::PgConnection;
use serde::Deserialize;
use std::io::{Error as IoError, ErrorKind};
use yarte::Template;

#[derive(Template)]
#[template(path = "login.hbs")]
struct LoginTemplate {
    msg: String,
    base_url: String,
}

#[derive(Template)]
#[template(path = "account.hbs")]
struct PanelTemplate {
    base_url: String,
    banner_subtitle: String,
    msg: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    user: String,
    pwd: String,
}

#[derive(Deserialize)]
pub struct AccountForm {
    #[serde(rename = "cpwd")]
    current_password: String,
    #[serde(rename = "npwd")]
    new_password: String,
    #[serde(rename = "cnpwd")]
    repeat_password: String,
}

pub async fn login(id: Identity, base_url: String) -> Result<HttpResponse, Error> {
    if let Some(_id) = id.identity() {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/account")
            .finish());
    }
    let template = LoginTemplate {
        base_url,
        msg: "".to_owned(),
    };
    return Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(
            template
                .call()
                .unwrap_or("Internal Server Error".to_string()),
        ));
}

pub async fn check_password(
    pool: web::Data<DbPool>,
    username: String,
    pwd: &str,
) -> Result<bool, Error> {
    let conn = pool.get().unwrap();
    let user = web::block(move || db::get_user_by_username(&conn, &username)).await?;
    let mut verifier = argonautica::Verifier::default();
    if let Some(user) = user {
        let encoded_password = format!("{}:{}", user.id, pwd);
        if let Some(password_hash) = user.password_hash {
            let is_valid: bool = web::block(move || {
                verifier
                    .with_hash(password_hash)
                    .with_password(encoded_password)
                    .verify()
            })
            .await?;
            return Ok(is_valid);
        }
    }

    Ok(false)
}

pub async fn hash_password(
    pool: web::Data<DbPool>,
    username: String,
    password: &str,
) -> Result<String, Error> {
    let mut hasher = argonautica::Hasher::default();
    let conn = pool.get().map_err(|_| {})?;
    hasher
        .configure_iterations(8)
        .configure_memory_size(65536)
        // Not supported by pakreqBot
        .opt_out_of_secret_key(true);
    let user = web::block(move || db::get_user_by_username(&conn, &username)).await?;
    if let Some(user) = user {
        let encoded = format!("{}:{}", user.id, password);
        let result = web::block(move || hasher.with_password(encoded).hash()).await?;
        return Ok(result);
    }

    Err(IoError::new(ErrorKind::InvalidData, "Invalid data").into())
}

pub async fn form_login(
    id: Identity,
    form: web::Form<LoginForm>,
    pool: web::Data<DbPool>,
    base_url: String,
) -> Result<HttpResponse, Error> {
    let username = form.user.clone();
    let template = LoginTemplate {
        base_url,
        msg: "Invalid credentials".to_owned(),
    };
    let is_valid = check_password(pool, username.clone(), &form.pwd)
        .await
        .map_err(|_| {
            HttpResponse::Unauthorized()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(
                    template
                        .call()
                        .unwrap_or("Internal Server Error".to_string()),
                )
        })?;
    if is_valid {
        id.remember(username);
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/account")
            .finish());
    }

    Ok(HttpResponse::Unauthorized()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(
            template
                .call()
                .unwrap_or("Internal Server Error".to_string()),
        ))
}

pub async fn account_panel(id: Identity, base_url: String) -> Result<HttpResponse, Error> {
    if let Some(id) = id.identity() {
        let template = PanelTemplate {
            base_url,
            banner_subtitle: format!("Settings for {}", id),
            msg: "".to_owned(),
        };
        return Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(
                template
                    .call()
                    .unwrap_or("Internal Server Error".to_string()),
            ));
    }
    return Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/login")
        .finish());
}

pub async fn form_account(
    id: Identity,
    form: web::Form<AccountForm>,
    pool: web::Data<DbPool>,
    base_url: String,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id.identity() {
        if form.new_password != form.repeat_password {
            let template = PanelTemplate {
                base_url,
                banner_subtitle: format!("Settings for {}", id),
                msg: "New password and Confirm new password mismatch!".to_owned(),
            };
            return Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(
                    template
                        .call()
                        .unwrap_or("Internal Server Error".to_string()),
                ));
        }
        let template = PanelTemplate {
            base_url: base_url.clone(),
            banner_subtitle: format!("Settings for {}", id),
            msg: "Current password is incorrect!".to_owned(),
        };
        let is_password_correct = check_password(pool.clone(), id.clone(), &form.current_password)
            .await
            .map_err(|_| {
                HttpResponse::Unauthorized()
                    .header(http::header::CONTENT_TYPE, "text/html")
                    .body(
                        template
                            .call()
                            .unwrap_or("Internal Server Error".to_string()),
                    )
            })?;
        if is_password_correct {
            let password_hash = hash_password(pool.clone(), id.clone(), &form.new_password).await?;
            let conn = pool.get().map_err(|_| {})?;
            web::block(move || db::update_password_hash(&conn, id.clone(), password_hash)).await?;
            let template = PanelTemplate {
                base_url,
                banner_subtitle: format!("Settings"),
                msg: "Password changed successfully".to_owned(),
            };
            return Ok(HttpResponse::Ok().body(
                template
                    .call()
                    .unwrap_or("Internal Server Error".to_string()),
            ));
        }
        return Ok(HttpResponse::Unauthorized()
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(
                template
                    .call()
                    .unwrap_or("Internal Server Error".to_string()),
            ));
    }
    return Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/login")
        .finish());
}

pub async fn logout(id: Identity) -> HttpResponse {
    if let Some(_) = id.identity() {
        id.forget();
    }

    HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish()
}
