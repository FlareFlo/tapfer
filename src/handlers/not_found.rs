use crate::configuration::{EMBED_DESCRIPTION, FAVICON};
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound {
    embed_image_url: &'static str,
    embed_description: &'static str,
}

impl Default for NotFound {
    fn default() -> Self {
        Self {
            embed_image_url: FAVICON,
            embed_description: EMBED_DESCRIPTION,
        }
    }
}

pub async fn not_found_handler() -> impl IntoResponse {
    match NotFound::default().render() {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Html("500 Internal Server Error".to_string())),
    }
}
