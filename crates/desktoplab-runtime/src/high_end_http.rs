use crate::high_end_health::{RuntimeEndpointError, RuntimeEndpointSpec};
use serde_json::Value;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, TcpStream};
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn http_get_json(
    endpoint: &RuntimeEndpointSpec,
    path: &str,
    timeout: Duration,
) -> Result<Value, RuntimeEndpointError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.socket_addr()?, timeout)
        .map_err(|_| RuntimeEndpointError::Unreachable)?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        endpoint.host(),
        endpoint.port()
    )
    .map_err(|_| RuntimeEndpointError::Unreachable)?;
    stream
        .flush()
        .map_err(|_| RuntimeEndpointError::Unreachable)?;

    let response = read_http_response(&mut stream)?;
    let header_end = find_header_end(&response).ok_or(RuntimeEndpointError::InvalidResponse)?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| RuntimeEndpointError::InvalidResponse)?;
    if !headers
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200 "))
    {
        return Err(RuntimeEndpointError::InvalidResponse);
    }
    let body = &response[header_end + 4..];
    serde_json::from_slice(body).map_err(|_| RuntimeEndpointError::InvalidResponse)
}

fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>, RuntimeEndpointError> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.len() > MAX_RESPONSE_BYTES {
                    return Err(RuntimeEndpointError::InvalidResponse);
                }
                if response_is_complete(&response)? {
                    break;
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if response_is_complete(&response)? {
                    break;
                }
                return Err(RuntimeEndpointError::InvalidResponse);
            }
            Err(_) => return Err(RuntimeEndpointError::InvalidResponse),
        }
    }
    response_is_complete(&response)?
        .then_some(response)
        .ok_or(RuntimeEndpointError::InvalidResponse)
}

fn response_is_complete(response: &[u8]) -> Result<bool, RuntimeEndpointError> {
    let Some(header_end) = find_header_end(response) else {
        return Ok(false);
    };
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| RuntimeEndpointError::InvalidResponse)?;
    let content_length = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    });
    Ok(content_length.is_some_and(|length| response.len() >= header_end + 4 + length))
}

fn find_header_end(response: &[u8]) -> Option<usize> {
    response.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(crate) fn is_local_host(host: &str) -> bool {
    host == "localhost" || host.parse::<IpAddr>().is_ok_and(is_local_ip)
}

pub(crate) fn is_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
    }
}
