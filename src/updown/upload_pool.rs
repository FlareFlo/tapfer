use crate::updown::upload_handle::UploadHandle;
use crate::file_meta::FileMeta;
use dashmap::DashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::{Notify, RwLock};
use tokio::task::block_in_place;
use tracing::error;
use uuid::Uuid;

pub static UPLOAD_POOL: LazyLock<UploadPool> = LazyLock::new(UploadPool::new);

/// A pool of currently running uploads
#[derive(Debug)]
pub struct UploadPool {
    pub uploads: DashMap<Uuid, UploadHandle>,
}

/// The progress of an upload
#[derive(Debug, Copy, Clone)]
pub struct UploadProgress {
    /// Bytes already written to disk
    pub progress: usize,
    pub complete: bool,
    pub upload_failed: bool,
}

impl UploadPool {
    pub fn new() -> Self {
        Self {
            uploads: DashMap::new(),
        }
    }
}