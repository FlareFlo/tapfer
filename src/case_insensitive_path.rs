use axum::body::Body;
use axum::{http::Request, middleware::Next, response::Response};
use http::Uri;

pub async fn lowercase_path_middleware(mut req: Request<Body>, next: Next) -> Response {
    let lower = req.uri().path().to_ascii_lowercase();
    let mut parts = req.uri().clone().into_parts();
    parts.path_and_query = Some(lower.parse().unwrap());
    let new_uri = Uri::from_parts(parts).unwrap();
    *req.uri_mut() = new_uri;
    next.run(req).await
}
