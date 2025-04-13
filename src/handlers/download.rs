use std::sync::Mutex;
use std::pin::Pin;
use axum::body::Bytes;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse};
use futures_util::StreamExt;
use tokio_util::io::ReaderStream;
use tracing::debug;
use uuid::Uuid;
use crate::file_meta::FileMeta;
use crate::retention_control::delete_asset;
use crate::util;
use crate::util::get_meta_from_path;

#[derive(Template)]
#[template(path = "download.html")]
struct DownloadTemplate<'a> {
	toml: &'a str,
	download_url: &'a str,
}

pub async fn download_html(Path(path): Path<String>) -> impl IntoResponse {
	let (uuid, meta) = util::get_meta_from_path(&path).await;
	let toml = toml::to_string_pretty(&meta).unwrap();

	let template = DownloadTemplate {
		toml: &toml,
		download_url: &format!("/uploads/{uuid}/download"),
	};

	Html(
		template.render().unwrap()
	)
}

pub async fn download_file(Path(path): Path<String>) -> impl IntoResponse {
	let (uuid, meta) = util::get_meta_from_path(&path).await;

	let mut headers = HeaderMap::new();
	headers.insert(
		header::CONTENT_TYPE,
		HeaderValue::from_str(meta.content_type()).unwrap()
	);
	headers.insert(
		header::CONTENT_DISPOSITION,
		HeaderValue::from_str(&format!("attachment; filename=\"{}\"", meta.name())).unwrap(),
	);
	headers.insert(
		header::CONTENT_LENGTH,
		HeaderValue::from_str(&meta.size().to_string()).unwrap(),
	);

	let path = format!("data/{uuid}/{}", meta.name());
	let file = tokio::fs::File::open(&path).await.unwrap();
	let stream = ReaderStream::new(file);
	let wrapped = CleanupStream::new(stream, uuid, meta);
	(headers, Body::from_stream(wrapped))
}

/// A stream wrapper that deletes the file when dropped
struct CleanupStream<S: Send> {
	inner: S,
	meta: FileMeta,
	uuid: Uuid,
}

impl<S: Send> CleanupStream<S> {
	fn new(inner: S, uuid: Uuid, meta: FileMeta) -> Self {
		Self {
			inner,
			meta,
			uuid,
		}
	}
}

impl<S: Send> Drop for CleanupStream<S> {
	fn drop(&mut self) {
		let meta = self.meta.clone();
		let uuid = self.uuid;
		tokio::spawn(async move {
			if meta.remove_after_download() {
				debug!("Removing {uuid} after download");
				delete_asset(uuid).await;
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