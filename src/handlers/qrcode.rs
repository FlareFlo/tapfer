use crate::error::TapferResult;
use axum::body::Body;
use axum::extract::Path;
use axum::http;
use axum::response::IntoResponse;
use qrcode_generator::QrCodeEcc;
use std::env;
use uuid::Uuid;
use crate::configuration::{QR_CODE_ECC, QR_CODE_SIZE};

pub async fn get_qrcode_from_uuid(
    uri: http::Uri,
    Path(path): Path<String>,
) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::parse_str(&path)?;
    let host = env::var("HOST").expect("Should ok as main checks this var already");
    let method = if host != "localhost" { "https://" } else { "" };
    let qrc = qrcode_generator::to_png_to_vec(
        format!("{}{}/uploads/{uuid}", method, host,).as_bytes(),
        QR_CODE_ECC,
        QR_CODE_SIZE,
    )?;
    Ok(Body::from(qrc))
}
