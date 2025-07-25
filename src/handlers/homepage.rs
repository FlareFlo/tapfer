use crate::configuration::{EMBED_DESCRIPTION, FAVICON, QR_CODE_SIZE};
use askama::Template;
use axum::response::{Html, IntoResponse};
use crate::error::TapferResult;
use crate::handlers::qrcode::random_base64_qr_from_id;

#[derive(Template)]
#[template(path = "homepage.html")]
pub struct Homepage {
    embed_image_url: &'static str,
    embed_description: &'static str,
    qr_size: usize,
    qr_b64: String,
}

pub async fn show_form() -> TapferResult<impl IntoResponse> {
    Ok(Html(
        Homepage {
            embed_image_url: FAVICON,
            embed_description: EMBED_DESCRIPTION,
            qr_size: QR_CODE_SIZE,
            qr_b64: random_base64_qr_from_id()?,
        }
            .render()
            .unwrap(),
    ))
}
