use crate::file_meta::{FileMeta, RemovalPolicy};
use crate::handlers::not_found::NotFound;
use crate::retention_control::delete_asset;
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse};
use futures_util::StreamExt;
use human_bytes::human_bytes;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs;
use tokio::io::BufReader;
use tokio_util::io::ReaderStream;
use tracing::{error, info};
use uuid::Uuid;
use crate::error::{TapferError, TapferResult};

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
    if fs::try_exists(format!("data/{path}")).await.ok() != Some(true) {
        return Err(TapferError::Custom { status_code: StatusCode::NOT_FOUND, body: Html(NotFound.render()?) });
    }

    let (uuid, meta) = FileMeta::read_from_path(&path).await?;

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
        filesize: &human_bytes(meta.size() as f64),
    };

    Ok(Html(template.render()?))
}

pub async fn download_file(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let (uuid, meta) = FileMeta::read_from_path(&path).await?;

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
    let file = tokio::fs::File::open(&path).await?;
    let stream = ReaderStream::new(BufReader::new(file));
    let wrapped = CleanupStream::new(stream, uuid, meta);
    Ok((headers, Body::from_stream(wrapped)))
}

/// A stream wrapper that deletes the file when dropped
struct CleanupStream<S: Send> {
    inner: S,
    meta: FileMeta,
    uuid: Uuid,
}

impl<S: Send> CleanupStream<S> {
    fn new(inner: S, uuid: Uuid, meta: FileMeta) -> Self {
        Self { inner, meta, uuid }
    }
}

impl<S: Send> Drop for CleanupStream<S> {
    fn drop(&mut self) {
        let meta = self.meta.clone();
        let uuid = self.uuid;
        tokio::spawn(async move {
            if meta.remove_after_download() {
                info!("Removing {uuid} as its download has completed");
                let res = delete_asset(uuid).await;
                if res.is_ok() {
                    error!("Failed to delete {uuid} because {res:?}")
                }
            }
        });
    }
}

impl<S, T, E> futures_core::Stream for CleanupStream<S>
where
    S: futures_core::Stream<Item = Result<T, E>> + Unpin + Send,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}
