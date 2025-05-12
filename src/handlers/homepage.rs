use crate::configuration::{EMBED_DESCRIPTION, FAVICON, QR_CODE_SIZE};
use askama::Template;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "homepage.html")]
pub struct Homepage {
    embed_image_url: &'static str,
    embed_description: &'static str,
    qr_size: usize,
}

pub async fn show_form() -> impl IntoResponse {
    Html(
        Homepage {
            embed_image_url: FAVICON,
            embed_description: EMBED_DESCRIPTION,
            qr_size: QR_CODE_SIZE,
        }
        .render()
        .unwrap(),
    )
}
