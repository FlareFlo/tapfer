use crate::error::{TapferError, TapferResult};
use crate::updown::upload_handle::UploadHandle;
use dashmap::DashMap;
use crate::tapfer_id::TapferId;

/// A pool of currently running uploads
#[derive(Debug)]
pub struct UploadPool {
    pub uploads: DashMap<TapferId, UploadHandle>,
}

/// The progress of an upload
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UploadFsm {
    Failed,
    InProgress {
        /// Bytes already written to disk
        progress: u64,
    },
    Completed,
}

impl UploadFsm {
    pub fn initial() -> Self {
        Self::InProgress { progress: 0 }
    }

    pub fn add_progress(&mut self, new_progress: usize) -> TapferResult<()> {
        match self {
            UploadFsm::InProgress { progress } => {
                *progress += new_progress as u64;
                Ok(())
            }
            _ => Err(TapferError::UploadHandleSize(*self)),
        }
    }
    pub fn mark_complete(&mut self) {
        *self = Self::Completed;
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, UploadFsm::Completed)
    }

    pub fn get_progress(&self) -> Option<u64> {
        match self {
            UploadFsm::InProgress { progress, .. } => Some(*progress),
            _ => None,
        }
    }
}

impl UploadPool {
    pub fn new() -> Self {
        Self {
            uploads: DashMap::new(),
        }
    }
}
