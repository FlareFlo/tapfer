use std::fs;
use std::str::FromStr;
use askama::Template;
use axum::extract::Path;
use axum::response::{Html, IntoResponse};
use uuid::Uuid;
use crate::file_meta::FileMeta;

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
	fs::read(format!("data/{uuid}/{}", meta.name())).unwrap()
}

async fn get_meta_from_path(path: &str) -> (Uuid, FileMeta) {
	let uuid = Uuid::from_str(path).unwrap();
	let meta: FileMeta = toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).unwrap()).unwrap();
	(uuid, meta)
}