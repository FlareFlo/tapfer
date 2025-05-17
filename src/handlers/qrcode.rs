use crate::configuration::{QR_CODE_ECC, QR_CODE_SIZE};
use crate::error::TapferResult;
use axum::body::Body;
use axum::extract::Path;
use axum::response::IntoResponse;
use qrcode_generator::QrCodeEcc;
use std::env;
use std::iter::{once, repeat};
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
    let qrc = qrcode_generator::to_png_to_vec_from_str(
        // Uppercase such that this falls into the Alphanumeric encoding for higher efficiency
        // https://en.wikipedia.org/wiki/QR_code
        format!("{host}/uploads/{uuid}",).to_ascii_uppercase(),
        QR_CODE_ECC,
        QR_CODE_SIZE,
    )?;
    Ok(qrc)
}

#[allow(dead_code)]
pub fn tiny_qr_from_uuid(uuid: Uuid) -> TapferResult<String> {
    let host = env::var("HOST").expect("Should ok as main checks this var already");
    let mut qrc = qrcode_generator::to_matrix_from_str(
        // Uppercase such that this falls into the Alphanumeric encoding for higher efficiency
        // https://en.wikipedia.org/wiki/QR_code
        format!("{host}/uploads/{uuid}",).to_ascii_uppercase(),
        QrCodeEcc::Low,
    )?;
    let full = '█';
    // Not inverted
    let mut top_border = vec![vec![false; qrc.len()]];
    top_border.append(&mut qrc);

    let s = top_border
        .chunks(2)
        .map(|e| TryInto::<&[Vec<bool>; 2]>::try_into(e))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
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
