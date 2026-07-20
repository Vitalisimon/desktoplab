#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApiRequestOrigin<'a> {
    host: Option<&'a str>,
    origin: Option<&'a str>,
}

impl<'a> LocalApiRequestOrigin<'a> {
    #[must_use]
    pub const fn new(host: Option<&'a str>, origin: Option<&'a str>) -> Self {
        Self { host, origin }
    }

    #[must_use]
    pub fn host_is_loopback(&self) -> bool {
        let Some(host) = self.host.map(trim_host_port) else {
            return false;
        };
        matches!(host, "localhost" | "127.0.0.1" | "::1")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginPolicy {
    allowed_origins: Vec<String>,
}

impl OriginPolicy {
    #[must_use]
    pub fn packaged_default() -> Self {
        Self {
            allowed_origins: vec![
                "tauri://localhost".to_string(),
                "http://tauri.localhost".to_string(),
                "http://127.0.0.1:1420".to_string(),
                "http://localhost:1420".to_string(),
            ],
        }
    }

    #[must_use]
    pub fn evaluate<'a>(
        &self,
        request: &'a LocalApiRequestOrigin<'a>,
        protected: bool,
    ) -> CorsDecision<'a> {
        if !protected {
            return CorsDecision::PublicProbe;
        }

        if !request.host_is_loopback() {
            return CorsDecision::Rejected {
                reason: "non-loopback host rejected",
            };
        }

        let Some(origin) = request.origin else {
            return CorsDecision::Allowed { origin: "null" };
        };

        if self.allowed_origins.iter().any(|allowed| allowed == origin) {
            return CorsDecision::Allowed { origin };
        }

        CorsDecision::Rejected {
            reason: "origin rejected",
        }
    }
}

impl Default for OriginPolicy {
    fn default() -> Self {
        Self::packaged_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorsDecision<'a> {
    PublicProbe,
    Allowed { origin: &'a str },
    Rejected { reason: &'static str },
}

fn trim_host_port(host: &str) -> &str {
    let trimmed = host.trim();
    if let Some(ipv6) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.split(']').next())
    {
        return ipv6;
    }

    trimmed.split(':').next().unwrap_or(trimmed)
}
