use askama::Template;
use axum::response::{Html, IntoResponse};
use crate::configuration::QR_CODE_SIZE;

#[derive(Template)]
#[template(path = "homepage.html")]
pub struct Homepage {
    embed_image_url: &'static str,
    qr_size: usize,
}

pub async fn show_form() -> impl IntoResponse {
    Html(Homepage{ embed_image_url: "/graphics/favicon.ico", qr_size: QR_CODE_SIZE }.render().unwrap())
}
