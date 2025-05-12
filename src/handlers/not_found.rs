use askama::Template;

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound {
	embed_image_url: &'static str,
}

impl Default for NotFound {
	fn default() -> Self {
		Self {
			embed_image_url: "/graphics/favicon.ico",
		}
	}
}