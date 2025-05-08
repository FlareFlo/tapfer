use crate::error::{TapferError, TapferResult};
use crate::upload_pool::UploadHandle;
use std::path::Path;
use std::str::FromStr;
use time::{Duration, UtcDateTime};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMeta {
    name: String,
    size: FileSize,
    created: UtcDateTime,
    removal_policy: RemovalPolicy,
    mimetype: String,
}
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum RemovalPolicy {
    SingleDownload,
    Expiry { after: Duration },
}

/// A wrapper for the size of an upload asset
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FileSize {
    /// When the client has transmitted the size of the asset
    AlreadyKnown(u64),
    /// When we are manually keeping track of the asset size
    Dynamic(u64),
}

impl FileSize {
    pub fn current_size(&self) -> u64 {
        match self {
            FileSize::AlreadyKnown(s) => *s,
            FileSize::Dynamic(s) => *s,
        }
    }

    /// Adds extra to the files tracked size. Returns error when size was already known
    pub fn add_size(&mut self, extra: u64) -> TapferResult<()> {
        match self {
            FileSize::AlreadyKnown(_) => Err(TapferError::AddSizeToAlreadyKnown),
            FileSize::Dynamic(s) => {
                *s += extra;
                Ok(())
            }
        }
    }
}
impl FileMeta {
    pub fn default_policy(name: String, mimetype: String, known_size: Option<u64>) -> Self {
        FileMetaBuilder::default().build(name, mimetype, known_size)
    }

    pub fn remove_after_download(&self) -> bool {
        match self.removal_policy {
            RemovalPolicy::SingleDownload => true,
            _ => false,
        }
    }

    pub async fn read_from_uuid_path(path: impl AsRef<Path>) -> TapferResult<(Uuid, Self)> {
        let uuid = Uuid::from_str(&path.as_ref().to_string_lossy())?;
        let meta: FileMeta = Self::read_from_uuid(uuid).await?;
        Ok((uuid, meta))
    }

    pub async fn read_from_uuid(uuid: Uuid) -> TapferResult<Self> {
        Ok(toml::from_str(
            &tokio::fs::read_to_string(format!("data/{uuid}/meta.toml")).await?,
        )?)
    }

    pub fn from_upload_handle(handle: &UploadHandle) -> Self {
        handle.file_meta().clone()
    }

    pub fn add_size(&mut self, extra: u64) -> TapferResult<()> {
        self.size.add_size(extra)
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn content_type(&self) -> &str {
        self.mimetype.as_str()
    }
    pub fn size(&self) -> u64 {
        self.size.current_size()
    }
    pub fn removal_policy(&self) -> RemovalPolicy {
        self.removal_policy
    }
    pub fn created(&self) -> UtcDateTime {
        self.created
    }
    pub fn known_size(&self) -> Option<u64> {
        match self.size {
            FileSize::AlreadyKnown(s) => Some(s),
            FileSize::Dynamic(_) => None,
        }
    }

    pub fn expires_on(&self) -> Option<UtcDateTime> {
        match self.removal_policy {
            RemovalPolicy::SingleDownload => None,
            RemovalPolicy::Expiry { after } => Some(self.created + after),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileMetaBuilder {
    pub expiration: Option<RemovalPolicy>,
    pub in_progress_token: Option<u32>,
}

impl FileMetaBuilder {
    pub fn build(self, name: String, mimetype: String, known_size: Option<u64>) -> FileMeta {
        FileMeta {
            name,
            size: if let Some(s) = known_size {
                FileSize::AlreadyKnown(s)
            } else {
                FileSize::Dynamic(0)
            },
            created: UtcDateTime::now(),
            removal_policy: self.expiration.unwrap_or(RemovalPolicy::SingleDownload),
            mimetype,
        }
    }
}
