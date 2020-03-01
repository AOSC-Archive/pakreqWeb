use actix_web::Responder;

const STYLE_CSS: &'static str = include_str!("../static/style.css");
const LOGO_SVG: &'static str = include_str!("../static/aosc.svg");
const LOGO_PNG: &'static [u8] = include_bytes!("../static/aosc.png");

pub async fn style_css() -> impl Responder {
    (STYLE_CSS).with_header("content-type", "text/css")
}

pub async fn logo_svg() -> impl Responder {
    (LOGO_SVG).with_header("content-type", "image/svg+xml")
}

pub async fn logo_png() -> impl Responder {
    (LOGO_PNG).with_header("content-type", "image/png")
}
