use crate::file_meta::FileMeta;
use dashmap::DashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;
use tokio::task::block_in_place;
use tracing::error;
use uuid::Uuid;

pub static UPLOAD_POOL: LazyLock<UploadPool> = LazyLock::new(|| UploadPool::new());

/// A pool of currently running uploads
#[derive(Debug)]
pub struct UploadPool {
    pub uploads: DashMap<Uuid, UploadHandle>,
}

/// A handle to a running upload
#[derive(Debug, Clone)]
pub struct UploadHandle {
    handle: Arc<RwLock<UploadProgress>>,
    uuid: Uuid,
    file_meta: FileMeta,
}

/// The progress of an upload
#[derive(Debug, Copy, Clone)]
pub struct UploadProgress {
    /// Bytes already written to disk
    progress: usize,
    complete: bool,
}

impl UploadPool {
    pub fn new() -> Self {
        Self {
            uploads: DashMap::new(),
        }
    }

    pub fn handle(&self, uuid: Uuid, file_meta: FileMeta) -> UploadHandle {
        let handle = UploadHandle {
            handle: Arc::new(RwLock::new(UploadProgress {
                progress: 0,
                complete: false,
            })),
            uuid,
            file_meta,
        };
        self.uploads.insert(uuid, handle.clone());
        handle
    }
}

impl UploadHandle {
    /// Adds already written bytes to progress
    pub async fn add_progress(&self, progress: usize) {
        self.handle.write().await.progress += progress;
    }

    /// Marks upload complete
    pub async fn mark_complete(&self) {
        self.handle.write().await.complete = true;
    }

    pub async fn is_complete(&self) -> bool {
        self.handle.read().await.complete
    }

    pub fn is_complete_blocking(&self) -> bool {
        block_in_place(|| self.handle.blocking_read().complete)
    }

    pub fn file_meta(&self) -> &FileMeta {
        &self.file_meta
    }
}

impl Drop for UploadHandle {
    fn drop(&mut self) {
        // The strong count could be incremented after this check, however, removing the entry is not problematic as
        // A. The incrementer holds a valid reference to the handle
        // B. The incrementer sees the upload is complete, therefore not needing the handle anymore
        // We check for 2 or less as the map always holds a strong count
        if Arc::strong_count(&self.handle) <= 2 {
            if self.is_complete_blocking() {
                // This is hopefully the case, as removing the last handle should only happen when it is completed
            } else {
                error!(
                    "Upload handle {} dropped while it was not completed!",
                    self.uuid
                );
            }
            // Remove it in either case to avoid stale and broken entries
            UPLOAD_POOL.uploads.remove(&self.uuid);
        }
    }
}
