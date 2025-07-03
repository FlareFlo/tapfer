mod case_insensitive_path;
mod configuration;
mod error;
mod file_meta;
mod handlers;
mod retention_control;
mod updown;

use crate::case_insensitive_path::lowercase_path_middleware;
use crate::configuration::MAX_UPLOAD_SIZE;
use crate::error::TapferResult;
use crate::handlers::upload;
use crate::retention_control::{GlobalRetentionPolicy, check_all_assets};
use crate::updown::upload_pool::UploadPool;
use axum::routing::{get_service, post};
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use dashmap::DashMap;
use handlers::homepage;
use std::sync::LazyLock;
use std::time::Duration;
use std::{env, fs};
use tokio::time::sleep;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

pub static PROGRESS_TOKEN_LUT: LazyLock<DashMap<u32, Uuid>> = LazyLock::new(DashMap::new);
pub static GLOBAL_RETENTION_POLICY: LazyLock<GlobalRetentionPolicy> =
    LazyLock::new(GlobalRetentionPolicy::default);
pub static UPLOAD_POOL: LazyLock<UploadPool> = LazyLock::new(UploadPool::new);

#[tokio::main]
async fn main() -> TapferResult<()> {
    if env::var("HOST").is_err() {
        panic!(
            "Please set the environment variable HOST containing the domain tapfer is served on"
        );
    }

    ctrlc::set_handler(move || {
        error!("Caught CTRL-C... Exiting right away");
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    init_datadir();

    let static_dir_service = get_service(ServeDir::new("static"));

    let lowercase_router =
        Router::new().route("/uploads/{uuid}", get(handlers::download::download_html));

    let lowercase_service = ServiceBuilder::new()
        // We lowercase the path as QR codes will ship them uppercase
        .layer(middleware::from_fn(lowercase_path_middleware))
        .service(lowercase_router);

    // build our application with some routes
    let app = Router::new()
        .route("/", get(homepage::show_form).post(upload::accept_form))
        .route(
            "/uploads/{uuid}/delete",
            post(handlers::delete::request_delete_asset),
        )
        .route(
            "/uploads/query_uuid/{token}",
            get(handlers::upload::progress_token_to_uuid),
        )
        .route(
            "/uploads/{uuid}/download",
            get(handlers::download::download_file),
        )
        .route("/qrcg/{uuid}", get(handlers::qrcode::get_qrcode_from_uuid))
        .route(
            "/qrcg/placeholder.png",
            get(handlers::qrcode::get_placeholder_qrcode),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(MAX_UPLOAD_SIZE))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .nest_service("/static", static_dir_service)
        .fallback_service(lowercase_service);

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
