use crate::{db, models, DbPool};
use actix_web::{error, web, Error, Responder};
use actix_web::{http, http::StatusCode, HttpRequest, HttpResponse};
use serde_json::to_string;
use std::path::{Iter, PathBuf};

const BAD_REQUEST_RETURN: &'static str = r#"{"success": false, "message": "Bad Request"}"#;
const INTERNAL_ERR_RESPONSE: &'static str = r#"{"success": false, "message": "Internal error"}"#;

macro_rules! BAD_REQUEST {
    () => {
        HttpResponse::BadRequest()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(BAD_REQUEST_RETURN);
    };
}

macro_rules! INTERNAL_ERROR {
    () => {
        HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(INTERNAL_ERR_RESPONSE);
    };
}

macro_rules! OK {
    ($r:ident) => {
        HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body($r);
    }
}

pub async fn rest_dispatch(pool: web::Data<DbPool>, req: HttpRequest) -> HttpResponse {
    let path = req.match_info().query("endpoint").parse::<PathBuf>();
    if path.is_err() {
        return BAD_REQUEST!();
    }
    let path = path.unwrap();
    let mut components = path.iter();
    if let Some(component) = components.next() {
        return {
            match &*component.to_string_lossy() {
                "requests" => rest_requests(pool, components).await,
                "request" => rest_request_detail(pool, components).await,
                _ => BAD_REQUEST!(),
            }
        };
    }

    BAD_REQUEST!()
}

#[inline]
async fn rest_request_detail(pool: web::Data<DbPool>, mut components: Iter<'_>) -> HttpResponse {
    let conn = pool.get().unwrap();

    let request_id = components.next();
    if let Some(request_id) = request_id {
        let request_id = str::parse::<i64>(&request_id.to_string_lossy());
        if let Ok(request_id) = request_id {
            let detail = web::block(move || db::get_request_detail_by_id(&conn, request_id)).await;
            if detail.is_err() {
                return INTERNAL_ERROR!();
            }
            let detail = detail.unwrap();
            if let Ok(result) = to_string(&detail) {
                return OK!(result);
            }
            return INTERNAL_ERROR!();
        }
    }

    BAD_REQUEST!()
}

#[inline]
async fn rest_requests(pool: web::Data<DbPool>, _components: Iter<'_>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let requests = web::block(move || db::get_open_requests(&conn)).await;
    if let Ok(requests) = requests {
        let result = to_string(&requests);
        if let Ok(result) = result {
            return OK!(result);
        }
    }

    INTERNAL_ERROR!()
}
