use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use qrcode_generator::QrCodeEcc;
use wasm_bindgen::prelude::wasm_bindgen;

// TODO: This was taken from tapfer-src/handlers/qrcode.rs. Maybe dedup this code in the future
pub const QR_CODE_SIZE: usize = 200; // pixels
pub const QR_CODE_ECC: QrCodeEcc = QrCodeEcc::Medium;

#[wasm_bindgen]
pub fn qr_base64_from_url(origin: &str, uri: &str) -> Option<String> {
	let data = qrcode_generator::to_png_to_vec_from_str(
		format!("{origin}{uri}"),
		QR_CODE_ECC,
		QR_CODE_SIZE,
	).ok()?;
	Some(format!("data:image/png;base64, {}", BASE64_STANDARD.encode(&data)))
}