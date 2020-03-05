#[macro_use]
extern crate diesel;

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::HttpResponse;
use actix_web::{http, http::StatusCode, middleware, web, App, Error, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use dotenv;
use yarte::Template;

mod assets;
mod auth;
mod db;
mod models;
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

async fn details(pool: web::Data<DbPool>, base_url: String, path: web::Path<(i64,)>) -> Result<HttpResponse, Error> {
    let conn = pool.get().unwrap();

    let detail = web::block(move || db::get_request_detail_by_id(&conn, path.0))
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
    (NotFoundTemplate {}).with_status(StatusCode::NOT_FOUND)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    dotenv::dotenv().ok();
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let base_url = std::env::var("BASE_URL").expect("BASE_URL not set");
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Unable to establish database connections");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default()) // enable logger
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("identity")
                    .secure(false),
            ))
            .data(pool.clone())
            .data(base_url.clone())
            .route("/", web::get().to(index))
            .route("/detail/{id}", web::get().to(details))
            .route("/login", web::get().to(auth::login))
            .route("/login", web::post().to(auth::form_login))
            .route("/logout", web::get().to(auth::logout))
            .route("/account", web::get().to(auth::account_panel))
            .route("/account", web::post().to(auth::form_account))
            .route("/static/aosc.png", web::get().to(assets::logo_png))
            .route("/static/aosc.svg", web::get().to(assets::logo_svg))
            .route("/static/style.css", web::get().to(assets::style_css))
            .default_service(web::route().to(not_found))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
