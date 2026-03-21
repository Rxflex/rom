use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpStream},
};

use super::proxy::ProxyConfig;

const SOCKS_VERSION: u8 = 0x05;
const NO_AUTHENTICATION: u8 = 0x00;
const USERNAME_PASSWORD: u8 = 0x02;
const CONNECT_COMMAND: u8 = 0x01;
const ADDRESS_IPV4: u8 = 0x01;
const ADDRESS_DOMAIN: u8 = 0x03;
const ADDRESS_IPV6: u8 = 0x04;

pub fn connect_via_socks5(
    stream: &mut TcpStream,
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<(), String> {
    negotiate_authentication(stream, proxy)?;
    send_connect_command(stream, target_host, target_port)?;
    Ok(())
}

fn negotiate_authentication(stream: &mut TcpStream, proxy: &ProxyConfig) -> Result<(), String> {
    let mut methods = vec![NO_AUTHENTICATION];
    if proxy.credentials.is_some() {
        methods.push(USERNAME_PASSWORD);
    }

    let mut greeting = Vec::with_capacity(2 + methods.len());
    greeting.push(SOCKS_VERSION);
    greeting.push(methods.len() as u8);
    greeting.extend_from_slice(&methods);
    stream
        .write_all(&greeting)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut response = [0_u8; 2];
    stream
        .read_exact(&mut response)
        .map_err(|error| error.to_string())?;

    if response[0] != SOCKS_VERSION {
        return Err("SOCKS5 proxy returned an unexpected version.".to_owned());
    }

    match response[1] {
        NO_AUTHENTICATION => Ok(()),
        USERNAME_PASSWORD => authenticate_with_password(stream, proxy),
        0xFF => Err("SOCKS5 proxy rejected all authentication methods.".to_owned()),
        method => Err(format!(
            "SOCKS5 proxy selected unsupported authentication method {method:#04x}."
        )),
    }
}

fn authenticate_with_password(stream: &mut TcpStream, proxy: &ProxyConfig) -> Result<(), String> {
    let credentials = proxy
        .credentials
        .as_ref()
        .ok_or_else(|| "SOCKS5 proxy requires credentials.".to_owned())?;
    let username = credentials.username.as_bytes();
    let password = credentials.password.as_bytes();

    if username.is_empty() || username.len() > u8::MAX as usize {
        return Err("SOCKS5 username must be between 1 and 255 bytes.".to_owned());
    }

    if password.len() > u8::MAX as usize {
        return Err("SOCKS5 password must be 255 bytes or less.".to_owned());
    }

    let mut request = Vec::with_capacity(3 + username.len() + password.len());
    request.push(0x01);
    request.push(username.len() as u8);
    request.extend_from_slice(username);
    request.push(password.len() as u8);
    request.extend_from_slice(password);

    stream
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut response = [0_u8; 2];
    stream
        .read_exact(&mut response)
        .map_err(|error| error.to_string())?;

    if response[1] != 0x00 {
        return Err("SOCKS5 proxy authentication failed.".to_owned());
    }

    Ok(())
}

fn send_connect_command(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<(), String> {
    let mut request = vec![SOCKS_VERSION, CONNECT_COMMAND, 0x00];
    append_target_address(&mut request, target_host)?;
    request.extend_from_slice(&target_port.to_be_bytes());

    stream
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut response_head = [0_u8; 4];
    stream
        .read_exact(&mut response_head)
        .map_err(|error| error.to_string())?;

    if response_head[0] != SOCKS_VERSION {
        return Err("SOCKS5 proxy returned an unexpected version.".to_owned());
    }

    if response_head[1] != 0x00 {
        return Err(socks5_reply_error(response_head[1]));
    }

    discard_bound_address(stream, response_head[3])?;
    Ok(())
}

fn append_target_address(buffer: &mut Vec<u8>, target_host: &str) -> Result<(), String> {
    match target_host.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => append_ipv4_address(buffer, address),
        Ok(IpAddr::V6(address)) => append_ipv6_address(buffer, address),
        Err(_) => append_domain_address(buffer, target_host),
    }
}

fn append_ipv4_address(buffer: &mut Vec<u8>, address: Ipv4Addr) -> Result<(), String> {
    buffer.push(ADDRESS_IPV4);
    buffer.extend_from_slice(&address.octets());
    Ok(())
}

fn append_ipv6_address(buffer: &mut Vec<u8>, address: Ipv6Addr) -> Result<(), String> {
    buffer.push(ADDRESS_IPV6);
    buffer.extend_from_slice(&address.octets());
    Ok(())
}

fn append_domain_address(buffer: &mut Vec<u8>, target_host: &str) -> Result<(), String> {
    if target_host.is_empty() || target_host.len() > u8::MAX as usize {
        return Err("SOCKS5 target host must be between 1 and 255 bytes.".to_owned());
    }

    buffer.push(ADDRESS_DOMAIN);
    buffer.push(target_host.len() as u8);
    buffer.extend_from_slice(target_host.as_bytes());
    Ok(())
}

fn discard_bound_address(stream: &mut TcpStream, address_type: u8) -> Result<(), String> {
    let address_len = match address_type {
        ADDRESS_IPV4 => 4,
        ADDRESS_IPV6 => 16,
        ADDRESS_DOMAIN => {
            let mut len = [0_u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|error| error.to_string())?;
            len[0] as usize
        }
        _ => return Err("SOCKS5 proxy returned an unsupported address type.".to_owned()),
    };

    let mut discard = vec![0_u8; address_len + 2];
    stream
        .read_exact(&mut discard)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn socks5_reply_error(code: u8) -> String {
    match code {
        0x01 => "SOCKS5 proxy reported a general failure.".to_owned(),
        0x02 => "SOCKS5 proxy rejected the connection by policy.".to_owned(),
        0x03 => "SOCKS5 proxy reported network unreachable.".to_owned(),
        0x04 => "SOCKS5 proxy reported host unreachable.".to_owned(),
        0x05 => "SOCKS5 proxy refused the target connection.".to_owned(),
        0x06 => "SOCKS5 proxy reported TTL expired.".to_owned(),
        0x07 => "SOCKS5 proxy does not support the requested command.".to_owned(),
        0x08 => "SOCKS5 proxy does not support the target address type.".to_owned(),
        _ => format!("SOCKS5 proxy rejected the connection with reply {code:#04x}."),
    }
}
