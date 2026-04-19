use crate::UPLOAD_POOL;
use crate::handlers::download::UpDownFsm;
use crate::handlers::not_found::NotFound;
use crate::structs::error::{TapferError, TapferResult};
use crate::structs::file_meta::FileMeta;
use crate::structs::tapfer_id::TapferId;
use askama::Template;
use axum::http::StatusCode;
use axum::response::Html;
use std::str::FromStr;
use tokio::fs;

pub(crate) mod checksum;
pub mod delete;
pub mod deposit;
pub mod download;
pub mod homepage;
mod not_found;
pub mod qrcode;
pub mod upload;

async fn get_any_meta(path: &String) -> TapferResult<((TapferId, FileMeta), UpDownFsm)> {
    let id = TapferId::from_str(path)?;
    let res = match fs::try_exists(&format!("data/{id}/meta.toml")).await.ok() {
        // Regular download
        Some(true) => (
            FileMeta::read_from_id_path(&path).await?,
            UpDownFsm::Completed,
        ),
        // In-progress upload or doesnt exist
        _ => {
            let id = TapferId::from_str(path)?;
            match UPLOAD_POOL.uploads.get(&id) {
                // The upload is not in progress either, so it does not exist
                None => {
                    return Err(TapferError::Custom {
                        status_code: StatusCode::NOT_FOUND,
                        body: Html(NotFound::default().render()?),
                    });
                }
                // The upload is in-progress
                Some(handle) => {
                    let fsm = if UPLOAD_POOL.uploads.contains_key(&id) {
                        UpDownFsm::UpdownInProgress {
                            progress: 0,
                            handle: handle.clone(),
                        }
                    } else {
                        UpDownFsm::Completed
                    };
                    (
                        (*handle.key(), FileMeta::from_upload_handle(handle.value())),
                        fsm,
                    )
                }
            }
        }
    };
    Ok(res)
}

pub fn is_localhost(host: &str) -> bool {
    host.starts_with("localhost") || host.starts_with("127.0.0.1")
}
