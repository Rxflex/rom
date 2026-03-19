use serde::{Deserialize, Serialize};
use rom_webapi::{LocationConfig, NavigatorConfig, WebRuntimeConfig};
use url::Url;

use crate::error::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub href: String,
    pub user_agent: String,
    pub app_name: String,
    pub platform: String,
    pub language: String,
    pub languages: Vec<String>,
    pub hardware_concurrency: u16,
    pub device_memory: f64,
    pub webdriver: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            href: "https://rom.local/".to_owned(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) ROM/0.1".to_owned(),
            app_name: "Netscape".to_owned(),
            platform: "Win32".to_owned(),
            language: "en-US".to_owned(),
            languages: vec!["en-US".to_owned(), "en".to_owned()],
            hardware_concurrency: 8,
            device_memory: 8.0,
            webdriver: false,
        }
    }
}

impl RuntimeConfig {
    pub fn to_web_config(&self) -> Result<WebRuntimeConfig> {
        let parsed = Url::parse(&self.href)?;
        let host = match parsed.port() {
            Some(port) => format!("{}:{port}", parsed.host_str().unwrap_or_default()),
            None => parsed.host_str().unwrap_or_default().to_owned(),
        };
        let protocol = format!("{}:", parsed.scheme());
        let pathname = if parsed.path().is_empty() {
            "/".to_owned()
        } else {
            parsed.path().to_owned()
        };
        let search = parsed
            .query()
            .map(|query| format!("?{query}"))
            .unwrap_or_default();
        let hash = parsed
            .fragment()
            .map(|fragment| format!("#{fragment}"))
            .unwrap_or_default();

        Ok(WebRuntimeConfig {
            navigator: NavigatorConfig {
                user_agent: self.user_agent.clone(),
                app_name: self.app_name.clone(),
                platform: self.platform.clone(),
                language: self.language.clone(),
                languages: self.languages.clone(),
                hardware_concurrency: self.hardware_concurrency,
                device_memory: self.device_memory,
                webdriver: self.webdriver,
            },
            location: LocationConfig {
                href: self.href.clone(),
                origin: format!("{protocol}//{host}"),
                protocol,
                host,
                hostname: parsed.host_str().unwrap_or_default().to_owned(),
                pathname,
                search,
                hash,
            },
        })
    }
}
