mod api_doc;
mod case_insensitive_path;
mod configuration;
mod error;
mod file_meta;
mod handlers;
mod retention_control;
mod tapfer_id;
mod updown;

use tower_http::cors::AllowOrigin;
use tower_http::cors::CorsLayer;
use crate::api_doc::ApiDoc;
use crate::case_insensitive_path::lowercase_path_middleware;
use crate::configuration::MAX_UPLOAD_SIZE;
use crate::error::TapferResult;
use crate::handlers::upload;
use crate::retention_control::{GlobalRetentionPolicy, check_all_assets};
use crate::tapfer_id::TapferId;
use crate::updown::upload_pool::UploadPool;
use axum::routing::{get_service, post};
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use dashmap::DashMap;
use handlers::homepage;
use std::sync::LazyLock;
use std::time::Duration;
use std::{env, fs};
use axum::extract::Request;
use axum::response::IntoResponse;
use http::{HeaderValue, Method};
use tokio::time::sleep;
use tower::{ServiceBuilder};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

pub static PROGRESS_TOKEN_LUT: LazyLock<DashMap<u32, TapferId>> = LazyLock::new(DashMap::new);
pub static GLOBAL_RETENTION_POLICY: LazyLock<GlobalRetentionPolicy> =
    LazyLock::new(GlobalRetentionPolicy::default);
pub static UPLOAD_POOL: LazyLock<UploadPool> = LazyLock::new(UploadPool::new);

async fn handle_options<B>(_req: Request<B>) -> impl IntoResponse {
    (axum::http::StatusCode::NO_CONTENT, "")
}

#[tokio::main]
async fn main() -> TapferResult<()> {
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

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_origin(AllowOrigin::list([HeaderValue::from_static("https://tapfer.lkl.lol"), HeaderValue::from_static("https://cdn.tapfer.lkl.lol")]));

    let lowercase_router =
        Router::new()
            .route("/uploads/{id}", get(handlers::download::download_html).options(handle_options));

    let fallback_service = ServiceBuilder::new()
        // We lowercase the path as QR codes will ship them uppercase
        .layer(middleware::from_fn(lowercase_path_middleware))
        .service(lowercase_router);

    // build our application with some routes
    let app = Router::new()
        .route("/", get(homepage::show_form).post(upload::accept_form))
        .route(
            "/uploads/{id}/delete",
            post(handlers::delete::request_delete_asset),
        )
        .route(
            "/uploads/query_id/{token}",
            get(handlers::upload::progress_token_to_id),
        )
        .route(
            "/uploads/{id}/download",
            get(handlers::download::download_file),
        )
        .route("/qrcg/{id}", get(handlers::qrcode::get_qrcode_from_id))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(MAX_UPLOAD_SIZE))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .nest_service("/static", static_dir_service)
        .merge(Scalar::with_url("/docs", <ApiDoc as OpenApi>::openapi()))
        .layer(cors)
        .fallback_service(fallback_service);

    let main_service = ServiceBuilder::new()
        .service(app);

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);

    tokio::spawn(async {
        loop {
            // TODO: Handle errors in a better way
            info!("Checking for stale assets");
            let e = check_all_assets().await;
            if e.is_err() {
                error!("Checking assets failed: {:?}", e);
            }

            sleep(Duration::from_secs_f64(
                GLOBAL_RETENTION_POLICY.recheck_interval.as_seconds_f64(),
            ))
            .await;
        }
    });

    axum::serve(listener, main_service).await?;
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
