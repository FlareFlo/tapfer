use std::io;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, RemovalPolicy};
use crate::handlers;
use crate::handlers::not_found::NotFound;
use crate::retention_control::delete_asset;
use crate::upload_pool::{UPLOAD_POOL, UploadHandle};
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse};
use dashmap::mapref::one::Ref;
use futures_util::StreamExt;
use human_bytes::human_bytes;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tokio::fs;
use tokio::fs::File;
use tokio::io::BufReader;
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Template)]
#[template(path = "download.html")]
struct DownloadTemplate<'a> {
    filename: &'a str,
    unix_time: i64,
    expiry: &'a str,
    download_url: &'a str,
    mimetype: &'a str,
    filesize: &'a str,
}

pub async fn download_html(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let ((uuid, meta), progress_handle) = get_any_meta(&path).await?;

    let expiry = match meta.removal_policy() {
        RemovalPolicy::SingleDownload => " after a single download".to_owned(),
        RemovalPolicy::Expiry { .. } => {
            format!(" on {}", meta.expires_on().unwrap())
        }
    };

    let template = DownloadTemplate {
        filename: meta.name(),
        unix_time: meta.created().unix_timestamp(),
        expiry: &expiry,
        download_url: &format!("/uploads/{uuid}/download"),
        mimetype: meta.content_type(),
        filesize: if let Some(_) = progress_handle {
            "upload in progress"
        } else {
            &human_bytes(meta.size() as f64)
        },
    };

    Ok(Html(template.render()?))
}

async fn get_any_meta(path: &String) -> TapferResult<((Uuid, FileMeta), Option<UploadHandle>)> {
    let data_path = format!("data/{path}");
    let res = match fs::try_exists(&data_path).await.ok() {
        // Regular download
        Some(true) => (FileMeta::read_from_uuid_path(&path).await?, None),
        // In-progress upload or doesnt exist
        _ => {
            let uuid = Uuid::from_str(&path)?;
            match UPLOAD_POOL.uploads.get(&uuid) {
                // The upload is not in progress either, so it does not exist
                None => {
                    return Err(TapferError::Custom {
                        status_code: StatusCode::NOT_FOUND,
                        body: Html(NotFound.render()?),
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
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&meta.size().to_string())?,
    );

    let path = format!("data/{uuid}/{}", meta.name());
    let file = File::open(&path).await?;
    let stream = ReaderStream::new(BufReader::new(file));
    let wrapped = CleanupStream::new(stream, uuid, meta, handle);
    Ok((headers, Body::from_stream(wrapped)))
}

/// A stream wrapper that deletes the file when dropped
struct CleanupStream {
    inner: ReaderStream<BufReader<File>>,
    meta: FileMeta,
    uuid: Uuid,
    handle: Option<UploadHandle>,
    self_progress: usize,
}

impl CleanupStream {
    fn new(inner: ReaderStream<BufReader<File>>, uuid: Uuid, meta: FileMeta, handle: Option<UploadHandle>) -> Self {
        Self { inner, meta, uuid, handle, self_progress: 0 }
    }
}

impl Drop for CleanupStream {
    fn drop(&mut self) {
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

impl futures_core::Stream for CleanupStream
{
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // match Pin::new(&mut self.inner).poll_next(cx) {
        //     Poll::Ready(Some(Ok(chunk))) => {
        //         todo!()
        //     }
        //     other => other,
        // }
        self.inner.poll_next_unpin(cx)
    }
}
