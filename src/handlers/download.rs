use crate::configuration::{DOWNLOAD_CHUNKSIZE, EMBED_DESCRIPTION, QR_CODE_SIZE};
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, RemovalPolicy};
use crate::handlers;
use crate::retention_control::delete_asset;
use crate::updown::upload_handle::UploadHandle;
use crate::updown::upload_pool::UploadFsm;
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse};
use futures_util::StreamExt;
use human_bytes::human_bytes;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use tokio::fs::File;
use tokio::select;
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;
use tracing::{error, info};
use crate::handlers::qrcode::base64_qr_from_id;
use crate::tapfer_id::TapferId;

#[derive(Template)]
#[template(path = "download.html")]
struct DownloadTemplate<'a> {
    filename: &'a str,
    expiry: &'a str,
    download_url: &'a str,
    mimetype: &'a str,
    filesize: &'a str,
    embed_image_url: &'a str,
    qr_size: usize,
    embed_description: &'a str,
    delete_url: &'a str,
    qr_b64: String,
}

pub async fn download_html(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((id, meta), progress_handle) = handlers::get_any_meta(&path).await?;

    static DES: &[BorrowedFormatItem<'_>] =
        format_description!("[hour]:[minute] [week_number]-[week_number]-[year]");
    let expiry = match meta.removal_policy() {
        RemovalPolicy::SingleDownload => " after a single download".to_owned(),
        RemovalPolicy::Expiry { .. } => meta.expires_on().unwrap().format(&DES)?.to_string(),
    };

    let template = DownloadTemplate {
        filename: meta.name(),
        expiry: &expiry,
        download_url: &format!("/uploads/{id}/download"),
        mimetype: meta.content_type(),
        filesize: if meta.known_size().is_some() {
            &human_bytes(meta.size() as f64)
        } else if matches!(progress_handle, UpDownFsm::UpdownInProgress { .. }) {
            "upload in progress"
        } else {
            &human_bytes(meta.size() as f64)
        },
        embed_image_url: &format!("/qrcg/{id}"),
        qr_size: QR_CODE_SIZE,
        embed_description: EMBED_DESCRIPTION,
        delete_url: &format!("/uploads/{id}/delete"),
        qr_b64: base64_qr_from_id(id)?,
    };

    Ok(Html(template.render()?))
}

pub async fn download_file(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((id, meta), fsm) = handlers::get_any_meta(&path).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(meta.content_type())?,
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", meta.name()))?,
    );
    // Add size when it is known
    if let Some(known) = meta.known_size() {
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&known.to_string())?,
        );
    }
    // or when there is no ongoing upload
    else if matches!(fsm, UpDownFsm::Completed) {
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&meta.size().to_string())?,
        );
    }

    let path = format!("data/{id}/{}", meta.name());
    let file = File::open(&path).await?;
    let stream = ReaderStream::with_capacity(file, DOWNLOAD_CHUNKSIZE);

    let wrapped = DownloadStream::new(stream, id, meta, fsm);
    Ok((headers, Body::from_stream(wrapped)))
}

/// A stream wrapper that deletes the file when dropped and rate-limits download during updown
struct DownloadStream {
    inner: ReaderStream<File>,
    meta: FileMeta,
    id: TapferId,
    fsm: UpDownFsm,
}

/// FSM describing the state of a possibly ongoing upload
pub enum UpDownFsm {
    Completed,
    UpdownInProgress { progress: u64, handle: UploadHandle },
}

impl UpDownFsm {
    /// Adds extra progress to current one.
    /// Does nothing when the upload is already complete
    pub fn add_progress(&mut self, additional: u64) {
        if let UpDownFsm::UpdownInProgress { progress, .. } = self {
            *progress += additional;
        }
    }
}

impl DownloadStream {
    fn new(inner: ReaderStream<File>, id: TapferId, meta: FileMeta, fsm: UpDownFsm) -> Self {
        Self {
            inner,
            meta,
            id: id,
            fsm,
        }
    }
}

/// Responsible for deleting single-download files.
/// Skips deletion when the download is initiated during upload
impl Drop for DownloadStream {
    fn drop(&mut self) {
        // Do not delete files in upload when an in-progress download fails early
        if !matches!(self.fsm, UpDownFsm::Completed) {
            return;
        }
        let meta = self.meta.clone();
        let id = self.id;

        tokio::spawn(async move {
            if meta.remove_after_download() {
                info!("Removing {id} as its download has completed");
                match delete_asset(id).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to delete {id} because {e:?}")
                    }
                }
            }
        });
    }
}

/// Main goals here:
/// Permit unbounded download when the asset is a regular file, transparently polling inner.
/// Throttle download to the already uploaded (and written) data boundary, when upload is in progress.
/// Abort download when the uploader failed/cancelled.
impl futures_core::Stream for DownloadStream {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if were in progress
        match &self.fsm {
            // Conditionally wake here
            UpDownFsm::UpdownInProgress {
                handle,
                progress: download_progress,
            } => {
                let upload_fsm = *handle.read_fsm_blocking();

                // Ensure all branches either return or wake
                match upload_fsm {
                    // Abort download on upload error
                    UploadFsm::Failed => {
                        return Poll::Ready(Some(Err(TapferError::Custom {
                            status_code: StatusCode::GONE,
                            body: Html("Upload was aborted".to_owned()),
                        }
                        .into())));
                    }
                    // Wake once progress is available only
                    UploadFsm::InProgress {
                        progress: upload_progress,
                    } => {
                        // Delay polling the file when it is incomplete and the current progress is very close to the upload progress
                        if (upload_progress - DOWNLOAD_CHUNKSIZE as u64 * 2) < *download_progress {
                            let waker = cx.waker().clone();
                            let handle = handle.clone();
                            // Ensure that we do not wait for progress perpetually, time out after a bit to poll the UploadFSM again in case it failed
                            tokio::spawn(async move {
                                let timeout = tokio::time::sleep(Duration::from_millis(100));
                                let progress = handle.wait_for_progress();
                                select! {
                                    _ = timeout => (),
                                    _ = progress => (),
                                }
                                waker.wake();
                            });
                            return Poll::Pending;
                        }
                    }
                    // Wake once such that we get polled again to wake from below
                    UploadFsm::Completed => {
                        self.fsm = UpDownFsm::Completed;
                        cx.waker().wake_by_ref();
                    }
                }
            }
            // Wake, always
            _ => cx.waker().wake_by_ref(),
        }

        let poll_res = self.inner.poll_next_unpin(cx);
        if let Poll::Ready(Some(Ok(b))) = &poll_res {
            self.fsm.add_progress(b.len() as u64);
        }
        poll_res
    }
}
