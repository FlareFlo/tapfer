use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tokio::task::block_in_place;
use tracing::error;
use uuid::Uuid;
use crate::file_meta::FileMeta;
use crate::updown::upload_pool::{UploadPool, UploadProgress, UPLOAD_POOL};

/// A handle to a running upload
#[derive(Debug, Clone)]
pub struct UploadHandle {
    handle: Arc<RwLock<UploadProgress>>,
    uuid: Uuid,
    file_meta: FileMeta,
    notify: Arc<Notify>,
}

impl UploadPool {
    pub fn handle(&self, uuid: Uuid, file_meta: FileMeta) -> UploadHandle {
        let handle = UploadHandle {
            handle: Arc::new(RwLock::new(UploadProgress {
                progress: 0,
                complete: false,
                upload_failed: false,
            })),
            uuid,
            file_meta,
            notify: Arc::new(Notify::new()),
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

    pub async fn _get_progress(&self) -> usize {
        self.handle.read().await.progress
    }

    pub fn get_progress_blocking(&self) -> usize {
        block_in_place(|| self.handle.blocking_read().progress)
    }

    pub fn has_upload_failed(&self) -> bool {
        self.handle.blocking_read().upload_failed
    }
    pub fn set_upload_failed(&self) {
        self.handle.blocking_write().upload_failed = true;
    }

    /// Waits for uploader to add progress
    pub async fn wait_for_progress(&self) {
        self.notify.notified().await;
    }

    /// Notifies all downloaders about progress
    pub fn notify_all_downloaders(&self) {
        self.notify.notify_waiters();
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
