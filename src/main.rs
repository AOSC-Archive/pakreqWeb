#[macro_use]
extern crate diesel;

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::HttpResponse;
use actix_web::{http, http::StatusCode, middleware, web, App, Error, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use dotenv;
use log::info;
use std::time::Duration;
use yarte::Template;

mod assets;
mod auth;
mod db;
mod models;
mod oauth;
mod rest;
mod schema;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Template)]
#[template(path = "404.hbs")]
struct NotFoundTemplate {}

#[derive(Template)]
#[template(path = "500.hbs")]
struct InternalErrorTemplate {}

#[derive(Template)]
#[template(path = "index.hbs")]
struct IndexTemplate {
    base_url: String,
    requests: Vec<models::Request>,
    banner_subtitle: String,
}

#[derive(Template)]
#[template(path = "details.hbs")]
struct DetailsTemplate {
    base_url: String,
    request: models::RequestStr,
    title: String,
    banner_title: String,
}

async fn details(
    pool: web::Data<DbPool>,
    base_url: String,
    path: web::Path<(i64,)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().unwrap();

    let detail = web::block(move || db::get_request_detail_by_id(&conn, (path.0).0))
        .await
        .map_err(|_| HttpResponse::InternalServerError().finish())?;
    let request_name = detail.name.clone();
    let response = DetailsTemplate {
        base_url,
        request: detail,
        banner_title: request_name.clone(),
        title: format!("{} - AOSC OS Package Requests", request_name),
    };
    let res = HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(
            response
                .call()
                .unwrap_or("Internal Server Error".to_string()),
        );
    Ok(res)
}

async fn ping(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let conn = pool.get();
    let res;
    if conn.is_ok() {
        res = HttpResponse::NoContent().finish();
    } else {
        res = HttpResponse::InternalServerError().finish();
    }
    Ok(res)
}

async fn index(pool: web::Data<DbPool>, base_url: String) -> Result<HttpResponse, Error> {
    let conn = pool.get().unwrap();

    let requests: Vec<models::Request> = web::block(move || db::get_open_requests(&conn))
        .await
        .map_err(|_| HttpResponse::InternalServerError().finish())?;
    let count = requests.len();
    let response = IndexTemplate {
        base_url,
        requests,
        banner_subtitle: format!("{} pending requests in total", count),
    };
    let res = HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(
            response
                .call()
                .unwrap_or("Internal Server Error".to_string()),
        );
    Ok(res)
}

async fn not_found() -> impl Responder {
    HttpResponse::NotFound()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(
            (NotFoundTemplate {})
                .call()
                .unwrap_or("Internal Server Error".to_string()),
        )
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    dotenv::dotenv().ok();
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let listen = std::env::var("LISTEN_ADDRESS").expect("LISTEN_ADDRESS not set");
    let base_url = std::env::var("BASE_URL").expect("BASE_URL not set");
    std::env::var("JWT_SECRET").expect("JWT_SECRET not set"); // will be used later
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .connection_timeout(Duration::from_secs(10))
        .build(manager)
        .expect("Unable to establish database connections");
    info!("Database connection established.");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default()) // enable logger
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("identity")
                    .secure(false),
            ))
            .wrap(middleware::NormalizePath::default())
            .data(pool.clone())
            .data(base_url.clone())
            // traditional pages
            .route("/", web::get().to(index))
            .route("/", web::head().to(ping))
            .route("/detail/{id}", web::get().to(details))
            .route("/login", web::get().to(auth::login))
            .route("/login", web::post().to(auth::form_login))
            .route("/logout", web::get().to(auth::logout))
            .route("/account", web::get().to(auth::account_panel))
            .route("/account", web::post().to(auth::form_account))
            // static files
            .route("/static/aosc.png", web::get().to(assets::logo_png))
            .route("/static/aosc.svg", web::get().to(assets::logo_svg))
            .route("/static/style.css", web::get().to(assets::style_css))
            // RESTful APIs
            .route("/api/{endpoint:.*}", web::get().to(rest::rest_dispatch))
            // OAuth handlers
            .route("/oauth/telegram", web::post().to(oauth::oauth_telegram))
            .route("/oauth/aosc", web::post().to(oauth::oauth_aosc))
            .default_service(web::route().to(not_found))
    })
    .bind(listen)?
    .run()
    .await
}
