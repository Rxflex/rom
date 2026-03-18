use serde::{Deserialize, Serialize};

use crate::{Result, RomRuntime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceSnapshot {
    pub globals: GlobalSurface,
    pub navigator: NavigatorSurface,
    pub canvas: CanvasSurface,
    pub observers: ObserverSurface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSurface {
    pub window: bool,
    pub document: bool,
    pub navigator: bool,
    pub location: bool,
    pub history: bool,
    pub performance: bool,
    pub crypto: bool,
    pub text_encoder: bool,
    pub text_decoder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigatorSurface {
    pub user_agent: String,
    pub language: String,
    pub languages: Vec<String>,
    pub platform: String,
    pub webdriver: bool,
    pub plugins_length: usize,
    pub mime_types_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSurface {
    pub has_canvas: bool,
    pub has_2d_context: bool,
    pub data_url_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverSurface {
    pub mutation_observer: bool,
    pub resize_observer: bool,
    pub intersection_observer: bool,
    pub performance_observer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintProbe {
    pub user_agent: String,
    pub language: String,
    pub languages: Vec<String>,
    pub platform: String,
    pub hardware_concurrency: Option<u16>,
    pub device_memory: Option<f64>,
    pub webdriver: bool,
    pub max_touch_points: u16,
    pub cookie_enabled: bool,
    pub timezone: Option<String>,
    pub screen: FingerprintScreen,
    pub storage: FingerprintStorage,
    pub media: FingerprintMedia,
    pub canvas: FingerprintCanvas,
    pub observers: FingerprintObservers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintScreen {
    pub width: u32,
    pub height: u32,
    pub color_depth: u16,
    pub pixel_depth: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintStorage {
    pub local_storage: bool,
    pub session_storage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintMedia {
    pub match_media: bool,
    pub audio_context: bool,
    pub audio_sample_rate: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintCanvas {
    pub has_canvas: bool,
    pub has_2d_context: bool,
    pub data_url_prefix: String,
    pub text_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintObservers {
    pub mutation: bool,
    pub resize: bool,
    pub intersection: bool,
    pub performance: bool,
}

impl RomRuntime {
    pub fn surface_snapshot(&self) -> Result<SurfaceSnapshot> {
        let json = self.eval_as_string(
            r#"
            (() => {
                const canvas = document.createElement("canvas");
                const context2d = canvas.getContext("2d");

                return {
                    globals: {
                        window: typeof window === "object",
                        document: typeof document === "object",
                        navigator: typeof navigator === "object",
                        location: typeof location === "object",
                        history: typeof history === "object",
                        performance: typeof performance === "object",
                        crypto: typeof crypto === "object",
                        text_encoder: typeof TextEncoder === "function",
                        text_decoder: typeof TextDecoder === "function",
                    },
                    navigator: {
                        user_agent: navigator.userAgent,
                        language: navigator.language,
                        languages: navigator.languages,
                        platform: navigator.platform,
                        webdriver: navigator.webdriver,
                        plugins_length: navigator.plugins.length,
                        mime_types_length: navigator.mimeTypes.length,
                    },
                    canvas: {
                        has_canvas: canvas instanceof HTMLCanvasElement,
                        has_2d_context: !!context2d,
                        data_url_prefix: canvas.toDataURL().slice(0, 21),
                    },
                    observers: {
                        mutation_observer: typeof MutationObserver === "function",
                        resize_observer: typeof ResizeObserver === "function",
                        intersection_observer: typeof IntersectionObserver === "function",
                        performance_observer: typeof PerformanceObserver === "function",
                    },
                };
            })()
            "#,
        )?;

        Ok(serde_json::from_str(&json)?)
    }

    pub fn fingerprint_probe(&self) -> Result<FingerprintProbe> {
        let json = self.eval_as_string(
            r#"
            (() => {
                const canvas = document.createElement("canvas");
                const context2d = canvas.getContext("2d");
                const audioContext =
                    typeof AudioContext === "function" ? new AudioContext() : null;
                const timezone =
                    typeof Intl === "object" && typeof Intl.DateTimeFormat === "function"
                        ? Intl.DateTimeFormat().resolvedOptions().timeZone ?? null
                        : null;

                return {
                    user_agent: navigator.userAgent,
                    language: navigator.language,
                    languages: navigator.languages,
                    platform: navigator.platform,
                    hardware_concurrency: navigator.hardwareConcurrency ?? null,
                    device_memory: navigator.deviceMemory ?? null,
                    webdriver: navigator.webdriver,
                    max_touch_points: navigator.maxTouchPoints ?? 0,
                    cookie_enabled: navigator.cookieEnabled,
                    timezone,
                    screen: {
                        width: screen.width,
                        height: screen.height,
                        color_depth: screen.colorDepth,
                        pixel_depth: screen.pixelDepth,
                    },
                    storage: {
                        local_storage: typeof localStorage === "object",
                        session_storage: typeof sessionStorage === "object",
                    },
                    media: {
                        match_media: typeof matchMedia === "function",
                        audio_context: typeof AudioContext === "function",
                        audio_sample_rate: audioContext?.sampleRate ?? null,
                    },
                    canvas: {
                        has_canvas: canvas instanceof HTMLCanvasElement,
                        has_2d_context: !!context2d,
                        data_url_prefix: canvas.toDataURL().slice(0, 21),
                        text_width: context2d ? context2d.measureText("rom").width : null,
                    },
                    observers: {
                        mutation: typeof MutationObserver === "function",
                        resize: typeof ResizeObserver === "function",
                        intersection: typeof IntersectionObserver === "function",
                        performance: typeof PerformanceObserver === "function",
                    },
                };
            })()
            "#,
        )?;

        Ok(serde_json::from_str(&json)?)
    }
}
