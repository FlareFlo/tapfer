use axum_extra::extract::Host;
use crate::configuration::UPLOAD_BUFSIZE;
use crate::error::{TapferError, TapferResult};
use crate::file_meta::{FileMeta, FileMetaBuilder, RemovalPolicy};
use crate::retention_control::delete_asset;
use crate::tapfer_id::TapferId;
use crate::updown::upload_handle::UploadHandle;
use crate::updown::upload_pool::UploadFsm;
use crate::{PROGRESS_TOKEN_LUT, UPLOAD_POOL};
use axum::body::Body;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Html;
use axum::response::{IntoResponse, Redirect, Response};
use futures_util::TryStreamExt;
use scopeguard::defer;
use std::io::Error;
use std::pin::{Pin, pin};
use std::str::FromStr;
use std::task::{Context, Poll};
use time::Duration as TimeDuration;
use tokio::fs::File;
use tokio::io::{AsyncWrite, BufReader, copy_buf};
use tokio::{fs, task};
use tokio_util::io::StreamReader;
use tracing::{debug, error, info, warn};

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

#[utoipa::path(
    post,
    path = "/",
    params(
        ("tapfer-source" = Option<String>, Header, description = "`frontend` when using frontend, unset otherwise"),
        ("tapfer-file-size" = Option<u64>, Header, description = "optional file size of asset"),
        ("tapfer-progress-token" = Option<u32>, Header, description = "random ID to associate upload with frontend"),
        ("tapfer-timezone" = Option<String>, Header, description = "client timezone in IANA string format, UTC otherwise"),
        ("tapfer-expiration" = Option<String>, Header, description = "Expiration either as `single_download` or `24_hours`"),
    ),
    responses(
        (status = 303, description = "Page to asset when using frontend"),
        (status = 200, description = "URL to asset when using CURL or similar"),
    ),
)]
#[axum::debug_handler]
pub async fn accept_form(
    headers: HeaderMap,
    Host(host): Host,
    multipart: Multipart,
) -> TapferResult<impl IntoResponse> {
    let id = TapferId::new_random();
    fs::create_dir(&format!("data/{id}")).await?;

    info!("Beginning upload of {id}");
    let res = do_upload(&headers, multipart, id).await;
    if res.is_err() {
        delete_asset(id).await?;
    }
    res?;
    info!("Completed upload of {id}");

    dbg!(&host);

    let method = if host.contains("localhost") {
        ""
    } else {
        "https://"
    };
    let source = match headers
        .get("tapfer-source")
        .map(|e| e.to_str())
        .transpose()?
    {
        Some("frontend") => RequestSource::Frontend(Redirect::to(&format!("{method}{host}/uploads/{id}"))),
        _ => {
            RequestSource::Unknown(Body::new(format!("{method}{host}/uploads/{id}\n")))
        }
    };
    Ok(source)
}

async fn do_upload(
    headers: &HeaderMap,
    mut multipart: Multipart,
    id: TapferId,
) -> TapferResult<()> {
    let mut meta = FileMetaBuilder::default();

    let header = |header: &str| headers.get(header).map(|h| h.to_str()).transpose();

    let size: Option<u64> = header("tapfer-file-size")?
        .map(|h| h.parse::<u64>())
        .transpose()?;
    let in_progress_token: Option<u32> = header("tapfer-progress-token")?
        .map(|h| h.parse())
        .transpose()?;

    if let Some(tz) = header("tapfer-timezone")? {
        meta.timezone = Some(tz.to_owned());
    } else {
        error!("Missing tapfer-timezone header");
    }

    if size.is_some() != in_progress_token.is_some() {
        warn!(
            "Size is {size:?} and progress token is {in_progress_token:?}. The frontend might not be sending both?"
        );
    }

    expiration_field(headers.get("tapfer-expiration"), &mut meta).await?;

    if let Some(tok) = in_progress_token {
        info!("Adding progress token {tok}");
        PROGRESS_TOKEN_LUT.insert(tok, id);
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
                payload_field(field, id, meta.clone(), size).await?;
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
    id: TapferId,
    metadata_builder: FileMetaBuilder,
    size: Option<u64>,
) -> TapferResult<()> {
    let file_name = field
        .file_name()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| id.to_string());
    let content_type = field
        .content_type()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM.as_ref())
        .to_string();

    let metadata = metadata_builder.build(file_name.clone(), content_type.clone(), size);
    // Only permit updown stream when the files final size was transmitted by the client
    let handle = UPLOAD_POOL.handle(id, metadata.clone());
    let f = File::create(format!("data/{id}/{file_name}")).await?;
    let mut f = UpdownWriter::new(f, handle.clone(), metadata, size.is_none());
    let mut s = BufReader::with_capacity(
        UPLOAD_BUFSIZE,
        StreamReader::new(field.map_err(TapferError::AxumMultipart)),
    );
    copy_buf(&mut s, &mut f).await?;
    let metadata = f.metadata();
    fs::write(
        format!("data/{id}/meta.toml"),
        toml::to_string_pretty(&metadata)?.as_bytes(),
    )
    .await?;
    // The upload is complete, mark the upload as complete
    handle.write_fsm().await.mark_complete();
    Ok(())
}

async fn expiration_field(
    field: Option<&HeaderValue>,
    meta: &mut FileMetaBuilder,
) -> TapferResult<()> {
    let Some(f) = field.map(|f| f.to_str()).transpose()? else {
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

#[utoipa::path(
    get,
    path = "/uploads/query_id/{token}",
    responses(
        (status = 200, description = "UUID of asset"),
        (status = 404, description = "Token matches no (running) asset"),
    ),

)]
pub async fn progress_token_to_id(Path(path): Path<String>) -> TapferResult<impl IntoResponse> {
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
        if *self.upload_handle.read_fsm_blocking() == UploadFsm::Failed {
            return Poll::Ready(Err(TapferError::Custom {
                status_code: StatusCode::NOT_FOUND,
                body: Html("upload failed".to_owned()),
            }
            .into()));
        }

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
                let e = handle.write_fsm().await.add_progress(n);
                if e.is_err() {
                    error!("Failed to add progress, fsm is already marked as completed?");
                }
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
        let mut fsm = self.upload_handle.write_fsm_blocking();
        if !fsm.is_complete() {
            *fsm = UploadFsm::Failed;
        }
    }
}
