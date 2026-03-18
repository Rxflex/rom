use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct UrlParsePayload {
    pub input: String,
    pub base: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UrlParseResult {
    pub href: String,
    pub origin: String,
    pub protocol: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

pub fn parse_url(payload: &str) -> Result<String, String> {
    let payload: UrlParsePayload =
        serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let url = match payload.base {
        Some(base) => {
            let base = Url::parse(&base).map_err(|error| error.to_string())?;
            base.join(&payload.input)
                .map_err(|error| error.to_string())?
        }
        None => Url::parse(&payload.input).map_err(|error| error.to_string())?,
    };

    let port = url
        .port()
        .map(|value| value.to_string())
        .unwrap_or_default();
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
        None => url.host_str().unwrap_or_default().to_owned(),
    };
    let search = url
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let hash = url
        .fragment()
        .map(|fragment| format!("#{fragment}"))
        .unwrap_or_default();

    serde_json::to_string(&UrlParseResult {
        href: url.as_str().to_owned(),
        origin: url.origin().ascii_serialization(),
        protocol: format!("{}:", url.scheme()),
        username: url.username().to_owned(),
        password: url.password().unwrap_or_default().to_owned(),
        host,
        hostname: url.host_str().unwrap_or_default().to_owned(),
        port,
        pathname: url.path().to_owned(),
        search,
        hash,
    })
    .map_err(|error| error.to_string())
}
