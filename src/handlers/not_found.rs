use askama::Template;
use crate::configuration::EMBED_DESCRIPTION;

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound {
	embed_image_url: &'static str,
	embed_description: &'static str,
}

impl Default for NotFound {
	fn default() -> Self {
		Self {
			embed_image_url: "/graphics/favicon.ico",
			embed_description: EMBED_DESCRIPTION,
		}
	}
}