use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::time::Duration;

const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HttpRequestReadError {
    Malformed,
    PayloadTooLarge,
}

pub(super) fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, HttpRequestReadError> {
    stream
        .set_nonblocking(false)
        .map_err(|_| HttpRequestReadError::Malformed)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| HttpRequestReadError::Malformed)?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    let mut expected_body_len = None;

    loop {
        let bytes_read = read_retrying_interrupts(stream, &mut buffer)
            .map_err(|_| HttpRequestReadError::Malformed)?;
        if bytes_read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..bytes_read]);
        let Some(body_start) = header_end(&request) else {
            continue;
        };
        if expected_body_len.is_none() {
            expected_body_len = content_length(&request[..body_start])?;
            if expected_body_len.unwrap_or(0) > MAX_REQUEST_BODY_BYTES {
                return Err(HttpRequestReadError::PayloadTooLarge);
            }
        }
        let body_len = expected_body_len.unwrap_or(0);
        if request.len().saturating_sub(body_start) >= body_len {
            request.truncate(body_start + body_len);
            return Ok(request);
        }
    }

    if header_end(&request).is_some() {
        Ok(request)
    } else {
        Err(HttpRequestReadError::Malformed)
    }
}

fn read_retrying_interrupts(reader: &mut impl Read, buffer: &mut [u8]) -> std::io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn header_end(request: &[u8]) -> Option<usize> {
    request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn content_length(headers: &[u8]) -> Result<Option<usize>, HttpRequestReadError> {
    let headers = String::from_utf8_lossy(headers);
    let Some(value) = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("content-length")
            .then(|| value.trim())
    }) else {
        return Ok(None);
    };
    value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| HttpRequestReadError::Malformed)
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use super::{read_http_request, read_retrying_interrupts};

    struct InterruptedOnceReader {
        interrupted: bool,
        bytes: &'static [u8],
    }

    impl Read for InterruptedOnceReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(Error::from(ErrorKind::Interrupted));
            }
            let length = self.bytes.len().min(buffer.len());
            buffer[..length].copy_from_slice(&self.bytes[..length]);
            Ok(length)
        }
    }

    #[test]
    fn interrupted_reads_are_retried_without_reclassifying_the_request() {
        let mut reader = InterruptedOnceReader {
            interrupted: false,
            bytes: b"request",
        };
        let mut buffer = [0_u8; 16];

        let bytes_read = read_retrying_interrupts(&mut reader, &mut buffer).unwrap();

        assert_eq!(&buffer[..bytes_read], b"request");
    }

    #[test]
    fn accepted_nonblocking_stream_waits_for_a_delayed_valid_request() {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let address = listener.local_addr().unwrap();
        let writer = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            thread::sleep(Duration::from_millis(25));
            stream
                .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();

        let request = read_http_request(&mut stream).expect("valid delayed request should parse");

        writer.join().unwrap();
        assert_eq!(request, b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n");
    }
}
