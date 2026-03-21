use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::env;
use url::Url;

#[derive(Clone, Debug)]
pub enum ProxyKind {
    Http,
    Socks5,
}

#[derive(Clone, Debug)]
pub struct ProxyCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    pub authorization: Option<String>,
    pub credentials: Option<ProxyCredentials>,
}

impl ProxyConfig {
    pub fn uses_http_connect(&self) -> bool {
        matches!(self.kind, ProxyKind::Http)
    }

    pub fn uses_absolute_form_for_http(&self) -> bool {
        matches!(self.kind, ProxyKind::Http)
    }
}

pub fn resolve_proxy(
    target_url: &Url,
    explicit_proxy_url: Option<&str>,
) -> Result<Option<ProxyConfig>, String> {
    if should_bypass_proxy(target_url) {
        return Ok(None);
    }

    let candidate = explicit_proxy_url
        .map(str::to_owned)
        .or_else(|| read_proxy_from_env(target_url.scheme()));

    match candidate {
        Some(proxy_url) => parse_proxy_url(&proxy_url).map(Some),
        None => Ok(None),
    }
}

fn read_proxy_from_env(scheme: &str) -> Option<String> {
    let candidates: &[&str] = if scheme.eq_ignore_ascii_case("https") {
        &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
    } else {
        &[
            "HTTP_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
        ]
    };

    candidates
        .iter()
        .find_map(|name| env::var(name).ok())
        .filter(|value| !value.trim().is_empty())
}

fn should_bypass_proxy(target_url: &Url) -> bool {
    let Some(host) = target_url.host_str() else {
        return true;
    };

    let no_proxy = env::var("NO_PROXY")
        .ok()
        .or_else(|| env::var("no_proxy").ok())
        .unwrap_or_default();

    no_proxy
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|entry| host_matches_no_proxy(host, entry))
}

fn host_matches_no_proxy(host: &str, entry: &str) -> bool {
    if entry == "*" {
        return true;
    }

    let normalized_entry = entry.strip_prefix('.').unwrap_or(entry);
    host.eq_ignore_ascii_case(normalized_entry)
        || host
            .to_ascii_lowercase()
            .ends_with(&format!(".{}", normalized_entry.to_ascii_lowercase()))
}

fn parse_proxy_url(proxy_url: &str) -> Result<ProxyConfig, String> {
    let parsed = Url::parse(proxy_url).map_err(|error| error.to_string())?;
    let kind = parse_proxy_kind(parsed.scheme())?;

    let host = parsed
        .host_str()
        .ok_or_else(|| "Proxy URL is missing host.".to_owned())?
        .to_owned();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "Proxy URL is missing port.".to_owned())?;
    let credentials = if parsed.username().is_empty() {
        None
    } else {
        Some(ProxyCredentials {
            username: parsed.username().to_owned(),
            password: parsed.password().unwrap_or_default().to_owned(),
        })
    };
    let authorization = credentials.as_ref().and_then(|credentials| {
        matches!(kind, ProxyKind::Http).then(|| {
            let basic = format!("{}:{}", credentials.username, credentials.password);
            format!("Basic {}", STANDARD.encode(basic))
        })
    });

    Ok(ProxyConfig {
        kind,
        host,
        port,
        authorization,
        credentials,
    })
}

fn parse_proxy_kind(scheme: &str) -> Result<ProxyKind, String> {
    if scheme.eq_ignore_ascii_case("http") {
        return Ok(ProxyKind::Http);
    }

    if scheme.eq_ignore_ascii_case("socks5") || scheme.eq_ignore_ascii_case("socks5h") {
        return Ok(ProxyKind::Socks5);
    }

    Err("Only http://, socks5://, and socks5h:// proxies are currently supported.".to_owned())
}
