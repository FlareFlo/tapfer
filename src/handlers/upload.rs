use crate::file_meta::{FileMetaBuilder, RemovalPolicy};
use crate::retention_control::delete_asset;
use axum::extract::Multipart;
use axum::extract::multipart::Field;
use axum::response::{IntoResponse, Redirect};
use time::Duration;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use crate::error::{TapferError, TapferResult};

pub async fn accept_form(multipart: Multipart) -> TapferResult<impl IntoResponse> {
    let uuid = Uuid::new_v4();
    let out_dir = format!("data/{uuid}");
    fs::create_dir(&out_dir).await?;

    let res = do_upload(multipart, &out_dir).await;
    if res.is_err() {
        delete_asset(uuid).await?;
    }
    res?;

    Ok(Redirect::to(&format!("/uploads/{}", uuid)))
}

async fn do_upload(mut multipart: Multipart, out_dir: &str) -> TapferResult<impl IntoResponse> {
    let mut meta = FileMetaBuilder::default();
    let mut got_file = false;
    let ensure_file_last = |got_file| {
        if got_file {
            Err(TapferError::BadMultipartOrder)
        } else {
            Ok(())
        }
    };
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().ok_or(TapferError::MultipartFieldNameMissing)?.to_string();
        match name.as_str() {
            "file" => {
                payload_field(field, out_dir, meta.clone()).await?;
                got_file = true;
            }
            "expiration" => {
                expiration_field(field, &mut meta).await?;
                ensure_file_last(got_file)?;
            }
            _ => {
                Err(TapferError::UnknownMultipartField {
                    field_name: name.to_owned(),
                })?;
            }
        }
    }
    Ok(())
}

async fn payload_field(
    mut field: Field<'_>,
    out_dir: &str,
    metadata_builder: FileMetaBuilder,
) -> TapferResult<()> {
    let file_name = field.file_name().unwrap().to_string();
    let content_type = field.content_type().unwrap().to_string();

    let mut metadata = metadata_builder.build(file_name.clone(), content_type.clone());
    let mut f = File::create(format!("{out_dir}/{file_name}"))
        .await?;
    while let Some(chunk) = field.chunk().await? {
        metadata.add_size(chunk.len() as u64);
        f.write_all(&chunk).await?;
    }
    fs::write(
        format!("{out_dir}/meta.toml"),
        toml::to_string_pretty(&metadata)?.as_bytes(),
    )
    .await?;
    Ok(())
}

async fn expiration_field(field: Field<'_>, meta: &mut FileMetaBuilder) -> TapferResult<()> {
    let text = field.text().await?;
    match text.as_str() {
        "single_download" => meta.expiration = Some(RemovalPolicy::SingleDownload),
        "24_hours" => {
            meta.expiration = Some(RemovalPolicy::Expiry {
                after: Duration::hours(24),
            })
        }
        _ => {}
    }
    Ok(())
}
