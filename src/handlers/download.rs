use crate::configuration::{DOWNLOAD_CHUNKSIZE, EMBED_DESCRIPTION, QR_CODE_SIZE};
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, RemovalPolicy};
use crate::handlers::not_found::NotFound;
use crate::retention_control::delete_asset;
use crate::updown::upload_handle::UploadHandle;
use crate::updown::upload_pool::{UPLOAD_POOL, UploadFsm};
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse};
use futures_util::StreamExt;
use human_bytes::human_bytes;
use std::io;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use tokio::fs::File;
use tokio::{fs};
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Template)]
#[template(path = "download.html")]
struct DownloadTemplate<'a> {
    filename: &'a str,
    expiry: &'a str,
    download_url: &'a str,
    mimetype: &'a str,
    filesize: &'a str,
    uuid: Uuid,
    embed_image_url: &'a str,
    qr_size: usize,
    embed_description: &'a str,
}

pub async fn download_html(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((uuid, meta), progress_handle) = get_any_meta(&path).await?;

    static DES: &[BorrowedFormatItem<'_>] =
        format_description!("[hour]:[minute] [week_number]-[week_number]-[year]");
    let expiry = match meta.removal_policy() {
        RemovalPolicy::SingleDownload => " after a single download".to_owned(),
        RemovalPolicy::Expiry { .. } => meta.expires_on().unwrap().format(&DES)?.to_string(),
    };

    let template = DownloadTemplate {
        filename: meta.name(),
        expiry: &expiry,
        download_url: &format!("/uploads/{uuid}/download"),
        mimetype: meta.content_type(),
        filesize: if meta.known_size().is_some() {
            &human_bytes(meta.size() as f64)
        } else if progress_handle.is_some() {
            "upload in progress"
        } else {
            &human_bytes(meta.size() as f64)
        },
        uuid,
        embed_image_url: &format!("/qrcg/{uuid}"),
        qr_size: QR_CODE_SIZE,
        embed_description: EMBED_DESCRIPTION,
    };

    Ok(Html(template.render()?))
}

async fn get_any_meta(path: &String) -> TapferResult<((Uuid, FileMeta), Option<UploadHandle>)> {
    let uuid = Uuid::from_str(path)?;
    let res = match fs::try_exists(&format!("data/{uuid}/meta.toml")).await.ok() {
        // Regular download
        Some(true) => (FileMeta::read_from_uuid_path(&path).await?, None),
        // In-progress upload or doesnt exist
        _ => {
            let uuid = Uuid::from_str(path)?;
            match UPLOAD_POOL.uploads.get(&uuid) {
                // The upload is not in progress either, so it does not exist
                None => {
                    return Err(TapferError::Custom {
                        status_code: StatusCode::NOT_FOUND,
                        body: Html(NotFound::default().render()?),
                    });
                }
                // The upload is in-progress
                Some(handle) => (
                    (*handle.key(), FileMeta::from_upload_handle(handle.value())),
                    Some(handle.clone()),
                ),
            }
        }
    };
    Ok(res)
}

pub async fn download_file(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((uuid, meta), handle) = get_any_meta(&path).await?;

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
    else if handle.is_none() {
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&meta.size().to_string())?,
        );
    }

    let path = format!("data/{uuid}/{}", meta.name());
    let file = File::open(&path).await?;
    let stream = ReaderStream::with_capacity(file, DOWNLOAD_CHUNKSIZE);
    let wrapped = DownloadStream::new(
        stream,
        uuid,
        meta,
        handle,
        UPLOAD_POOL.uploads.contains_key(&uuid),
    );
    Ok((headers, Body::from_stream(wrapped)))
}

/// A stream wrapper that deletes the file when dropped and rate-limits download during updown
struct DownloadStream {
    inner: ReaderStream<File>,
    meta: FileMeta,
    uuid: Uuid,
    handle: Option<UploadHandle>,
    self_progress: u64,
    is_updown: bool,
}

impl DownloadStream {
    fn new(
        inner: ReaderStream<File>,
        uuid: Uuid,
        meta: FileMeta,
        handle: Option<UploadHandle>,
        is_updown: bool,
    ) -> Self {
        Self {
            inner,
            meta,
            uuid,
            handle,
            self_progress: 0,
            is_updown,
        }
    }
}

impl Drop for DownloadStream {
    fn drop(&mut self) {
        // Do not delete files in upload when an in-progress download fails early
        if self.is_updown {
            return;
        }
        let meta = self.meta.clone();
        let uuid = self.uuid;

        tokio::spawn(async move {
            if meta.remove_after_download() {
                info!("Removing {uuid} as its download has completed");
                match delete_asset(uuid).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to delete {uuid} because {e:?}")
                    }
                }
            }
        });
    }
}

impl futures_core::Stream for DownloadStream {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(handle) = &self.handle {
            // Delay polling the file when it is incomplete and the current progress is very close to the upload progress
            let fsm = handle.read_fsm_blocking().clone();

            match fsm {
                UploadFsm::Failed => {
                    return Poll::Ready(Some(Err(TapferError::Custom {
                        status_code: StatusCode::RESET_CONTENT,
                        body: Html("Upload was aborted".to_owned()),
                    }
                    .into())));
                }
                UploadFsm::InProgress { progress } => {
                    if (progress - DOWNLOAD_CHUNKSIZE as u64 * 2) < self.self_progress {
                        let waker = cx.waker().clone();
                        let handle = handle.clone();
                        tokio::spawn(async move {
                            handle.wait_for_progress().await;
                            waker.wake();
                        });
                        return Poll::Pending;
                    }
                }
                UploadFsm::Completed =>{},
            }
        }

        let poll_res = self.inner.poll_next_unpin(cx);
        if let Poll::Ready(Some(Ok(b))) = &poll_res {
            self.self_progress += b.len() as u64;
        }
        poll_res
    }
}
