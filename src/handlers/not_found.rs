use crate::configuration::{EMBED_DESCRIPTION, FAVICON};
use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use std::fmt::{Display, Formatter};

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound {
    embed_image_url: &'static str,
    embed_description: &'static str,
    reason: &'static str,
}

impl NotFound {
    pub fn with_reason(hint: Option<Reason404>) -> Self {
        Self {
            embed_image_url: FAVICON,
            embed_description: EMBED_DESCRIPTION,
            reason: match hint {
                None => "The asset you're looking for doesn’t exist or has been deleted",
                Some(Reason404::Deleted) => "The asset has been deleted",
            },
        }
    }
}

pub async fn not_found_handler(Query(params): Query<Params>) -> impl IntoResponse {
    match NotFound::with_reason(params.hint).render() {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("500 Internal Server Error".to_string()),
        ),
    }
}

pub fn redirect_not_found(reason404: Reason404) -> impl IntoResponse {
    Redirect::to(&format!("/404?hint={reason404}"))
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Params {
    hint: Option<Reason404>,
}

#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum Reason404 {
    #[serde(rename = "deleted")]
    Deleted,
}

impl Display for Reason404 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason404::Deleted => {
                write!(f, "deleted")
            }
        }
    }
}
