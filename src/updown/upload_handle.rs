use crate::UPLOAD_POOL;
use crate::file_meta::FileMeta;
use crate::updown::upload_pool::{UploadFsm, UploadPool};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use tokio::task::block_in_place;
use tracing::error;
use uuid::Uuid;

/// A handle to a running upload
#[derive(Debug, Clone)]
pub struct UploadHandle {
    handle: Arc<RwLock<UploadFsm>>,
    uuid: Uuid,
    file_meta: FileMeta,
    notify: Arc<Notify>,
}

impl UploadPool {
    pub fn handle(&self, uuid: Uuid, file_meta: FileMeta) -> UploadHandle {
        let handle = UploadHandle {
            handle: Arc::new(RwLock::new(UploadFsm::initial())),
            uuid,
            file_meta,
            notify: Arc::new(Notify::new()),
        };
        self.uploads.insert(uuid, handle.clone());
        handle
    }
}

impl UploadHandle {
    #[allow(dead_code)]
    pub async fn read_fsm(&self) -> RwLockReadGuard<UploadFsm> {
        self.handle.read().await
    }

    pub async fn write_fsm(&self) -> RwLockWriteGuard<UploadFsm> {
        self.handle.write().await
    }

    pub fn read_fsm_blocking(&self) -> RwLockReadGuard<UploadFsm> {
        block_in_place(|| self.handle.blocking_read())
    }

    pub fn write_fsm_blocking(&self) -> RwLockWriteGuard<UploadFsm> {
        block_in_place(|| self.handle.blocking_write())
    }

    /// Waits for uploader to add progress
    pub async fn wait_for_progress(&self) {
        self.notify.notified().await;
    }

    /// Notifies all downloaders about progress
    pub fn notify_all_downloaders(&self) {
        self.notify.notify_waiters();
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
            if matches!(*self.read_fsm_blocking(), UploadFsm::Completed | UploadFsm::Failed) {
                // This is hopefully the case, as removing the last (external) handle should only happen when it is completed or aborted
            } else {
                error!(
                    "Upload handle {} dropped while it was in progress!",
                    self.uuid
                );
            }
            // Remove it in either case to avoid stale and broken entries
            UPLOAD_POOL.uploads.remove(&self.uuid);
        }
    }
}
