use time::{Duration, UtcDateTime};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileMeta {
	name: String,
	size: u64,
	created: UtcDateTime,
	removal_policy: RemovalPolicy,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum RemovalPolicy {
	SingleDownload,
	Expiry {
		after: Duration,
	}
}

impl FileMeta {
	pub fn default_policy(name: String) -> Self {
		Self {
			name,
			size: 0,
			created: UtcDateTime::now(),
			removal_policy: RemovalPolicy::SingleDownload,
		}
	}
	pub fn add_size(&mut self, extra: u64) {
		self.size += extra;
	}
	pub fn name(&self) -> &str {
		self.name.as_str()
	}
}