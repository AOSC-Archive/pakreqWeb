use crate::{db, models::Oauth};
use actix_identity::Identity;
use actix_web::{get, post, web, Error};
use actix_web::{http, HttpResponse};
use argonautica;
use serde::Deserialize;
use sqlx::PgPool;
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
    oauth: Vec<Oauth>,
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

#[get("/login")]
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
    pool: web::Data<PgPool>,
    username: String,
    pwd: &str,
) -> Result<bool, Error> {
    let conn = pool.get_ref();
    let user = db::get_user_by_username(&conn, &username)
        .await
        .map_err(|_| HttpResponse::BadRequest().body("Internal Server Error"))?;
    let mut verifier = argonautica::Verifier::default();
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

    Ok(false)
}

pub async fn hash_password(
    pool: web::Data<PgPool>,
    username: String,
    password: &str,
) -> Result<String, Error> {
    let mut hasher = argonautica::Hasher::default();
    let conn = pool.get_ref();
    hasher
        .configure_iterations(8)
        .configure_memory_size(65536)
        // Not supported by pakreqBot
        .opt_out_of_secret_key(true);
    let user = db::get_user_by_username(&conn, &username)
        .await
        .map_err(|_| HttpResponse::BadRequest().body("Internal Server Error"))?;
    let encoded = format!("{}:{}", user.id, password);
    let result = web::block(move || hasher.with_password(encoded).hash()).await?;

    Ok(result)
}

#[post("/login")]
pub async fn form_login(
    id: Identity,
    form: web::Form<LoginForm>,
    pool: web::Data<PgPool>,
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

#[get("/account")]
pub async fn account_panel(id: Identity, base_url: String, pool: web::Data<PgPool>,) -> Result<HttpResponse, Error> {
    if let Some(id) = id.identity() {
        let oauth = db::get_oauth_by_username(pool.get_ref(), &id).await.unwrap_or(vec![]);
        let template = PanelTemplate {
            base_url,
            banner_subtitle: format!("Settings for {}", id),
            msg: "".to_owned(),
            oauth,
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

#[post("/account")]
pub async fn form_account(
    id: Identity,
    form: web::Form<AccountForm>,
    pool: web::Data<PgPool>,
    base_url: String,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id.identity() {
        let oauth = db::get_oauth_by_username(pool.get_ref(), &id).await.unwrap_or(vec![]);
        if form.new_password != form.repeat_password {
            let template = PanelTemplate {
                base_url,
                oauth,
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
        let mut template = PanelTemplate {
            oauth,
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
            let conn = pool.get_ref();
            db::update_password_hash(&conn, id.clone(), password_hash)
                .await
                .map_err(|_| HttpResponse::BadRequest().body("Internal Server Error"))?;
            template.msg = "Password changed successfully".to_owned();
            return Ok(HttpResponse::InternalServerError().body(
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

#[get("/logout")]
pub async fn logout(id: Identity) -> HttpResponse {
    if let Some(_) = id.identity() {
        id.forget();
    }

    HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish()
}
