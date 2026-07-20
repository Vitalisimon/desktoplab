use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};

pub struct ProviderMockConfig {
    pub expected_path: &'static str,
    pub required_authorization: Option<&'static str>,
    pub status: u16,
    pub body: &'static str,
}

pub struct ProviderMock {
    endpoint: String,
    handle: JoinHandle<ProviderMockEvidence>,
}

#[derive(Debug)]
pub struct ProviderMockEvidence {
    pub request: Value,
    pub jsonl: String,
    pub driver: &'static str,
}

impl ProviderMock {
    pub fn start(config: ProviderMockConfig) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || serve(listener, config));
        Self { endpoint, handle }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn finish(self) -> ProviderMockEvidence {
        self.handle.join().unwrap()
    }
}

fn serve(listener: TcpListener, config: ProviderMockConfig) -> ProviderMockEvidence {
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let raw = read_request(&mut stream);
    let request = parse_request(&raw);
    let path_matches = request["path"] == config.expected_path;
    let authorization_matches = config
        .required_authorization
        .is_none_or(|required| request["authorizationRaw"] == required);
    let status = if !path_matches {
        404
    } else if !authorization_matches {
        401
    } else {
        config.status
    };
    let body = if status == config.status {
        config.body
    } else {
        r#"{"error":"request_rejected"}"#
    };
    write_response(&mut stream, status, body);
    let redacted = json!({
        "driver":"mock",
        "method":request["method"],
        "path":request["path"],
        "authorization":request["authorizationRaw"].as_str().map(|_| "[REDACTED]"),
        "body":redact_body(request["body"].as_str().unwrap_or_default()),
        "responseStatus":status
    });
    ProviderMockEvidence {
        request,
        jsonl: format!("{}\n", redacted),
        driver: "mock",
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap();
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

fn parse_request(raw: &[u8]) -> Value {
    let header_end = find_header_end(raw).unwrap();
    let headers = String::from_utf8_lossy(&raw[..header_end]);
    let mut lines = headers.lines();
    let mut request_line = lines.next().unwrap().split_whitespace();
    let method = request_line.next().unwrap_or_default();
    let path = request_line.next().unwrap_or_default();
    let authorization = lines.find_map(|line| {
        line.split_once(':')
            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| value.trim())
    });
    json!({
        "method":method,
        "path":path,
        "authorizationRaw":authorization,
        "body":String::from_utf8_lossy(&raw[header_end + 4..])
    })
}

fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        _ => "Not Found",
    };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn redact_body(body: &str) -> String {
    body.replace("sk-test-secret", "[REDACTED]")
}
