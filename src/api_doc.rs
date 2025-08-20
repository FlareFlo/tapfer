use crate::handlers::delete::__path_request_delete_asset;
use crate::handlers::download::__path_download_file;
use crate::handlers::qrcode::__path_get_qrcode_from_id;
use crate::upload::__path_accept_form;
use crate::upload::__path_progress_token_to_id;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        accept_form,
        download_file,
        progress_token_to_id,
        request_delete_asset,
        get_qrcode_from_id
    ),
    info(title = "Tapfer API", version = "1.0")
)]
pub struct ApiDoc;
