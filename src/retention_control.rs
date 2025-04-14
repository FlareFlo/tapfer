use std::ops::Add;
use std::sync::LazyLock;
use time::{Duration, UtcDateTime};
use tokio::fs;
use tracing::info;
use uuid::Uuid;
use crate::file_meta::FileMeta;

static GLOBAL_RETENTION_POLICY: LazyLock<GlobalRetentionPolicy> = LazyLock::new(|| {
	GlobalRetentionPolicy::default()
});

pub struct GlobalRetentionPolicy {
	maximum_age: Duration,
	recheck_interval: Duration,
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
pub async fn check_against_global_retention(uuid: Uuid, now: UtcDateTime) -> Option<()> {
	let meta = FileMeta::read_from_uuid(uuid).await?;
	if meta.created().add(GLOBAL_RETENTION_POLICY.maximum_age) > now {
		info!("Deleting {uuid} as it expired}");
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
		let (uuid, _) = FileMeta::read_from_path(&entry.path()).await?;
		check_against_global_retention(uuid, now).await?;
	}

	Some(())
}