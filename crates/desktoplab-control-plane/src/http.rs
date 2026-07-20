use crate::{
    AuthDecision, ControlPlane, ControlPlaneStatus, CorsDecision, ErrorCode, LocalApiAuth,
    LocalApiRequestOrigin, LocalApiRouter, OriginPolicy, ReadinessState, ShutdownMode,
};
mod agent_worker;
mod auth_response;
mod request_body;

use auth_response::{forbidden, unauthorized};
use request_body::{HttpRequestReadError, read_http_request};
use std::fmt;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpServerConfig {
    bind_addr: SocketAddr,
    auth: LocalApiAuth,
    origin_policy: OriginPolicy,
}

impl HttpServerConfig {
    pub fn new(bind_addr: SocketAddr) -> Result<Self, HttpServerError> {
        if !bind_addr.ip().is_loopback() {
            return Err(HttpServerError::NonLoopbackBindRejected);
        }

        Ok(Self {
            bind_addr,
            auth: LocalApiAuth::disabled(),
            origin_policy: OriginPolicy::default(),
        })
    }

    pub fn loopback(port: u16) -> Result<Self, HttpServerError> {
        Self::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    }

    fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    #[must_use]
    pub fn with_auth(mut self, auth: LocalApiAuth) -> Self {
        self.auth = auth;
        self
    }

    fn auth(&self) -> LocalApiAuth {
        self.auth.clone()
    }

    fn origin_policy(&self) -> OriginPolicy {
        self.origin_policy.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HttpServerError {
    NonLoopbackBindRejected,
    BindFailed(String),
    Io(String),
    ThreadJoinFailed,
}

impl fmt::Display for HttpServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonLoopbackBindRejected => write!(formatter, "non-loopback bind rejected"),
            Self::BindFailed(error) => write!(formatter, "bind failed: {error}"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
            Self::ThreadJoinFailed => write!(formatter, "server thread join failed"),
        }
    }
}

impl std::error::Error for HttpServerError {}

pub struct ControlPlaneHttpServer {
    listener: TcpListener,
    control_plane: Arc<Mutex<ControlPlane>>,
    router: Arc<Mutex<LocalApiRouter>>,
    auth: LocalApiAuth,
    origin_policy: OriginPolicy,
    stop: Arc<AtomicBool>,
}

impl ControlPlaneHttpServer {
    pub fn bind(
        config: HttpServerConfig,
        control_plane: Arc<Mutex<ControlPlane>>,
    ) -> Result<Self, HttpServerError> {
        Self::bind_with_router(config, control_plane, LocalApiRouter::default())
    }

    pub fn bind_with_router(
        config: HttpServerConfig,
        control_plane: Arc<Mutex<ControlPlane>>,
        router: LocalApiRouter,
    ) -> Result<Self, HttpServerError> {
        let listener = TcpListener::bind(config.bind_addr())
            .map_err(|error| HttpServerError::BindFailed(error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HttpServerError::Io(error.to_string()))?;

        Ok(Self {
            listener,
            control_plane,
            router: Arc::new(Mutex::new(router)),
            auth: config.auth(),
            origin_policy: config.origin_policy(),
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("bound listener should have a local address")
    }

    pub fn spawn(self) -> HttpServerHandle {
        let stop = self.stop.clone();
        let thread = thread::spawn(move || self.serve());
        HttpServerHandle { stop, thread }
    }

    fn serve(self) {
        let _agent_worker =
            agent_worker::AgentWorker::spawn(self.router.clone(), self.stop.clone());
        while !self.stop.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let control_plane = self.control_plane.clone();
                    let router = self.router.clone();
                    let auth = self.auth.clone();
                    let origin_policy = self.origin_policy.clone();
                    let stop = self.stop.clone();
                    thread::spawn(move || {
                        let should_stop =
                            handle_stream(stream, &control_plane, &router, &auth, &origin_policy);
                        if should_stop {
                            stop.store(true, Ordering::SeqCst);
                        }
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    }
}

pub struct HttpServerHandle {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl HttpServerHandle {
    pub fn shutdown(self) -> Result<(), HttpServerError> {
        self.stop.store(true, Ordering::SeqCst);
        TcpStream::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).ok();
        self.join()
    }

    pub fn join(self) -> Result<(), HttpServerError> {
        self.thread
            .join()
            .map_err(|_| HttpServerError::ThreadJoinFailed)
    }
}

fn handle_stream(
    mut stream: TcpStream,
    control_plane: &Arc<Mutex<ControlPlane>>,
    router: &Arc<Mutex<LocalApiRouter>>,
    auth: &LocalApiAuth,
    origin_policy: &OriginPolicy,
) -> bool {
    let request_bytes = match read_http_request(&mut stream) {
        Ok(request) => request,
        Err(HttpRequestReadError::PayloadTooLarge) => {
            let body = r#"{"code":"PAYLOAD_TOO_LARGE","message":"request body is too large"}"#;
            let response = format!(
                "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: null\r\nAccess-Control-Allow-Headers: authorization, content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            return false;
        }
        Err(HttpRequestReadError::Malformed) => {
            let body = r#"{"code":"BAD_REQUEST","message":"malformed http request"}"#;
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: null\r\nAccess-Control-Allow-Headers: authorization, content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            return false;
        }
    };
    let request = String::from_utf8_lossy(&request_bytes);
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or_default();
    let headers: Vec<&str> = lines.by_ref().take_while(|line| !line.is_empty()).collect();
    let authorization_header = find_header(&headers, "authorization");
    let host_header = find_header(&headers, "host");
    let origin_header = find_header(&headers, "origin");
    let protected = protected_route(request_line);
    let request_origin = LocalApiRequestOrigin::new(host_header, origin_header);
    let cors_decision = origin_policy.evaluate(&request_origin, protected);
    if let CorsDecision::Rejected { reason } = cors_decision {
        let body = forbidden(reason);
        let response = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: null\r\nAccess-Control-Allow-Headers: authorization, content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        return false;
    }
    let body_start = request
        .find("\r\n\r\n")
        .map(|index| index + 4)
        .unwrap_or(request.len());
    let request_body = &request[body_start..];
    let (status, body, should_stop) = response_for(
        request_line,
        authorization_header,
        request_body,
        control_plane,
        router,
        auth,
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Headers: authorization, content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        cors_origin(cors_decision),
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    should_stop
}

fn response_for(
    request_line: &str,
    authorization_header: Option<&str>,
    request_body: &str,
    control_plane: &Arc<Mutex<ControlPlane>>,
    router: &Arc<Mutex<LocalApiRouter>>,
    auth: &LocalApiAuth,
) -> (&'static str, String, bool) {
    if request_line.starts_with("OPTIONS ") {
        return ("204 No Content", String::new(), false);
    }

    if protected_route(request_line) {
        match auth.authorize(authorization_header) {
            AuthDecision::Allowed => {}
            AuthDecision::Missing => {
                return unauthorized("local auth token is missing");
            }
            AuthDecision::Invalid => {
                return unauthorized("local auth token is invalid");
            }
        }
    }

    match request_line {
        "GET /health HTTP/1.1" => {
            let status = control_plane.lock().unwrap().health().status();
            let value = match status {
                ControlPlaneStatus::Healthy => "healthy",
                ControlPlaneStatus::Draining => "draining",
            };
            ("200 OK", format!(r#"{{"status":"{value}"}}"#), false)
        }
        "GET /v1/readiness HTTP/1.1" => {
            let state = control_plane.lock().unwrap().readiness().state();
            let value = match state {
                ReadinessState::Starting => "starting",
                ReadinessState::Ready => "ready",
            };
            ("200 OK", format!(r#"{{"state":"{value}"}}"#), false)
        }
        "GET /v1/version HTTP/1.1" => {
            let guard = control_plane.lock().unwrap();
            (
                "200 OK",
                format!(
                    r#"{{"productVersion":"{}","apiVersion":"{}"}}"#,
                    guard.version().product_version(),
                    guard.version().api_version()
                ),
                false,
            )
        }
        "POST /v1/shutdown HTTP/1.1" => {
            control_plane
                .lock()
                .unwrap()
                .request_shutdown(ShutdownMode::Graceful);
            ("200 OK", r#"{"shutdown":"graceful"}"#.to_string(), true)
        }
        _ => {
            if let Some((method, path)) = request_parts(request_line) {
                if let Some(response) = router
                    .lock()
                    .expect("local api router lock should not be poisoned")
                    .route_deferred(method, path, request_body)
                {
                    return (response.status(), response.body().to_string(), false);
                }
            }
            let error = crate::ControlPlaneError::not_found("route not found");
            (
                "404 Not Found",
                format!(
                    r#"{{"code":"{}","message":"{}"}}"#,
                    ErrorCode::as_str(error.code()),
                    error.message()
                ),
                false,
            )
        }
    }
}

fn protected_route(request_line: &str) -> bool {
    request_line.contains(" /v1/")
}

fn cors_origin(decision: CorsDecision<'_>) -> &str {
    match decision {
        CorsDecision::PublicProbe => "*",
        CorsDecision::Allowed { origin } => origin,
        CorsDecision::Rejected { .. } => "null",
    }
}

fn request_parts(request_line: &str) -> Option<(&str, &str)> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    Some((method, path))
}

fn find_header<'a>(headers: &'a [&'a str], name: &str) -> Option<&'a str> {
    headers.iter().find_map(|line| {
        let (candidate, value) = line.split_once(':')?;
        if candidate.trim().eq_ignore_ascii_case(name) {
            Some(value.trim())
        } else {
            None
        }
    })
}
