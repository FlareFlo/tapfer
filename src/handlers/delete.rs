use crate::UPLOAD_POOL;
use crate::error::TapferResult;
use crate::handlers::get_any_meta;
use crate::retention_control::delete_asset;
use crate::updown::upload_pool::UploadFsm;
use crate::websocket;
use crate::websocket::WsEvent;
use axum::extract::Path;
use axum::response::{IntoResponse, Redirect};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

#[utoipa::path(
    get,
    path = "/uploads/{id}",
    responses(
        (status = 303, description = "Asset deleted, redirects to home page"),
        (status = 404, description = "Asset does not exist"),
    ),
)]
pub async fn request_delete_asset(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((id, _), _) = get_any_meta(&path).await?;
    info!("Request to delete {id}");

    // Ensure the uploader (if present) fails the upload
    if let Some(handle) = UPLOAD_POOL.uploads.get(&id) {
        let v = handle.value();
        *v.write_fsm().await = UploadFsm::Failed;
        v.notify_all_downloaders();
        info!("Notified uploader and downloaders that {id} is slated for deletion");
        // Wait for all downloaders to abort and drop their resources gracefully
        sleep(Duration::from_millis(200)).await;
        info!("Aborted upload and downloads for {id} as requested");
    } else {
        let r = delete_asset(id).await;
        match r {
            Ok(_) => {
                info!("Deleted {id} as requested");
            }
            Err(e) => {
                error!("Failed to delete {id} from filesystem due to {e}");
            }
        }
    }

    Ok(Redirect::to("/"))
}
