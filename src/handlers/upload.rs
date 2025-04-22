use crate::error_compat::ApiResult;
use crate::error_compat::error_compat::InternalServerErrorExt;
use crate::file_meta::FileMeta;
use crate::retention_control::delete_asset;
use axum::extract::Multipart;
use axum::extract::multipart::Field;
use axum::response::{IntoResponse, Redirect};
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub async fn accept_form(multipart: Multipart) -> ApiResult<impl IntoResponse> {
    let uuid = Uuid::new_v4();
    let out_dir = format!("data/{uuid}");
    fs::create_dir(&out_dir).await.unwrap();

    let res = do_upload(multipart, &out_dir).await;
    if res.is_err() {
        delete_asset(uuid).await.ise()?;
    }
    res.ise()?;

    Ok(Redirect::to(&format!("/uploads/{}", uuid)))
}

async fn do_upload(mut multipart: Multipart, out_dir: &str) -> ApiResult<impl IntoResponse> {
    while let Some(mut field) = multipart.next_field().await.unwrap() {
        dbg!(&field);
        let name = field.name().unwrap().to_string();
        match name.as_str() {
            "file" => {payload_field(field, out_dir).await?}
            "expiration" => {
                todo!()
            }
            _ => {
                panic!("Unknown field {name}")
            }
        }
    }
    Ok(())
}

async fn payload_field(mut field: Field<'_>, out_dir: &str) -> ApiResult<()> {
    let file_name = field.file_name().unwrap().to_string();
    let content_type = field.content_type().unwrap().to_string();

    let mut metadata = FileMeta::default_policy(file_name.clone(), content_type.clone());
    let mut f = File::create(format!("{out_dir}/{file_name}"))
        .await
        .unwrap();
    while let Some(chunk) = field.chunk().await.unwrap() {
        metadata.add_size(chunk.len() as u64);
        f.write_all(&chunk).await.ise()?;
    }
    fs::write(
        format!("{out_dir}/meta.toml"),
        toml::to_string_pretty(&metadata).unwrap().as_bytes(),
    )
    .await
    .unwrap();
    Ok(())
}
