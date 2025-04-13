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