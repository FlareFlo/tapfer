use crate::UPLOAD_POOL;
use crate::error::TapferResult;
use crate::handlers::get_any_meta;
use crate::retention_control::delete_asset;
use crate::updown::upload_pool::UploadFsm;
use axum::extract::Path;
use axum::response::{IntoResponse, Redirect};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

pub async fn request_delete_asset(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((uuid, _), _) = get_any_meta(&path).await?;
    info!("Request to delete {uuid}");

    // Ensure the uploader (if present) fails the upload
    if let Some(handle) = UPLOAD_POOL.uploads.get(&uuid) {
        let v = handle.value();
        *v.write_fsm().await = UploadFsm::Failed;
        v.notify_all_downloaders();
        info!("Notified uploader and downloaders that {uuid} is slated for deletion");
        // Wait for all downloaders to abort and drop their resources gracefully
        sleep(Duration::from_millis(200)).await;
        info!("Aborted upload and downloads for {uuid} as requested");
    } else {
        let r = delete_asset(uuid).await;
        match r {
            Ok(_) => {
                info!("Deleted {uuid} as requested");
            }
            Err(e) => {
                error!("Failed to delete {uuid} from filesystem due to {e}");
            }
        }
    }

    Ok(Redirect::to("/"))
}
