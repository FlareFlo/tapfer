use std::ops::Add;
use std::sync::LazyLock;
use time::{Duration, UtcDateTime};
use tokio::fs;
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
		delete_asset(uuid).await?;
	}
	Some(())
}

#[must_use]
pub async fn delete_asset(asset: Uuid) -> Option<()> {
	fs::remove_dir_all(format!("data/{asset}")).await.ok()
}