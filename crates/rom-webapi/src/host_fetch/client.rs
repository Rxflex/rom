use httparse::Response as HttpParseResponse;
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};
use ureq::http::{Method, StatusCode};
use url::Url;

use super::{
    FetchRequestPayload, FetchResponsePayload, HeaderEntry,
    proxy::{ProxyConfig, ProxyKind, resolve_proxy},
    socks::connect_via_socks5,
    tls::build_tls_config,
};

const READ_TIMEOUT: Duration = Duration::from_secs(20);
const WRITE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_REDIRECTS: usize = 10;
pub fn execute_request(request: &FetchRequestPayload) -> Result<FetchResponsePayload, String> {
    let mut current_url = Url::parse(&request.url).map_err(|error| error.to_string())?;
    let mut method = request
        .method
        .parse::<Method>()
        .map_err(|error| error.to_string())?;
    let mut body = request.body.clone();
    let mut redirect_count = 0;

    loop {
        let response = execute_single_request(
            &current_url,
            &method,
            &request.headers,
            &body,
            request.proxy_url.as_deref(),
        )?;

        if request.redirect_mode != "follow" || !response.is_redirect_response {
            return Ok(FetchResponsePayload {
                redirected: current_url.as_str() != request.url,
                ..response
            });
        }

        if redirect_count >= MAX_REDIRECTS {
            return Err("Too many redirects.".to_owned());
        }

        let next_location = response
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("location"))
            .map(|header| header.value.clone())
            .ok_or_else(|| "Redirect response is missing Location header.".to_owned())?;
        current_url = current_url
            .join(&next_location)
            .map_err(|error| error.to_string())?;
        redirect_count += 1;

        if should_switch_to_get(response.status, &method) {
            method = Method::GET;
            body.clear();
        }
    }
}

fn execute_single_request(
    url: &Url,
    method: &Method,
    headers: &[HeaderEntry],
    body: &[u8],
    explicit_proxy_url: Option<&str>,
) -> Result<FetchResponsePayload, String> {
    let proxy = resolve_proxy(url, explicit_proxy_url)?;
    let mut stream = connect_stream(url, proxy.as_ref())?;
    let request_bytes = build_http1_request(url, method, headers, body, proxy.as_ref())?;
    stream
        .write_all(&request_bytes)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let response = read_http1_response(&mut stream, method == Method::HEAD)?;

    Ok(FetchResponsePayload {
        url: url.as_str().to_owned(),
        status: response.status.as_u16(),
        status_text: response.status.canonical_reason().unwrap_or("").to_owned(),
        redirected: false,
        is_redirect_response: response.status.is_redirection(),
        headers: response.headers,
        body: response.body,
    })
}

fn connect_stream(url: &Url, proxy: Option<&ProxyConfig>) -> Result<ClientStream, String> {
    let target_host = url
        .host_str()
        .ok_or_else(|| "Request URL is missing host.".to_owned())?;
    let target_port = url
        .port_or_known_default()
        .ok_or_else(|| "Request URL is missing port.".to_owned())?;

    let mut stream = match proxy {
        Some(proxy) => connect_proxy_stream(proxy, target_host, target_port)?,
        None => Box::new(connect_tcp(target_host, target_port)?),
    };

    if url.scheme() == "https" {
        if let Some(proxy) = proxy
            && proxy.uses_http_connect()
        {
            send_connect_request(
                &mut *stream,
                target_host,
                target_port,
                proxy.authorization.as_deref(),
            )?;
        }

        let tls = build_tls_config(target_host)?
            .connect(target_host, stream)
            .map_err(|error| error.to_string())?;
        return Ok(Box::new(tls));
    }

    Ok(stream)
}

fn connect_proxy_stream(
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<ClientStream, String> {
    let mut tcp = connect_tcp(&proxy.host, proxy.port)?;

    if matches!(proxy.kind, ProxyKind::Socks5) {
        connect_via_socks5(&mut tcp, proxy, target_host, target_port)?;
        return Ok(Box::new(tcp));
    }

    if proxy.uses_tls() {
        let tls = build_tls_config(&proxy.host)?
            .connect(&proxy.host, tcp)
            .map_err(|error| error.to_string())?;
        return Ok(Box::new(tls));
    }

    Ok(Box::new(tcp))
}

fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, String> {
    let address = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .next()
        .ok_or_else(|| format!("Unable to resolve {host}:{port}"))?;
    let stream = TcpStream::connect(address).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| error.to_string())?;
    Ok(stream)
}

fn send_connect_request(
    stream: &mut dyn ReadWriteStream,
    target_host: &str,
    target_port: u16,
    proxy_authorization: Option<&str>,
) -> Result<(), String> {
    let mut request = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\nConnection: keep-alive\r\n",
    );
    if let Some(value) = proxy_authorization {
        request.push_str(&format!("Proxy-Authorization: {value}\r\n"));
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let response = read_http1_response(stream, true)?;
    if response.status.as_u16() / 100 != 2 {
        return Err(format!(
            "Proxy CONNECT failed with status {}.",
            response.status
        ));
    }

    Ok(())
}

fn build_http1_request(
    url: &Url,
    method: &Method,
    headers: &[HeaderEntry],
    body: &[u8],
    proxy: Option<&ProxyConfig>,
) -> Result<Vec<u8>, String> {
    let request_target = if proxy
        .map(ProxyConfig::uses_absolute_form_for_http)
        .unwrap_or(false)
        && url.scheme() == "http"
    {
        url.as_str().to_owned()
    } else {
        let path = url.path();
        let query = url
            .query()
            .map(|value| format!("?{value}"))
            .unwrap_or_default();
        if path.is_empty() {
            format!("/{query}")
        } else {
            format!("{path}{query}")
        }
    };

    let mut lines = vec![format!("{method} {request_target} HTTP/1.1")];
    let mut has_host = false;
    let mut has_connection = false;
    let mut has_content_length = false;
    let mut has_accept_encoding = false;
    let mut has_proxy_authorization = false;

    for header in headers {
        let normalized = header.name.to_ascii_lowercase();
        has_host |= normalized == "host";
        has_connection |= normalized == "connection";
        has_content_length |= normalized == "content-length";
        has_accept_encoding |= normalized == "accept-encoding";
        has_proxy_authorization |= normalized == "proxy-authorization";
        lines.push(format!("{}: {}", header.name, header.value));
    }

    if !has_host {
        lines.push(format!("Host: {}", host_header_value(url)));
    }
    if !has_connection {
        lines.push("Connection: close".to_owned());
    }
    if !has_accept_encoding {
        lines.push("Accept-Encoding: identity".to_owned());
    }
    if !has_proxy_authorization
        && url.scheme() == "http"
        && let Some(proxy_authorization) = proxy.and_then(|value| value.authorization.as_deref())
        && proxy
            .map(ProxyConfig::uses_absolute_form_for_http)
            .unwrap_or(false)
    {
        lines.push(format!("Proxy-Authorization: {proxy_authorization}"));
    }
    if !body.is_empty() && !has_content_length {
        lines.push(format!("Content-Length: {}", body.len()));
    }

    let mut request_bytes = lines.join("\r\n").into_bytes();
    request_bytes.extend_from_slice(b"\r\n\r\n");
    request_bytes.extend_from_slice(body);
    Ok(request_bytes)
}

fn host_header_value(url: &Url) -> String {
    match url.port() {
        Some(port) if Some(port) != url.port_or_known_default() => {
            format!("{}:{port}", url.host_str().unwrap_or_default())
        }
        _ => url.host_str().unwrap_or_default().to_owned(),
    }
}

struct HttpResponseData {
    status: StatusCode,
    headers: Vec<HeaderEntry>,
    body: Vec<u8>,
}

fn read_http1_response(
    stream: &mut (impl Read + ?Sized),
    body_forbidden: bool,
) -> Result<HttpResponseData, String> {
    let (status, headers, mut body_buffer) = read_response_head(stream)?;
    if body_forbidden
        || status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
    {
        return Ok(HttpResponseData {
            status,
            headers,
            body: Vec::new(),
        });
    }

    if is_chunked_response(&headers) {
        return Ok(HttpResponseData {
            status,
            headers,
            body: read_chunked_body(stream, &mut body_buffer)?,
        });
    }

    if let Some(length) = content_length(&headers) {
        while body_buffer.len() < length {
            let mut chunk = [0_u8; 8192];
            let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            body_buffer.extend_from_slice(&chunk[..read]);
        }

        body_buffer.truncate(length);
        return Ok(HttpResponseData {
            status,
            headers,
            body: body_buffer,
        });
    }

    let mut tail = Vec::new();
    stream
        .read_to_end(&mut tail)
        .map_err(|error| error.to_string())?;
    body_buffer.extend_from_slice(&tail);

    Ok(HttpResponseData {
        status,
        headers,
        body: body_buffer,
    })
}

fn read_response_head(
    stream: &mut (impl Read + ?Sized),
) -> Result<(StatusCode, Vec<HeaderEntry>, Vec<u8>), String> {
    let mut buffer = Vec::new();
    let mut header_end = None;

    while header_end.is_none() {
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        header_end = buffer
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4);
    }

    let header_end = header_end.ok_or_else(|| "Incomplete HTTP response headers.".to_owned())?;
    let mut parsed_headers = [httparse::EMPTY_HEADER; 64];
    let mut response = HttpParseResponse::new(&mut parsed_headers);
    response
        .parse(&buffer[..header_end])
        .map_err(|error| error.to_string())?;
    let status = StatusCode::from_u16(
        response
            .code
            .ok_or_else(|| "Missing HTTP status.".to_owned())?,
    )
    .map_err(|error| error.to_string())?;
    let headers = response
        .headers
        .iter()
        .map(|header| HeaderEntry {
            name: header.name.to_owned(),
            value: String::from_utf8_lossy(header.value).into_owned(),
        })
        .collect::<Vec<_>>();

    Ok((status, headers, buffer[header_end..].to_vec()))
}

fn content_length(headers: &[HeaderEntry]) -> Option<usize> {
    headers.iter().find_map(|header| {
        header
            .name
            .eq_ignore_ascii_case("content-length")
            .then(|| header.value.trim().parse::<usize>().ok())
            .flatten()
    })
}

fn is_chunked_response(headers: &[HeaderEntry]) -> bool {
    headers.iter().any(|header| {
        header.name.eq_ignore_ascii_case("transfer-encoding")
            && header
                .value
                .split(',')
                .any(|entry| entry.trim().eq_ignore_ascii_case("chunked"))
    })
}

fn read_chunked_body(
    stream: &mut (impl Read + ?Sized),
    buffer: &mut Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();

    loop {
        let line = read_line(stream, buffer)?;
        let chunk_size_hex = line.split(';').next().unwrap_or("").trim();
        let chunk_size =
            usize::from_str_radix(chunk_size_hex, 16).map_err(|error| error.to_string())?;

        if chunk_size == 0 {
            let _ = read_line(stream, buffer)?;
            break;
        }

        ensure_buffered(stream, buffer, chunk_size + 2)?;
        output.extend_from_slice(&buffer[..chunk_size]);
        buffer.drain(..chunk_size + 2);
    }

    Ok(output)
}

fn read_line(stream: &mut (impl Read + ?Sized), buffer: &mut Vec<u8>) -> Result<String, String> {
    loop {
        if let Some(index) = buffer.windows(2).position(|window| window == b"\r\n") {
            let line = String::from_utf8_lossy(&buffer[..index]).into_owned();
            buffer.drain(..index + 2);
            return Ok(line);
        }
        ensure_buffered(stream, buffer, buffer.len() + 1)?;
    }
}

fn ensure_buffered(
    stream: &mut (impl Read + ?Sized),
    buffer: &mut Vec<u8>,
    required_len: usize,
) -> Result<(), String> {
    while buffer.len() < required_len {
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("Unexpected EOF while reading HTTP response.".to_owned());
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    Ok(())
}

fn should_switch_to_get(status: u16, method: &Method) -> bool {
    status == 303 || ((status == 301 || status == 302) && *method == Method::POST)
}

trait ReadWriteStream: Read + Write {}

impl<T> ReadWriteStream for T where T: Read + Write {}

type ClientStream = Box<dyn ReadWriteStream>;
