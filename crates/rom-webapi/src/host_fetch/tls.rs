use boring::{
    ssl::{ConnectConfiguration, SslConnector, SslMethod, SslVerifyMode},
    x509::X509,
};
use rustls_native_certs::load_native_certs;
use std::{net::IpAddr, sync::OnceLock};

const CHROME_LIKE_CIPHER_LIST: &str = "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:ECDHE-RSA-AES128-SHA:ECDHE-RSA-AES256-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA:AES256-SHA";

struct NativeRoots {
    certs: Vec<Vec<u8>>,
    error: Option<String>,
}

pub fn build_tls_config(host: &str) -> Result<ConnectConfiguration, String> {
    let mut builder = SslConnector::builder(SslMethod::tls()).map_err(|error| error.to_string())?;

    if is_local_tls_host(host) {
        builder.set_verify(SslVerifyMode::NONE);
    } else {
        builder.set_verify(SslVerifyMode::PEER);
        load_trust_roots(&mut builder)?;
    }

    builder.set_grease_enabled(true);
    builder.enable_ocsp_stapling();
    builder.enable_signed_cert_timestamps();
    builder
        .set_cipher_list(CHROME_LIKE_CIPHER_LIST)
        .map_err(|error| error.to_string())?;
    builder
        .set_alpn_protos(b"\x08http/1.1")
        .map_err(|error| error.to_string())?;

    let connector = builder.build();
    let mut config = connector.configure().map_err(|error| error.to_string())?;
    if is_local_tls_host(host) {
        config.set_verify_hostname(false);
    }

    Ok(config)
}

fn load_trust_roots(builder: &mut boring::ssl::SslConnectorBuilder) -> Result<(), String> {
    let default_verify_paths_loaded = builder.set_default_verify_paths().is_ok();
    let native_roots = native_roots();

    if let Some(error) = &native_roots.error
        && !default_verify_paths_loaded
    {
        return Err(error.clone());
    }

    for certificate_der in &native_roots.certs {
        let certificate = X509::from_der(certificate_der).map_err(|error| error.to_string())?;
        let _ = builder.cert_store_mut().add_cert(certificate);
    }

    if default_verify_paths_loaded || !native_roots.certs.is_empty() {
        return Ok(());
    }

    Err("Failed to load any TLS trust roots.".to_owned())
}

fn native_roots() -> &'static NativeRoots {
    static CACHE: OnceLock<NativeRoots> = OnceLock::new();

    CACHE.get_or_init(|| {
        let result = load_native_certs();
        let certs: Vec<Vec<u8>> = result
            .certs
            .iter()
            .map(|certificate| certificate.as_ref().to_vec())
            .collect();

        let error = if certs.is_empty() && !result.errors.is_empty() {
            Some(format!(
                "Failed to load native TLS root certificates: {}",
                result
                    .errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        } else {
            None
        };

        NativeRoots { certs, error }
    })
}

fn is_local_tls_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|address| address.is_loopback())
            .unwrap_or(false)
}
