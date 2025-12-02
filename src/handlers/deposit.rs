use crate::configuration::{EMBED_DESCRIPTION, FAVICON, QR_CODE_ECC, QR_CODE_SIZE};
use crate::error::TapferResult;
use askama::Template;
use axum::extract::{Path, Query, WebSocketUpgrade};
use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::Host;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use uuid::Uuid;
use crate::tapfer_id::TapferId;
use crate::websocket::wss_method;

#[derive(Template)]
#[template(path = "deposit.html")]
pub struct Deposit {
	embed_image_url: &'static str,
	embed_description: &'static str,
	qr_size: usize,
	qr_b64: String,
	ws_url: String,
}

pub async fn show_form(Host(host): Host) -> TapferResult<impl IntoResponse> {
	let deposit_id = Uuid::new_v4().as_u64_pair().0; // Hacky? Sure. But this avoids another RNG library that we use once

	let qr_code = qrcode_generator::to_png_to_vec_from_str(
		format!("https://{host}?deposit={deposit_id}",),
		QR_CODE_ECC,
		QR_CODE_SIZE,
	)?;

	let template = Deposit {
		embed_image_url: FAVICON,
		embed_description: EMBED_DESCRIPTION,
		qr_size: QR_CODE_SIZE,
		qr_b64: BASE64_STANDARD.encode(&qr_code),
		ws_url: format!("{}://{host}/deposit/ws?deposit={deposit_id}", wss_method(&host)),
	};

	Ok(Html(template.render()?))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Param {
	deposit: u64,
}

#[axum::debug_handler]
pub async fn start_ws(query: Query<Param>, ws: WebSocketUpgrade) -> Response {
	ws.on_upgrade(move |socket| crate::websocket::handle_socket(socket, query.deposit))
}