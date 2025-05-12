use crate::configuration::{QR_CODE_ECC, QR_CODE_SIZE};
use crate::error::TapferResult;
use axum::body::Body;
use axum::extract::Path;
use axum::http;
use axum::response::IntoResponse;
use qrcode_generator::QrCodeEcc;
use std::env;
use std::str::FromStr;
use uuid::Uuid;

pub async fn get_qrcode_from_uuid(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::parse_str(&path)?;
    let qrc = qr_from_uuid(uuid)?;
    Ok(Body::from(qrc))
}

pub async fn get_placeholder_qrcode() -> TapferResult<impl IntoResponse> {
    // Just any funny looking UUID, it doesn't really matter
    let qrc = qr_from_uuid(Uuid::new_v4())?;
    Ok(Body::from(qrc))
}

fn qr_from_uuid(uuid: Uuid) -> TapferResult<Vec<u8>> {
    let host = env::var("HOST").expect("Should ok as main checks this var already");
    let method = if host != "localhost" { "https://" } else { "" };
    let qrc = qrcode_generator::to_png_to_vec(
        format!("{}{}/uploads/{uuid}", method, host,).as_bytes(),
        QR_CODE_ECC,
        QR_CODE_SIZE,
    )?;
    Ok(qrc)
}
