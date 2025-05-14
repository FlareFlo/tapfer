use crate::updown::upload_handle::UploadHandle;
use crate::updown::upload_pool::UPLOAD_POOL;
use crate::configuration::UPLOAD_BUFSIZE;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, FileMetaBuilder, RemovalPolicy};
use crate::retention_control::delete_asset;
use axum::body::Body;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect, Response};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use scopeguard::defer;
use std::env;
use std::io::Error;
use std::pin::{Pin, pin};
use std::str::FromStr;
use std::sync::LazyLock;
use std::task::{Context, Poll};
use std::time::Duration;
use time::Duration as TimeDuration;
use tokio::fs::File;
use tokio::io::{AsyncWrite, BufReader, copy_buf};
use tokio::{fs, task};
use tokio::task::block_in_place;
use tokio_util::io::StreamReader;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug)]
enum RequestSource {
    Frontend(Redirect),
    Unknown(Body),
}

impl IntoResponse for RequestSource {
    fn into_response(self) -> Response {
        match self {
            RequestSource::Frontend(r) => r.into_response(),
            RequestSource::Unknown(b) => Response::new(b),
        }
    }
}

pub static PROGRESS_TOKEN_LUT: LazyLock<DashMap<u32, Uuid>> = LazyLock::new(DashMap::new);

#[axum::debug_handler]
pub async fn accept_form(
    headers: HeaderMap,
    multipart: Multipart,
) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::new_v4();
    fs::create_dir(&format!("data/{uuid}")).await?;

    info!("Beginning upload of {uuid}");
    let res = do_upload(&headers, multipart, uuid).await;
    if res.is_err() {
        delete_asset(uuid).await?;
    }
    res?;
    info!("Completed upload of {uuid}");

    let source = match headers
        .get("tapfer-source")
        .map(|e| e.to_str())
        .transpose()?
    {
        Some("frontend") => RequestSource::Frontend(Redirect::to(&format!("/uploads/{uuid}"))),
        _ => {
            let host = env::var("HOST").expect("Should ok as main checks this var already");
            let method = if host.contains("localhost") {
                "https://"
            } else {
                ""
            };
            RequestSource::Unknown(Body::new(format!("{method}{host}/uploads/{uuid}\n")))
        }
    };
    Ok(source)
}

async fn do_upload(headers: &HeaderMap, mut multipart: Multipart, uuid: Uuid) -> TapferResult<()> {
    let mut meta = FileMetaBuilder::default();

    let size: Option<u64> = headers
        .get("tapfer-file-size")
        .map(|h| h.to_str())
        .transpose()?
        .map(|h| h.parse::<u64>())
        .transpose()?;
    let in_progress_token: Option<u32> = headers
        .get("tapfer-progress-token")
        .map(|h| h.to_str())
        .transpose()?
        .map(|h| h.parse())
        .transpose()?;

    if size.is_some() != in_progress_token.is_some() {
        warn!(
            "Size is {size:?} and progress token is {in_progress_token:?}. The frontend might not be sending both?"
        );
    }

    expiration_field(headers.get("tapfer-expiration"), &mut meta).await?;

    if let Some(tok) = in_progress_token {
        info!("Adding progress token {tok}");
        PROGRESS_TOKEN_LUT.insert(tok, uuid);
    }
    defer! {
        if let Some(t) = in_progress_token {
            info!("deleting progress token {t}");
            PROGRESS_TOKEN_LUT.remove(&t);
        }
    }

    while let Some(field) = multipart.next_field().await? {
        let name = field
            .name()
            .ok_or(TapferError::MultipartFieldNameMissing)?
            .to_string();
        debug!("reading field {name}");
        match name.as_str() {
            "file" => {
                payload_field(field, uuid, meta.clone(), size).await?;
            }
            _ => {
                error!("Got unexpected form field {name}");
                Err(TapferError::UnknownMultipartField {
                    field_name: name.to_owned(),
                })?;
            }
        }
    }
    Ok(())
}

async fn payload_field(
    field: Field<'_>,
    uuid: Uuid,
    metadata_builder: FileMetaBuilder,
    size: Option<u64>,
) -> TapferResult<()> {
    let file_name = field
        .file_name()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| uuid.to_string());
    let content_type = field.content_type().unwrap_or("unknown").to_string();

    let metadata = metadata_builder.build(file_name.clone(), content_type.clone(), size);
    // Only permit updown stream when the files final size was transmitted by the client
    let handle = UPLOAD_POOL.handle(uuid, metadata.clone());
    let f = File::create(format!("data/{uuid}/{file_name}")).await?;
    let mut f = UpdownWriter::new(f, handle.clone(), metadata, size.is_none());
    let mut s = BufReader::with_capacity(
        UPLOAD_BUFSIZE,
        StreamReader::new(field.map_err(TapferError::AxumMultipart)),
    );
    copy_buf(&mut s, &mut f).await?;
    let metadata = f.metadata();
    fs::write(
        format!("data/{uuid}/meta.toml"),
        toml::to_string_pretty(&metadata)?.as_bytes(),
    )
    .await?;
    // The upload is complete, mark the upload as complete
    handle.mark_complete().await;
    Ok(())
}

async fn expiration_field(
    field: Option<&HeaderValue>,
    meta: &mut FileMetaBuilder,
) -> TapferResult<()> {
    let f = if let Some(f) = field {
        f.to_str()?
    } else {
        return Ok(());
    };
    match f {
        "single_download" => meta.expiration = Some(RemovalPolicy::SingleDownload),
        "24_hours" => {
            meta.expiration = Some(RemovalPolicy::Expiry {
                after: TimeDuration::hours(24),
            })
        }
        _ => {
            Err(TapferError::InvalidExpiration(f.to_owned()))?;
        }
    }
    Ok(())
}

pub async fn progress_token_to_uuid(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
    let token = u32::from_str(&path)?;
    Ok(PROGRESS_TOKEN_LUT
        .get(&token)
        .ok_or(TapferError::TokenDoesNotExist(token))?
        .to_string())
}

pub struct UpdownWriter<S> {
    file: S,
    upload_handle: UploadHandle,
    metadata: FileMeta,
    write_to_meta: bool,
}

impl<S> UpdownWriter<S> {
    pub fn new(
        file: S,
        upload_handle: UploadHandle,
        metadata: FileMeta,
        write_to_meta: bool,
    ) -> Self {
        Self {
            file,
            upload_handle,
            metadata,
            write_to_meta,
        }
    }

    pub fn metadata(&self) -> &FileMeta {
        &self.metadata
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for UpdownWriter<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let mut pinned = pin!(&mut self.file);
        let pollres = pinned.as_mut().poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = pollres {
            if self.write_to_meta {
                self.metadata
                    .add_size(n as u64)
                    .expect("since write_to_meta is set this should not panic");
            }
            let handle = self.upload_handle.clone();
            task::spawn(async move {
                handle.add_progress(n).await;
                handle.notify_all_downloaders();
            });
        }
        pollres
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut pinned = pin!(&mut self.file);
        pinned.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut pinned = pin!(&mut self.file);
        pinned.as_mut().poll_shutdown(cx)
    }
}

impl<S> Drop for UpdownWriter<S> {
    fn drop(&mut self) {
        if !self.upload_handle.is_complete_blocking() {
            self.upload_handle.set_upload_failed();
        }
    }
}
