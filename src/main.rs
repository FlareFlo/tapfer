mod file_meta;
mod handlers;
mod retention_control;
mod error_compat;

use std::fs;
use axum::{
    extract::{DefaultBodyLimit},
    routing::get,
    Router,
};
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use handlers::homepage;
use crate::handlers::upload;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    init_datadir();

    // build our application with some routes
    let app = Router::new()
        .route("/", get(homepage::show_form).post(upload::accept_form))
        .route("/uploads/{uuid}", get(handlers::download::download_html))
        .route("/uploads/{uuid}/download", get(handlers::download::download_file))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            1024 * 1024 * 1024 * 5, /* 5gb */
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

pub fn init_datadir() {
    fs::create_dir_all("data").unwrap();
    fs::write("./data/CACHEDIR.TAG", "Signature: 8a477f597d28d172789f06886806bc55").unwrap();
}