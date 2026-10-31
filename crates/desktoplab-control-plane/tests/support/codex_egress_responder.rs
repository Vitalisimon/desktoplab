use std::io::{ErrorKind, Read};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

pub fn assert_no_nonempty_request(listener: &TcpListener, finished: &Receiver<()>) {
    let mut deadline = None;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut request = [0_u8; 256];
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .expect("read timeout should be set");
                let read = stream
                    .read(&mut request)
                    .expect("request should be readable");
                assert_eq!(read, 0, "responder must not receive repository context");
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => panic!("responder accept failed: {error}"),
        }
        if deadline.is_none() && !matches!(finished.try_recv(), Err(TryRecvError::Empty)) {
            deadline = Some(Instant::now() + Duration::from_secs(1));
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::mpsc;

    #[test]
    fn rejects_a_nonempty_request_even_before_the_stop_signal() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (_stop, finished) = mpsc::channel();
        let worker = std::thread::spawn(move || assert_no_nonempty_request(&listener, &finished));
        let mut connection = TcpStream::connect(address).unwrap();
        connection.write_all(b"private test context").unwrap();
        assert!(
            worker.join().is_err(),
            "unexpected egress must fail the test"
        );
    }
}
