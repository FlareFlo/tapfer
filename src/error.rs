use std::io;
use axum::http::StatusCode;
use axum::response::Html;

pub type ApiResult<T> = Result<T, (StatusCode, Html<String>)>;

#[derive(thiserror::Error, Debug)]
pub enum InternalServerError {
    #[error("multipart form had fields after the file")]
    BadMultipartOrder,
    #[error("unknown field name {field_name}")]
    UnknownMultipartField { field_name: String },
}

pub type TapferResult<T> = Result<T, TapferError>;
#[derive(thiserror::Error, Debug)]
pub enum TapferError {
	
    
    #[error(transparent)]
    StdIo(#[from] io::Error),
}