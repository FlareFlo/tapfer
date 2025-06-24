use uuid::Uuid;
use tokio::fs;
use axum::http::StatusCode;
use axum::response::Html;
use std::str::FromStr;
use askama::Template;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::FileMeta;
use crate::handlers::download::UpDownFsm;
use crate::handlers::not_found::NotFound;
use crate::UPLOAD_POOL;

pub mod delete;
pub mod download;
pub mod homepage;
mod not_found;
pub mod qrcode;
pub mod upload;

async fn get_any_meta(path: &String) -> TapferResult<((Uuid, FileMeta), UpDownFsm)> {
    let uuid = Uuid::from_str(path)?;
    let res = match fs::try_exists(&format!("data/{uuid}/meta.toml")).await.ok() {
        // Regular download
        Some(true) => (
            FileMeta::read_from_uuid_path(&path).await?,
            UpDownFsm::Completed,
        ),
        // In-progress upload or doesnt exist
        _ => {
            let uuid = Uuid::from_str(path)?;
            match UPLOAD_POOL.uploads.get(&uuid) {
                // The upload is not in progress either, so it does not exist
                None => {
                    return Err(TapferError::Custom {
                        status_code: StatusCode::NOT_FOUND,
                        body: Html(NotFound::default().render()?),
                    });
                }
                // The upload is in-progress
                Some(handle) => {
                    let fsm = if UPLOAD_POOL.uploads.contains_key(&uuid) {
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