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
