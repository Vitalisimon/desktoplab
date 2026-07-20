#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenAiCompatibleEndpointPolicy {
    allow_remote_https: bool,
}

impl OpenAiCompatibleEndpointPolicy {
    #[must_use]
    pub fn local_only() -> Self {
        Self {
            allow_remote_https: false,
        }
    }

    #[must_use]
    pub fn allow_remote_https() -> Self {
        Self {
            allow_remote_https: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenAiCompatibleEndpointClass {
    Localhost,
    RemoteHttps,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCompatibleEndpoint {
    url: String,
    class: OpenAiCompatibleEndpointClass,
}

impl OpenAiCompatibleEndpoint {
    pub fn validate(
        raw_url: &str,
        policy: OpenAiCompatibleEndpointPolicy,
    ) -> Result<Self, OpenAiCompatibleEndpointError> {
        let url = raw_url.trim();
        if url.is_empty() {
            return Err(OpenAiCompatibleEndpointError::Empty);
        }
        if url.contains(char::is_whitespace) {
            return Err(OpenAiCompatibleEndpointError::ContainsWhitespace);
        }
        if contains_secret_query(url) {
            return Err(OpenAiCompatibleEndpointError::SecretInUrl);
        }
        let class = classify_url(url, policy)?;
        if !url.contains("/v1") {
            return Err(OpenAiCompatibleEndpointError::MissingOpenAiPath);
        }
        Ok(Self {
            url: url.to_string(),
            class,
        })
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn class(&self) -> OpenAiCompatibleEndpointClass {
        self.class
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenAiCompatibleEndpointError {
    Empty,
    ContainsWhitespace,
    MissingOpenAiPath,
    RemoteEndpointBlocked,
    SecretInUrl,
    UnsupportedScheme,
}

fn classify_url(
    url: &str,
    policy: OpenAiCompatibleEndpointPolicy,
) -> Result<OpenAiCompatibleEndpointClass, OpenAiCompatibleEndpointError> {
    if is_localhost_url(url) {
        return Ok(OpenAiCompatibleEndpointClass::Localhost);
    }
    if url.starts_with("https://") {
        if !policy.allow_remote_https {
            return Err(OpenAiCompatibleEndpointError::RemoteEndpointBlocked);
        }
        return Ok(OpenAiCompatibleEndpointClass::RemoteHttps);
    }
    Err(OpenAiCompatibleEndpointError::UnsupportedScheme)
}

fn is_localhost_url(url: &str) -> bool {
    url.starts_with("http://127.0.0.1")
        || url.starts_with("http://localhost")
        || url.starts_with("http://[::1]")
        || url.starts_with("https://localhost")
}

fn contains_secret_query(url: &str) -> bool {
    let Some(query) = url
        .split_once('?')
        .map(|(_, query)| query.to_ascii_lowercase())
    else {
        return false;
    };
    ["api_key=", "apikey=", "token=", "key=", "authorization="]
        .iter()
        .any(|needle| query.contains(needle))
}
