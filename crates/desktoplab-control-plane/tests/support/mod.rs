use std::io::Read;
use std::net::{TcpListener, TcpStream};

pub fn accept_http_request(listener: &TcpListener) -> (TcpStream, String) {
    loop {
        let (mut stream, _) = listener.accept().expect("request should arrive");
        let request = read_request(&mut stream);
        if !request.is_empty() {
            return (stream, String::from_utf8_lossy(&request).to_string());
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("request should be readable");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        let Some(header_end) = find_header_end(&bytes) else {
            continue;
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::trim)
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .unwrap_or(0);
        if bytes.len() >= header_end + 4 + content_length {
            break;
        }
    }
    bytes
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}
