use std::{fs, thread};
use std::fs::File;
use std::io::{Read};
use axum::extract::Path;
use axum::response::IntoResponse;
use axum_extra::extract::Host;
use futures::executor::block_on;
use sha2::Digest;
use tracing::error;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::FileMeta;
use crate::handlers::get_any_meta;
use crate::tapfer_id::TapferId;

#[utoipa::path(
	get,
	path = "/uploads/{id}/checksum.sha512",
	responses(
        (status = 200, description = "Returns checksum"),
        (status = 404, description = "Asset does not exist"),
	),
)]
pub async fn get_sha512sum(
	Path(path): Path<String>,
	Host(host): Host,
) -> TapferResult<impl IntoResponse> {
	let ((id, _), _) = get_any_meta(&path).await?;
	Ok(get_checksum_for_asset(id).map_err(todo!("Add 404 here")))
}

pub fn get_checksum_for_asset(id: TapferId) -> TapferResult<Option<String>> {
	let precomputed = fs::read_to_string(format!("data/{id}/checksum.sha512"));

	Ok(precomputed.ok())
}

pub fn spawn_sha512_checksum(id: TapferId) {
	let core = move || {
		let meta = block_on(FileMeta::read_from_id(id))?;
		let mut asset = File::open(format!("data/{id}/{}", meta.name()))?;
		let mut h = sha2::Sha512::new();
		let mut buf = vec![0_u8; 2_usize.pow(30)]; // 1 GB at a time
		while let read = asset.read(&mut buf)? {
			Digest::update(&mut h, &buf[..read]);
		}
		fs::write(format!("data/{id}/checksum.sha512"), h.finalize())?;
		Ok(())
	};
	// Ignore handle
	let _ = thread::Builder::new().name(format!("hasher_{id}")).spawn(move ||{
		let res: TapferResult<()> = core();
		if let Err(e) = res {
			error!("Failed to checksum {id} because of: {e}");
		}
	});
}