use axum::http::StatusCode;
use axum::response::Html;

pub(crate) mod error_compat {
    use crate::error_compat::ApiResult;
    use axum::http::StatusCode;
    use axum::response::Html;

    pub trait InternalServerErrorExt<T> {
        fn ise(self) -> ApiResult<T>;
    }

    impl<T, E> InternalServerErrorExt<T> for Result<T, E> {
        fn ise(self) -> ApiResult<T> {
            self.map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("Internal Server Error".to_owned()),
                )
            })
        }
    }

    impl<T> InternalServerErrorExt<T> for Option<T> {
        fn ise(self) -> ApiResult<T> {
            self.ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Internal Server Error".to_owned()),
            ))
        }
    }
}

pub type ApiResult<T> = Result<T, (StatusCode, Html<String>)>;

#[derive(thiserror::Error, Debug)]
pub enum InternalServerError {
    #[error("multipart form had fields after the file")]
    BadMultipartOrder,
    #[error("unknown field name {field_name}")]
    UnknownMultipartField { field_name: String },
}
