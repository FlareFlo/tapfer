use askama::Template;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "homepage.html")]
pub struct Homepage {
    embed_image_url: &'static str,
}

pub async fn show_form() -> impl IntoResponse {
    Html(Homepage{ embed_image_url: "/graphics/favicon.ico" }.render().unwrap())
}
