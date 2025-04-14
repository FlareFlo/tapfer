use std::path::Path;
use std::str::FromStr;
use time::{Duration, UtcDateTime};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMeta {
	name: String,
	size: u64,
	created: UtcDateTime,
	removal_policy: RemovalPolicy,
	mimetype: String,
}
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum RemovalPolicy {
	SingleDownload,
	Expiry {
		after: Duration,
	}
}

impl FileMeta {
	pub fn default_policy(name: String, mimetype: String) -> Self {
		Self {
			name,
			size: 0,
			created: UtcDateTime::now(),
			removal_policy: RemovalPolicy::SingleDownload,
			mimetype,
		}
	}

	pub fn remove_after_download(&self) -> bool {
		match self.removal_policy {
			RemovalPolicy::SingleDownload => {
				true
			}
			_ => false,
		}
	}

	pub async fn read_from_path(path: impl AsRef<Path>) -> Option<(Uuid, Self)> {
		let uuid = Uuid::from_str(path.as_ref().to_str()?).ok()?;
		let meta: FileMeta = Self::read_from_uuid(uuid).await?;
		Some((uuid, meta))
	}

	pub async fn read_from_uuid(uuid: Uuid) -> Option<Self> {
		toml::from_str(&tokio::fs::read_to_string(format!("data/{uuid}/meta.toml")).await.ok()?).ok()?
	}

	pub fn add_size(&mut self, extra: u64) {
		self.size += extra;
	}
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
	pub fn content_type(&self) -> &str {
		self.mimetype.as_str()
	}
	pub fn size(&self) -> u64 {
		self.size
	}
	pub fn created(&self) -> UtcDateTime {
		self.created
	}
}