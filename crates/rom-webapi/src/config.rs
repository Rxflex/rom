use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NavigatorConfig {
    pub user_agent: String,
    pub app_name: String,
    pub platform: String,
    pub language: String,
    pub languages: Vec<String>,
    pub hardware_concurrency: u16,
    pub device_memory: f64,
    pub webdriver: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocationConfig {
    pub href: String,
    pub origin: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FetchConfig {
    pub cors_enabled: bool,
    pub proxy_url: Option<String>,
    pub cookie_store: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebRuntimeConfig {
    pub navigator: NavigatorConfig,
    pub location: LocationConfig,
    pub fetch: FetchConfig,
}
