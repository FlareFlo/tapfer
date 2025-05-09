use crate::error::{TapferResult};
use axum::body::Body;
use axum::extract::Path;
use axum::http;
use axum::response::IntoResponse;
use qrcode_generator::QrCodeEcc;
use uuid::Uuid;

pub async fn get_qrcode_from_uuid(
    uri: http::Uri,
    Path(path): Path<String>,
) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::parse_str(&path)?;
    let host = uri.host();
    let method = if host.is_some() { "https://" } else { "" };
    let qrc = qrcode_generator::to_png_to_vec(
        format!(
            "{}{}/uploads/{uuid}",
            method,
            host.unwrap_or("localhost:3000"),
        )
        .as_bytes(),
        QrCodeEcc::Medium,
        200,
    )?;
    println!("{}", qrc.len());
    Ok((Body::from(qrc)))
}
