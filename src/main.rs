mod error;
mod file_meta;
mod handlers;
mod retention_control;
mod upload_pool;

use crate::error::TapferResult;
use crate::handlers::upload;
use crate::retention_control::{GLOBAL_RETENTION_POLICY, check_all_assets};
use axum::{Router, extract::DefaultBodyLimit, routing::get};
use handlers::homepage;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> TapferResult<()> {
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
        .route("/uploads/query_uuid/{token}", get(handlers::upload::progress_token_to_uuid))
        .route(
            "/uploads/{uuid}/download",
            get(handlers::download::download_file),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            1024 * 1024 * 1024 * 100, /* 5gb */
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);

    tokio::spawn(async {
        loop {
            // TODO: Handle errors
            info!("Checking for stale assets");
            check_all_assets().await.unwrap();

            sleep(Duration::from_secs_f64(
                GLOBAL_RETENTION_POLICY.recheck_interval.as_seconds_f64(),
            ))
            .await;
        }
    });

    axum::serve(listener, app).await?;
    Ok(())
}

pub fn init_datadir() {
    fs::create_dir_all("data").unwrap();
    fs::write(
        "./data/CACHEDIR.TAG",
        "Signature: 8a477f597d28d172789f06886806bc55",
    )
    .unwrap();
}
