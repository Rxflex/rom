use serde::{Deserialize, Serialize};
use ureq::{RequestExt, ResponseExt, http};

#[derive(Debug, Deserialize)]
pub struct FetchRequestPayload {
    pub url: String,
    pub method: String,
    #[serde(default = "default_redirect_mode")]
    pub redirect_mode: String,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct FetchResponsePayload {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub redirected: bool,
    pub is_redirect_response: bool,
    pub headers: Vec<HeaderEntry>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

fn default_redirect_mode() -> String {
    "follow".to_owned()
}

pub fn perform_fetch(payload: &str) -> Result<String, String> {
    let request: FetchRequestPayload =
        serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let method = request
        .method
        .parse::<http::Method>()
        .map_err(|error| error.to_string())?;

    let mut builder = http::Request::builder()
        .method(method)
        .uri(request.url.as_str());

    for header in &request.headers {
        builder = builder.header(header.name.as_str(), header.value.as_str());
    }

    let http_request = builder
        .body(request.body.clone())
        .map_err(|error| error.to_string())?;

    let max_redirects = if request.redirect_mode == "follow" {
        10
    } else {
        0
    };
    let mut response = http_request
        .with_default_agent()
        .configure()
        .http_status_as_error(false)
        .max_redirects(max_redirects)
        .save_redirect_history(true)
        .run()
        .map_err(|error| error.to_string())?;

    let final_url = response.get_uri().to_string();
    let redirected = final_url != request.url;
    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("").to_owned();
    let is_redirect_response = status.is_redirection();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| HeaderEntry {
            name: name.to_string(),
            value: value.to_str().unwrap_or_default().to_owned(),
        })
        .collect();
    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| error.to_string())?;

    serde_json::to_string(&FetchResponsePayload {
        url: final_url,
        status: status.as_u16(),
        status_text,
        redirected,
        is_redirect_response,
        headers,
        body,
    })
    .map_err(|error| error.to_string())
}
