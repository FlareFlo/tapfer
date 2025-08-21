use crate::configuration::{QR_CODE_ECC, QR_CODE_SIZE};
use crate::error::TapferResult;
use crate::handlers::get_any_meta;
use crate::tapfer_id::TapferId;
use axum::body::Body;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum_extra::extract::Host;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use qrcode_generator::QrCodeEcc;
use std::iter::{once, repeat};

fn qr_from_id(id: TapferId, host: &str) -> TapferResult<Vec<u8>> {
    let qrc = qrcode_generator::to_png_to_vec_from_str(
        // Uppercase such that this falls into the Alphanumeric encoding for higher efficiency
        // https://en.wikipedia.org/wiki/QR_code
        format!("{host}/uploads/{id}",).to_ascii_uppercase(),
        QR_CODE_ECC,
        QR_CODE_SIZE,
    )?;
    Ok(qrc)
}

pub fn base64_qr_from_id(id: TapferId, host: &str) -> TapferResult<String> {
    let data = qr_from_id(id, host)?;
    Ok(BASE64_STANDARD.encode(&data))
}

pub fn random_base64_qr_from_id(host: &str) -> TapferResult<String> {
    base64_qr_from_id(TapferId::new_random(), &host)
}

#[allow(dead_code)]
pub fn tiny_qr_from_id(id: TapferId, host: &str) -> TapferResult<String> {
    let mut qrc = qrcode_generator::to_matrix_from_str(
        // Uppercase such that this falls into the Alphanumeric encoding for higher efficiency
        // https://en.wikipedia.org/wiki/QR_code
        format!("{host}/uploads/{id}",).to_ascii_uppercase(),
        QrCodeEcc::Low,
    )?;
    let full = '█';
    // Not inverted
    let mut top_border = vec![vec![false; qrc.len()]];
    top_border.append(&mut qrc);

    let s = top_border
        .chunks(2)
        .map(|e| TryInto::<&[Vec<bool>; 2]>::try_into(e))
        // Not the best idea alright, but it avoids allocations
        .filter_map(Result::ok)
        .map(|[top, bottom]| {
            once((&false, &false)) // Left border
                .chain(top.iter().zip(bottom))
                .map(|(&top, &bottom)| {
                    match (!top, !bottom) {
                        // Invert colors such that on means black
                        (true, true) => full,  // full block U+2588
                        (true, false) => '▀',  // upper half block U+2580
                        (false, true) => '▄',  // lower half block U+2584
                        (false, false) => ' ', // space
                    }
                })
                .chain(once(full)) // Right border
                .chain(once('\n'))
                .collect::<String>()
        })
        .chain(once(repeat(full).take(top_border.len() + 1).collect())) // Bottom border
        .collect();
    Ok(s)
}

#[utoipa::path(
    get,
    path = "/qrcg/{id}",
    responses(
        (status = 200, description = "Returns QR code"),
        (status = 404, description = "Asset does not exist"),
    ),
)]
pub async fn get_qrcode_from_id(
    Path(path): Path<String>,
    Host(host): Host,
) -> TapferResult<impl IntoResponse> {
    let ((id, _), _) = get_any_meta(&path).await?;
    let qrc = qr_from_id(id, &host)?;
    Ok(Body::from(qrc))
}
