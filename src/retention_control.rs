use crate::file_meta::FileMeta;
use std::ops::Add;
use std::sync::LazyLock;
use time::{Duration, UtcDateTime};
use tokio::fs;
use tokio::fs::remove_dir_all;
use tracing::info;
use uuid::Uuid;
use crate::error::TapferResult;

pub static GLOBAL_RETENTION_POLICY: LazyLock<GlobalRetentionPolicy> =
    LazyLock::new(|| GlobalRetentionPolicy::default());

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

pub async fn check_against_global_retention((uuid, meta): (Uuid, FileMeta), now: UtcDateTime) -> TapferResult<()> {
    if meta.created().add(GLOBAL_RETENTION_POLICY.maximum_age) > now {
        info!("Deleting {uuid} as it has expired");
        delete_asset(uuid).await?;
    }
    Ok(())
}

pub async fn delete_asset(asset: Uuid) -> TapferResult<()> {
    fs::remove_dir_all(format!("data/{asset}")).await?;
    Ok(())
}

pub async fn check_all_assets() -> TapferResult<()> {
    let now = UtcDateTime::now();
    let mut dir = fs::read_dir("data").await?;
    for entry in dir.next_entry().await? {
        // Skip cachedir tag etc.
        if entry.path().is_file() {
            continue
        }
        
        if let Some(meta) = FileMeta::read_from_path(&entry.path()).await {
            check_against_global_retention(meta, now).await?;
        } else {
            // Delete asset straight up when metadata is missing or corrupt
            info!("Deleting {} as its metadata appears corrupt", entry.path().as_os_str().to_string_lossy());
            remove_dir_all(entry.path()).await?;
        };
    }
    Ok(())
}
