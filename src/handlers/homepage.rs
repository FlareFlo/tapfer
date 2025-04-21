use askama::Template;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "homepage.html")]
pub struct Homepage;

pub async fn show_form() -> impl IntoResponse {
    Html(Homepage.render().unwrap())
}
