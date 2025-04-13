use time::{Duration, UtcDateTime};

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