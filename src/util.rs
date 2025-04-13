use uuid::Uuid;
use std::fs;
use std::str::FromStr;
use crate::file_meta::FileMeta;

pub async fn get_meta_from_path(path: &str) -> Option<(Uuid, FileMeta)> {
	let uuid = Uuid::from_str(path).ok()?;
	let meta: FileMeta = toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).ok()?).ok()?;
	Some((uuid, meta))
}

pub async fn get_meta_from_uuid(uuid: Uuid) -> Option<FileMeta> {
	toml::from_str(&fs::read_to_string(format!("data/{uuid}/meta.toml")).ok()?).ok()?
}

pub(crate) mod error_compat {
	use axum::http::StatusCode;

	pub trait InternalServerErrorExt<T> {
		fn ise(self) -> Result<T, StatusCode>;
	}

	impl<T, E> InternalServerErrorExt<T> for Result<T, E> {
		fn ise(self) -> Result<T, StatusCode> {
			self.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
		}
	}

	impl<T> InternalServerErrorExt<T> for Option<T> {
		fn ise(self) -> Result<T, StatusCode> {
			self.ok_or(StatusCode::INTERNAL_SERVER_ERROR)
		}
	}
}