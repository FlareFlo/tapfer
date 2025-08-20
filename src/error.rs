use crate::updown::upload_pool::UploadFsm;
use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::http::header::{InvalidHeaderValue, ToStrError};
use axum::response::{Html, IntoResponse, Response};
use qrcode_generator::QRCodeError;
use std::array::TryFromSliceError;
use std::io;
use std::num::ParseIntError;
use time::error::Format;

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

    #[error("Added size to UploadFsm when it was already {0:?}")]
    UploadHandleSize(UploadFsm),

    #[error("The requested token {0} does not have a matching ID/upload")]
    TokenDoesNotExist(u32),

    #[error("Invalid expiration {0}")]
    InvalidExpiration(String),

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

    #[error(transparent)]
    QRCodeError(#[from] QRCodeError),

    #[error(transparent)]
    TimeFormat(#[from] Format),

    #[error(transparent)]
    TryFromSlice(#[from] TryFromSliceError),
}

impl IntoResponse for TapferError {
    fn into_response(self) -> Response {
        let generic = |hint: &str| {
            (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Internal Server Error. This is a bug. Report it at https://github.com/FlareFlo/tapfer/issues Hint: {hint}")),
        ).into_response()
        };
        use TapferError::*;
        match self {
            BadMultipartOrder => generic("multipart order"),
            UnknownMultipartField { .. } => generic("unknown multipart field"),
            MultipartFieldNameMissing => generic("multipart field name missing"),
            Custom { status_code, body } => (status_code, body).into_response(),
            StdIo(_) => generic("std_io"),
            Askama(_) => generic("askama"),
            Uuid(_) => generic("uuid"),
            TomlDeserialize(_) => generic("toml deserialization"),
            TomlSerialize(_) => generic("toml serialization"),
            InvalidHeader(_) => generic("invalid header"),
            AxumMultipart(_) => generic("axum multipart"),
            ParseIntError(_) => generic("parse int error"),
            ToStrError(_) => generic("to str error"),
            AddSizeToAlreadyKnown => generic("add size to already known"),
            TokenDoesNotExist(id) => (
                StatusCode::NOT_FOUND,
                format!("Upload token {id} not associated with any asset"),
            )
                .into_response(),
            QRCodeError(_) => generic("qr code generation"),
            InvalidExpiration(s) => generic(&format!("invalid expiration: {s}")),
            TimeFormat(_) => generic("time format"),
            UploadHandleSize(_) => generic("upload handle size"),
            TryFromSlice(_) => generic("try from slice"),
        }
    }
}

impl From<TapferError> for io::Error {
    fn from(t: TapferError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, t.to_string())
    }
}
