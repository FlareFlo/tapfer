use std::io;
use axum::extract::multipart::MultipartError;
use axum::http::header::InvalidHeaderValue;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub type TapferResult<T> = Result<T, TapferError>;
#[derive(thiserror::Error, Debug)]
pub enum TapferError {
    #[error("multipart form had fields after the file")]
    BadMultipartOrder,

    #[error("unknown field name {field_name}")]
    UnknownMultipartField { field_name: String },
    
    #[error("multipart field has no name")]
    MultipartFieldNameMissing,
    
    #[error("Custom error with status code {status_code}")]
    Custom {
        status_code: StatusCode,
        body: Html<String>,
    },
    
    #[error(transparent)]
    StdIo(#[from] io::Error),
    
    #[error(transparent)]
    Askama(#[from] askama::Error),
    
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    
    #[error(transparent)]
    InvalidHeader(#[from] InvalidHeaderValue),
    
    #[error(transparent)]
    AxumMultipart(#[from] MultipartError)
}

impl IntoResponse for TapferError {
    fn into_response(self) -> Response {
        let generic = (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("Internal Server Error. This is a bug. Report it at https://github.com/FlareFlo/tapfer/issues".to_owned()),
        ).into_response();
        match self {
            TapferError::BadMultipartOrder => {generic}
            TapferError::UnknownMultipartField { .. } => {generic}
            TapferError::MultipartFieldNameMissing => {generic}
            TapferError::Custom { .. } => {generic}
            TapferError::StdIo(_) => {generic}
            TapferError::Askama(_) => {generic}
            TapferError::Uuid(_) => {generic}
            TapferError::TomlDeserialize(_) => {generic}
            TapferError::TomlSerialize(_) => {generic}
            TapferError::InvalidHeader(_) => {generic}
            TapferError::AxumMultipart(_) => {generic}
        }
    }
}