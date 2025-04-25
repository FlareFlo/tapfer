use crate::file_meta::FileMeta;
use std::ops::Add;
use std::sync::LazyLock;
use time::{Duration, UtcDateTime};
use tokio::fs;
use tokio::fs::remove_dir_all;
use tracing::info;
use uuid::Uuid;

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

#[must_use]
pub async fn check_against_global_retention((uuid, meta): (Uuid, FileMeta), now: UtcDateTime) -> Option<()> {
    if meta.created().add(GLOBAL_RETENTION_POLICY.maximum_age) > now {
        info!("Deleting {uuid} as it has expired");
        delete_asset(uuid).await?;
    }
    Some(())
}

#[must_use]
pub async fn delete_asset(asset: Uuid) -> Option<()> {
    fs::remove_dir_all(format!("data/{asset}")).await.ok()
}

#[must_use]
pub async fn check_all_assets() -> Option<()> {
    let now = UtcDateTime::now();
    let mut dir = fs::read_dir("data").await.ok()?;
    for entry in dir.next_entry().await.ok()? {
        // Skip cachedir tag etc.
        if entry.path().is_file() {
            continue
        }
        
        if let Some(meta) = FileMeta::read_from_path(&entry.path()).await {
            check_against_global_retention(meta, now).await?;
        } else {
            // Delete asset straight up when metadata is missing or corrupt
            info!("Deleting {} as its metadata appears corrupt", entry.path().as_os_str().to_string_lossy());
            remove_dir_all(entry.path()).await.ok()?;
        };
    }
    Some(())
}
