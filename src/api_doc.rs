use crate::handlers::delete::__path_request_delete_asset;
use crate::upload::__path_progress_token_to_id;
use crate::handlers::download::__path_download_file;
use crate::upload::__path_accept_form;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(accept_form, download_file, progress_token_to_id, request_delete_asset), info(title = "Tapfer API", version = "1.0"))]
pub struct ApiDoc;
