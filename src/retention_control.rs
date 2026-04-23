pub(crate) use crate::GLOBAL_RETENTION_POLICY;
use crate::structs::error::{TapferErrorExt, TapferResult};
use crate::structs::file_meta::FileMeta;
use crate::structs::tapfer_id::TapferId;
use crate::websocket::WsEvent;
use crate::{UPLOAD_POOL, websocket};
use std::ops::{Add, Not};
use std::str::FromStr;
use time::{Duration, UtcDateTime};
use tokio::fs;
use tokio::fs::remove_dir_all;
use tracing::{info};

pub struct GlobalRetentionPolicy {
    pub maximum_age: Duration,
    pub recheck_interval: Duration,
}

impl Default for GlobalRetentionPolicy {
    fn default() -> Self {
        Self {
            maximum_age: Duration::hours(24),
            recheck_interval: Duration::seconds(60),
        }
    }
}

pub async fn check_against_global_retention(
    (id, meta): (TapferId, FileMeta),
    now: UtcDateTime,
) -> TapferResult<()> {
    if meta.created().add(GLOBAL_RETENTION_POLICY.maximum_age) < now {
        info!("Deleting {id} as it has expired");
        delete_asset(id).await?;
    }
    Ok(())
}

pub async fn delete_asset(asset: TapferId) -> TapferResult<()> {
    websocket::broadcast_event(asset, WsEvent::DeleteAsset)
        .log_error("Failed to broadcast deletion event");
    fs::remove_dir_all(format!("data/{asset}")).await?;
    Ok(())
}

pub async fn check_all_assets() -> TapferResult<()> {
    let now = UtcDateTime::now();
    let mut dir = fs::read_dir("data").await?;
    while let Some(entry) =  dir.next_entry().await? {
        let file_meta = match entry.metadata().await {
            Ok(m) => m,
            e => {
                e.log_error("Failed to get metadata");
                continue;
            }
        };
        // Skip cachedir tag
        if file_meta.is_dir().not() {
            continue;
        }
        let mut path = entry.path().to_path_buf();
        path.push("meta.toml");
        let id = match TapferId::from_str(&entry.file_name().to_string_lossy()) {
            Ok(t) => {t}
            e => {e.log_error(&format!("Failed get ID from {}", path.display())); continue}
        };

        if let Ok(meta) = FileMeta::read_from_id(id).await {
            check_against_global_retention((id, meta), now).await?;
        } else {
            // Only delete element if it isn't in progress
            if !UPLOAD_POOL.uploads.contains_key(&id) {
                // Delete asset straight up when metadata is missing or corrupt
                info!(
                    "Deleting {} as its metadata appears corrupt",
                    entry.path().as_os_str().to_string_lossy()
                );
                remove_dir_all(entry.path()).await?;
            }
        }
    }
    Ok(())
}
