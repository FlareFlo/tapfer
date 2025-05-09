use std::str::FromStr;
use std::sync::LazyLock;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, FileMetaBuilder, RemovalPolicy};
use crate::retention_control::delete_asset;
use crate::upload_pool::{UploadHandle, UPLOAD_POOL};
use axum::extract::{Multipart, Path};
use axum::extract::multipart::Field;
use axum::http::header::CONTENT_LENGTH;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use std::time::Duration as StdDuration;
use dashmap::DashMap;
use scopeguard::defer;
use time::Duration as TimeDuration;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

pub static PROGRESS_TOKEN_LUT: LazyLock<DashMap<u32, Uuid>> = LazyLock::new(|| DashMap::new());

pub async fn accept_form(multipart: Multipart) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::new_v4();
    fs::create_dir(&format!("data/{uuid}")).await?;

    let res = do_upload(multipart, uuid).await;
    if res.is_err() {
        delete_asset(uuid).await?;
    }
    res?;

    Ok(Redirect::to(&format!("/uploads/{}", uuid)))
}

async fn do_upload(mut multipart: Multipart, uuid: Uuid) -> TapferResult<impl IntoResponse> {
    let mut meta = FileMetaBuilder::default();
    let mut got_file = false;
    let ensure_file_last = |got_file| {
        if got_file {
            Err(TapferError::BadMultipartOrder)
        } else {
            Ok(())
        }
    };
    let mut size: Option<u64> = None;
    let mut in_progress_token: Option<u32> = None;
    while let Some(field) = multipart.next_field().await? {
        let name = field
            .name()
            .ok_or(TapferError::MultipartFieldNameMissing)?
            .to_string();
        match name.as_str() {
            "file" => {
                if size.is_some() != in_progress_token.is_some() {  
                    warn!("Size is {size:?} and progress token is {in_progress_token:?}. The frontend might not be sending both?");
                }
                payload_field(field, uuid, meta.clone(), size, in_progress_token).await?;
                got_file = true;
            }
            "expiration" => {
                expiration_field(field, &mut meta).await?;
                ensure_file_last(got_file)?;
            }
            "file_size" => {
                size = Some(field.text().await?.parse()?);
            }
            "in_progress_token" => {
                in_progress_token = Some(field.text().await?.parse()?);
                info!("Adding progress token {in_progress_token:?}");
                PROGRESS_TOKEN_LUT.insert(in_progress_token.expect("infallible"), uuid);
            }
            _ => {
                Err(TapferError::UnknownMultipartField {
                    field_name: name.to_owned(),
                })?;
            }
        }
    }
    defer!{
        if let Some(t) = in_progress_token {
            info!("deleting progress token {t}");
            PROGRESS_TOKEN_LUT.remove(&t);
        }
    }
    Ok(())
}

async fn payload_field(
    mut field: Field<'_>,
    uuid: Uuid,
    metadata_builder: FileMetaBuilder,
    size: Option<u64>,
    in_progress_token: Option<u32>,
) -> TapferResult<()> {
    let file_name = field.file_name().unwrap().to_string();
    let content_type = field.content_type().unwrap().to_string();

    let mut metadata = metadata_builder.build(file_name.clone(), content_type.clone(), size);
    // Only permit updown stream when the files final size was transmitted by the client
    let handle = UPLOAD_POOL.handle(uuid, metadata.clone());
    let mut f = File::create(format!("data/{uuid}/{file_name}")).await?;
    while let Some(chunk) = field.chunk().await? {
        f.write_all(&chunk).await?;

        // Increment the updown handle when applicable, otherwise, keep track of the dynamic filesize
        handle.add_progress(chunk.len()).await;
        if size.is_none() {
            metadata.add_size(chunk.len() as _)?;
        }
        // sleep(StdDuration::from_micros(2000)).await; // Debug slowdown for live upload and download
    }
    fs::write(
        format!("data/{uuid}/meta.toml"),
        toml::to_string_pretty(&metadata)?.as_bytes(),
    )
    .await?;
    // The upload is complete, mark the upload as complete
    handle.mark_complete().await;
    Ok(())
}

async fn expiration_field(field: Field<'_>, meta: &mut FileMetaBuilder) -> TapferResult<()> {
    let text = field.text().await?;
    match text.as_str() {
        "single_download" => meta.expiration = Some(RemovalPolicy::SingleDownload),
        "24_hours" => {
            meta.expiration = Some(RemovalPolicy::Expiry {
                after: TimeDuration::hours(24),
            })
        }
        _ => {}
    }
    Ok(())
}

pub async fn progress_token_to_uuid(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let token = u32::from_str(&path)?;
    Ok(PROGRESS_TOKEN_LUT.get(&token).ok_or(TapferError::TokenDoesNotExist(token))?.to_string())
}