use axum::extract::Multipart;
use axum::response::{IntoResponse, Redirect};
use axum::http::StatusCode;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use crate::file_meta::FileMeta;

pub async fn accept_form(multipart: Multipart) -> Result<impl IntoResponse, StatusCode> {
    let uuid = Uuid::new_v4();
    let out_dir = format!("data/{uuid}");
    fs::create_dir(&out_dir).await.unwrap();

    let res = do_upload(multipart, &out_dir).await;
    if res.is_err() {
        fs::remove_dir_all(&out_dir).await.unwrap();
    }
    res?;

    Ok(Redirect::to(&format!("/uploads/{}", uuid)))
}

async fn do_upload(mut multipart: Multipart, out_dir: &str) -> Result<impl IntoResponse, StatusCode> {
    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();

        println!(
            "`{name}` (`{file_name}`: `{content_type}`)",
        );

        let mut metadata = FileMeta::default_policy(file_name.clone(), content_type.clone());
        let mut f = File::create(format!("{out_dir}/{file_name}")).await.unwrap();
        while let Some(chunk) = field.chunk().await.unwrap() {
            metadata.add_size(chunk.len() as u64);
            f.write_all(&chunk).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        fs::write(format!("{out_dir}/meta.toml"), toml::to_string_pretty(&metadata).unwrap().as_bytes()).await.unwrap();
    }
    Ok(())
}