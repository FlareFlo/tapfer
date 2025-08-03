pub(crate) use crate::GLOBAL_RETENTION_POLICY;
use crate::UPLOAD_POOL;
use crate::error::TapferResult;
use crate::file_meta::FileMeta;
use crate::tapfer_id::TapferId;
use std::ops::Add;
use std::str::FromStr;
use time::{Duration, UtcDateTime};
use tokio::fs;
use tokio::fs::remove_dir_all;
use tracing::info;

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
    if meta.created().add(GLOBAL_RETENTION_POLICY.maximum_age) > now {
        info!("Deleting {id} as it has expired");
        delete_asset(id).await?;
    }
    Ok(())
}

pub async fn delete_asset(asset: TapferId) -> TapferResult<()> {
    fs::remove_dir_all(format!("data/{asset}")).await?;
    Ok(())
}

pub async fn check_all_assets() -> TapferResult<()> {
    let now = UtcDateTime::now();
    let mut dir = fs::read_dir("data").await?;
    #[allow(for_loops_over_fallibles)]
    for entry in dir.next_entry().await? {
        // Skip cachedir tag etc.
        if entry.path().is_file() {
            continue;
        }

        if let Ok(meta) = FileMeta::read_from_id_path(&entry.path()).await {
            check_against_global_retention(meta, now).await?;
        } else {
            let id = TapferId::from_str(&entry.path().to_string_lossy())?;

            // Only delete element if it isn't in progress
            if !UPLOAD_POOL.uploads.contains_key(&id) {
                // Delete asset straight up when metadata is missing or corrupt
                info!(
                    "Deleting {} as its metadata appears corrupt",
                    entry.path().as_os_str().to_string_lossy()
                );
                remove_dir_all(entry.path()).await?;
            }
        };
    }
    Ok(())
}
