use rquickjs::{Ctx, Function, Object, Result};

use crate::config::WebRuntimeConfig;
use crate::host_crypto::CryptoHost;
use crate::host_fetch::perform_fetch;
use crate::host_url::parse_url;
use crate::host_websocket::WebSocketHost;

const WEB_BOOTSTRAP: &str = concat!(
    include_str!("js/bootstrap_prelude.js"),
    "\n",
    include_str!("js/bootstrap_dom.js"),
    "\n",
    include_str!("js/bootstrap_url.js"),
    "\n",
    include_str!("js/bootstrap_parser.js"),
    "\n",
    include_str!("js/bootstrap_urlpattern.js"),
    "\n",
    include_str!("js/bootstrap_navigator.js"),
    "\n",
    include_str!("js/bootstrap_cookie.js"),
    "\n",
    include_str!("js/bootstrap_body.js"),
    "\n",
    include_str!("js/bootstrap_crypto.js"),
    "\n",
    include_str!("js/bootstrap_fetch.js"),
    "\n",
    include_str!("js/bootstrap_messaging.js"),
    "\n",
    include_str!("js/bootstrap_eventsource.js"),
    "\n",
    include_str!("js/bootstrap_websocket.js"),
    "\n",
    include_str!("js/bootstrap_globals.js"),
);

pub fn install_browser_api(ctx: Ctx<'_>, config: &WebRuntimeConfig) -> Result<()> {
    let globals = ctx.globals();
    let crypto_host = CryptoHost::default();
    let websocket_host = WebSocketHost::default();

    globals.set("__rom_console_log", make_console_fn(ctx.clone(), "log"))?;
    globals.set("__rom_console_warn", make_console_fn(ctx.clone(), "warn"))?;
    globals.set("__rom_console_error", make_console_fn(ctx.clone(), "error"))?;
    globals.set("__rom_fetch_sync", make_fetch_fn(ctx.clone())?)?;
    globals.set("__rom_parse_url", make_parse_url_fn(ctx.clone())?)?;
    globals.set(
        "__rom_random_bytes",
        make_random_bytes_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_digest",
        make_subtle_digest_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_generate_key",
        make_subtle_generate_key_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_import_key",
        make_subtle_import_key_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_export_key",
        make_subtle_export_key_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_sign",
        make_subtle_sign_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_verify",
        make_subtle_verify_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_encrypt",
        make_subtle_encrypt_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_decrypt",
        make_subtle_decrypt_fn(ctx.clone(), crypto_host.clone())?,
    )?;
    globals.set(
        "__rom_subtle_derive_bits",
        make_subtle_derive_bits_fn(ctx.clone(), crypto_host)?,
    )?;
    globals.set(
        "__rom_websocket_connect",
        make_websocket_connect_fn(ctx.clone(), websocket_host.clone())?,
    )?;
    globals.set(
        "__rom_websocket_send",
        make_websocket_send_fn(ctx.clone(), websocket_host.clone())?,
    )?;
    globals.set(
        "__rom_websocket_poll",
        make_websocket_poll_fn(ctx.clone(), websocket_host.clone())?,
    )?;
    globals.set(
        "__rom_websocket_close",
        make_websocket_close_fn(ctx.clone(), websocket_host)?,
    )?;
    globals.set("__rom_config", make_config_object(ctx.clone(), config)?)?;

    ctx.eval::<(), _>(WEB_BOOTSTRAP)?;
    globals.remove("__rom_config")?;

    Ok(())
}

fn make_console_fn<'js>(ctx: Ctx<'js>, level: &'static str) -> Result<Function<'js>> {
    Function::new(ctx, move |message: String| {
        println!("[rom:{level}] {message}");
    })
}

fn make_fetch_fn<'js>(ctx: Ctx<'js>) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        perform_fetch(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("FetchRequest", "FetchResponse", error)
        })
    })
}

fn make_parse_url_fn<'js>(ctx: Ctx<'js>) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        parse_url(&payload)
            .map_err(|error| rquickjs::Error::new_from_js_message("URLInput", "URL", error))
    })
}

fn make_random_bytes_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |length: usize| -> rquickjs::Result<String> {
        crypto_host.random_bytes_json(length).map_err(|error| {
            rquickjs::Error::new_from_js_message("RandomLength", "RandomBytes", error)
        })
    })
}

fn make_subtle_digest_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host
            .subtle_digest(&payload)
            .map_err(|error| rquickjs::Error::new_from_js_message("DigestInput", "Digest", error))
    })
}

fn make_subtle_generate_key_fn<'js>(
    ctx: Ctx<'js>,
    crypto_host: CryptoHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_generate_key(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("GenerateKeyInput", "CryptoKey", error)
        })
    })
}

fn make_subtle_import_key_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_import_key(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("ImportKeyInput", "CryptoKey", error)
        })
    })
}

fn make_subtle_export_key_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_export_key(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("ExportKeyInput", "ExportedKey", error)
        })
    })
}

fn make_subtle_sign_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host
            .subtle_sign(&payload)
            .map_err(|error| rquickjs::Error::new_from_js_message("SignInput", "Signature", error))
    })
}

fn make_subtle_verify_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_verify(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("VerifyInput", "VerifyResult", error)
        })
    })
}

fn make_subtle_encrypt_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_encrypt(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("EncryptInput", "Ciphertext", error)
        })
    })
}

fn make_subtle_decrypt_fn<'js>(ctx: Ctx<'js>, crypto_host: CryptoHost) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_decrypt(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("DecryptInput", "Plaintext", error)
        })
    })
}

fn make_subtle_derive_bits_fn<'js>(
    ctx: Ctx<'js>,
    crypto_host: CryptoHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        crypto_host.subtle_derive_bits(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("DeriveBitsInput", "DerivedBits", error)
        })
    })
}

fn make_websocket_connect_fn<'js>(
    ctx: Ctx<'js>,
    websocket_host: WebSocketHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        websocket_host.connect(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("WebSocketConnectInput", "WebSocket", error)
        })
    })
}

fn make_websocket_send_fn<'js>(
    ctx: Ctx<'js>,
    websocket_host: WebSocketHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<()> {
        websocket_host.send(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("WebSocketSendInput", "WebSocket", error)
        })
    })
}

fn make_websocket_poll_fn<'js>(
    ctx: Ctx<'js>,
    websocket_host: WebSocketHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        websocket_host.poll(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("WebSocketPollInput", "WebSocket", error)
        })
    })
}

fn make_websocket_close_fn<'js>(
    ctx: Ctx<'js>,
    websocket_host: WebSocketHost,
) -> Result<Function<'js>> {
    Function::new(ctx, move |payload: String| -> rquickjs::Result<String> {
        websocket_host.close(&payload).map_err(|error| {
            rquickjs::Error::new_from_js_message("WebSocketCloseInput", "WebSocket", error)
        })
    })
}

fn make_config_object<'js>(ctx: Ctx<'js>, config: &WebRuntimeConfig) -> Result<Object<'js>> {
    let root = Object::new(ctx.clone())?;

    let navigator = Object::new(ctx.clone())?;
    navigator.set("userAgent", config.navigator.user_agent.as_str())?;
    navigator.set("appName", config.navigator.app_name.as_str())?;
    navigator.set("platform", config.navigator.platform.as_str())?;
    navigator.set("language", config.navigator.language.as_str())?;
    navigator.set("languages", config.navigator.languages.clone())?;
    navigator.set(
        "hardwareConcurrency",
        i32::from(config.navigator.hardware_concurrency),
    )?;
    navigator.set("deviceMemory", config.navigator.device_memory)?;
    navigator.set("webdriver", config.navigator.webdriver)?;

    let location = Object::new(ctx.clone())?;
    location.set("href", config.location.href.as_str())?;
    location.set("origin", config.location.origin.as_str())?;
    location.set("protocol", config.location.protocol.as_str())?;
    location.set("host", config.location.host.as_str())?;
    location.set("hostname", config.location.hostname.as_str())?;
    location.set("pathname", config.location.pathname.as_str())?;
    location.set("search", config.location.search.as_str())?;
    location.set("hash", config.location.hash.as_str())?;

    root.set("navigator", navigator)?;
    root.set("location", location)?;

    Ok(root)
}
