use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::http::header::{InvalidHeaderValue, ToStrError};
use axum::response::{Html, IntoResponse, Response};
use std::io;
use std::num::ParseIntError;

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

    #[error("Attempted to add size to already known size")]
    AddSizeToAlreadyKnown,

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
    AxumMultipart(#[from] MultipartError),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error(transparent)]
    ToStrError(#[from] ToStrError),
}

impl IntoResponse for TapferError {
    fn into_response(self) -> Response {
        let generic = |hint: &str| {
            (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Internal Server Error. This is a bug. Report it at https://github.com/FlareFlo/tapfer/issues Hint: {hint}")),
        ).into_response()
        };
        match self {
            TapferError::BadMultipartOrder => generic("multipart order"),
            TapferError::UnknownMultipartField { .. } => generic("unknown multipart field"),
            TapferError::MultipartFieldNameMissing => generic("multipart field name missing"),
            TapferError::Custom { status_code, body } => (status_code, body).into_response(),
            TapferError::StdIo(_) => generic("std_io"),
            TapferError::Askama(_) => generic("askama"),
            TapferError::Uuid(_) => generic("uuid"),
            TapferError::TomlDeserialize(_) => generic("toml deserialization"),
            TapferError::TomlSerialize(_) => generic("toml serialization"),
            TapferError::InvalidHeader(_) => generic("invalid header"),
            TapferError::AxumMultipart(_) => generic("axum multipart"),
            TapferError::ParseIntError(_) => generic("parse int error"),
            TapferError::ToStrError(_) => generic("to str error"),
            TapferError::AddSizeToAlreadyKnown => generic("add size to already known"),
        }
    }
}
