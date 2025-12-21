use crate::error::TapferResult;
use crate::file_meta::FileMeta;
use crate::handlers::get_any_meta;
use crate::tapfer_id::TapferId;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::Host;
use dashmap::DashSet;
use http::StatusCode;
use scopeguard::defer;
use sha2::Digest;
use std::fs::File;
use std::io::BufReader;
use std::sync::LazyLock;
use std::{fs, io, thread};
use tracing::{error, info};

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
    Host(_host): Host,
) -> TapferResult<impl IntoResponse> {
    let ((id, _), _) = get_any_meta(&path).await?;
    if let Some(chksum) = get_sha512_for_asset(id)? {
        Ok(Response::builder().body(chksum)?)
    } else {
        Ok(Response::builder()
            .status(StatusCode::PROCESSING)
            .body("Checksum computation is in progress".to_owned())?)
    }
}

pub fn get_sha512_for_asset(id: TapferId) -> TapferResult<Option<String>> {
    let precomputed = fs::read_to_string(format!("data/{id}/checksum.sha512"));

    Ok(precomputed.ok())
}

static ACTIVE_CHECKSUMS: LazyLock<DashSet<TapferId>> = LazyLock::new(|| DashSet::new());

pub fn spawn_sha512_checksum(id: TapferId) {
    let core = move || {
        let meta = FileMeta::read_from_id_blocking(id)?;
        let mut asset = BufReader::with_capacity(
            2_usize.pow(24),
            File::open(format!("data/{id}/{}", meta.name()))?,
        );
        let mut h = sha2::Sha512::new();
        let _ = io::copy(&mut asset, &mut h)?;
        fs::write(
            format!("data/{id}/checksum.sha512"),
            base16ct::lower::encode_string(&h.finalize()),
        )?;
        Ok(())
    };
    if ACTIVE_CHECKSUMS.contains(&id) {
        // Do not spawn another active thread
        return;
    }
    // Ignore handle
    let _ = thread::Builder::new()
        .name(format!("hasher_{id}"))
        .spawn(move || {
            ACTIVE_CHECKSUMS.insert(id);
            // Defer also runs on panic - so the map isn't poisoned when the hashing thread panics
            defer!(if ACTIVE_CHECKSUMS.remove(&id).is_none() {
                error!("Checksum of {id} not found in ACTIVE_CHECKSUMS");
            });
            let res: TapferResult<()> = core();
            if let Err(e) = res {
                error!("Failed to checksum {id} because of: {e}");
            } else {
                info!("Computed sha512 for {id}");
            }
        });
}
