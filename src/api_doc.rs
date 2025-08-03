use crate::upload::__path_accept_form;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(accept_form), info(title = "Tapfer API", version = "1.0"))]
pub struct ApiDoc;
