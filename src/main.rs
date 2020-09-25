use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_session::CookieSession;
use actix_web::HttpResponse;
use actix_web::{get, head, http, middleware, web, App, Error, HttpServer, Responder};
use dotenv;
use log::info;
use middleware::normalize::TrailingSlash;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use sqlx::PgPool;
use rand::RngCore;
use yarte::Template;

mod assets;
mod auth;
mod db;
mod models;
mod oauth;
mod rest;

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

#[get("/detail/{id}")]
async fn details(
    pool: web::Data<PgPool>,
    base_url: String,
    path: web::Path<(i64,)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get_ref();

    let detail = db::get_request_detail_by_id(&conn, (path.0).0)
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

#[head("/")]
async fn ping(pool: web::Data<PgPool>) -> Result<HttpResponse, Error> {
    pool.get_ref();
    Ok(HttpResponse::NoContent().finish())
}

#[get("/")]
async fn index(pool: web::Data<PgPool>, base_url: String) -> Result<HttpResponse, Error> {
    let conn = pool.get_ref();

    let requests: Vec<models::Request> = db::get_open_requests(&conn)
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    dotenv::dotenv().ok();
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let listen = std::env::var("LISTEN_ADDRESS").expect("LISTEN_ADDRESS not set");
    let base_url = std::env::var("BASE_URL").expect("BASE_URL not set");
    std::env::var("JWT_SECRET").expect("JWT_SECRET not set"); // will be used later
    std::env::var("OAUTH_JWK_URL").expect("OAUTH_JWK_URL not set"); // will be used later
    let oauth_client = std::env::var("OAUTH_CLIENT_ID").expect("OAUTH_CLIENT_ID not set");
    let oauth_secret = std::env::var("OAUTH_SECRET").expect("OAUTH_SECRET not set");
    let oauth_auth_url = std::env::var("OAUTH_AUTH_URL").expect("OAUTH_URL not set");
    let oauth_token_url = std::env::var("OAUTH_TOKEN_URL").expect("OAUTH_URL not set");
    let pool = PgPool::new(&connspec)
        .await
        .expect("Unable to connect to database.");
    info!("Database connection established.");
    let oauth = BasicClient::new(
        ClientId::new(oauth_client),
        Some(ClientSecret::new(oauth_secret)),
        AuthUrl::new(oauth_auth_url).expect("OAUTH_URL malformed"),
        Some(TokenUrl::new(oauth_token_url).unwrap()),
    )
    .set_redirect_url(RedirectUrl::new(format!("{}/oauth/aosc/", base_url)).unwrap());
    let mut rng = rand::thread_rng();
    let mut id_key: [u8; 32] = [0; 32];
    let mut csrf_key: [u8; 32] = [0; 32];
    rng.fill_bytes(&mut id_key);
    rng.fill_bytes(&mut csrf_key);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default()) // enable logger
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&id_key)
                    .name("identity")
                    .secure(true),
            ))
            .wrap(
                CookieSession::private(&csrf_key)
                    .name("csrf")
                    .path("/")
                    .secure(true))
            .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
            .data(pool.clone())
            .data(base_url.clone())
            .data(oauth.clone())
            // traditional pages
            .service(ping)
            .service(index)
            .service(details)
            .service(auth::login)
            .service(auth::form_login)
            .service(auth::logout)
            .service(auth::account_panel)
            .service(auth::form_account)
            // static files
            .service(assets::logo_png)
            .service(assets::logo_svg)
            .service(assets::style_css)
            // RESTful APIs
            .route("/api/{endpoint:.*}", web::get().to(rest::rest_dispatch))
            // OAuth handlers
            .route("/oauth/telegram", web::post().to(oauth::oauth_telegram))
            .service(oauth::oauth_aosc)
            .service(oauth::oauth_aosc_new)
            .service(oauth::oauth_aosc_unlink)
            .default_service(web::route().to(not_found))
    })
    .bind(listen)?
    .run()
    .await
}
