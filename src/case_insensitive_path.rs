use axum::{http::Request, middleware::Next, response::Response};
use http::Uri;

async fn lowercase_path_middleware<B>(mut req: Request<B>, next: Next<B>) -> Response {
	if let Some(pq) = req.uri().path_and_query() {
		let lower = pq.as_str().to_lowercase();
		let mut parts = req.uri().clone().into_parts();
		parts.path_and_query = Some(lower.parse().unwrap());
		let new_uri = Uri::from_parts(parts).unwrap();
		*req.uri_mut() = new_uri;
	}
	next.run(req).await
}
