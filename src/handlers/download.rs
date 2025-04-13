use std::fs;
use std::str::FromStr;
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse};
use uuid::Uuid;
use crate::file_meta::FileMeta;
use tokio_util::io::ReaderStream;

#[derive(Template)]
#[template(path = "download.html")]
struct DownloadTemplate<'a> {
	toml: &'a str,
	download_url: &'a str,
}

pub async fn download_html(Path(path): Path<String>) -> impl IntoResponse {
	let (uuid, meta) = get_meta_from_path(&path).await;
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
	let (uuid, meta) = get_meta_from_path(&path).await;

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
	let file = tokio::fs::File::open(path).await.unwrap();
	let stream = ReaderStream::new(file);
	(headers, Body::from_stream(stream))
}

async fn get_meta_from_path(path: &str) -> (Uuid, FileMeta) {
	let uuid = Uuid::from_str(path).unwrap();
	let meta: FileMeta = toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).unwrap()).unwrap();
	(uuid, meta)
}