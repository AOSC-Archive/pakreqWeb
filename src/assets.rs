use actix_web::{get, Responder};

const STYLE_CSS: &'static str = include_str!("../static/style.css");
const LOGO_SVG: &'static str = include_str!("../static/aosc.svg");
const LOGO_PNG: &'static [u8] = include_bytes!("../static/aosc.png");

#[get("/static/style.css")]
pub async fn style_css() -> impl Responder {
    (STYLE_CSS).with_header("content-type", "text/css")
}

#[get("/static/aosc.svg")]
pub async fn logo_svg() -> impl Responder {
    (LOGO_SVG).with_header("content-type", "image/svg+xml")
}

#[get("/static/aosc.png")]
pub async fn logo_png() -> impl Responder {
    (LOGO_PNG).with_header("content-type", "image/png")
}
