use uuid::Uuid;
use std::fs;
use std::str::FromStr;
use crate::file_meta::FileMeta;

pub async fn get_meta_from_path(path: &str) -> (Uuid, FileMeta) {
	let uuid = Uuid::from_str(path).unwrap();
	let meta: FileMeta = toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).unwrap()).unwrap();
	(uuid, meta)
}

pub async fn get_meta_from_uuid(uuid: Uuid) -> FileMeta {
	let meta: FileMeta = toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).unwrap()).unwrap();
	meta
}