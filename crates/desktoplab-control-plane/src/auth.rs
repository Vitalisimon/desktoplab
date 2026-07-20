#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAuthToken {
    value: String,
}

impl LocalAuthToken {
    #[must_use]
    pub fn for_desktop_session() -> Self {
        let mut bytes = [0_u8; 32];
        getrandom::getrandom(&mut bytes).expect("OS random source should be available");
        Self {
            value: hex_token(&bytes),
        }
    }

    #[must_use]
    pub fn explicit_for_test(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    #[must_use]
    pub fn redacted(&self) -> &'static str {
        "[REDACTED_LOCAL_API_TOKEN]"
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub(crate) fn matches(&self, candidate: &str) -> bool {
        self.value == candidate
    }
}

fn hex_token(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 0x0f) as usize] as char);
    }
    token
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthDecision {
    Allowed,
    Missing,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalApiAuth {
    Disabled,
    Required(LocalAuthToken),
}

impl LocalApiAuth {
    #[must_use]
    pub fn disabled() -> Self {
        Self::Disabled
    }

    #[must_use]
    pub fn required(token: LocalAuthToken) -> Self {
        Self::Required(token)
    }

    pub(crate) fn authorize(&self, authorization_header: Option<&str>) -> AuthDecision {
        match self {
            Self::Disabled => AuthDecision::Allowed,
            Self::Required(expected) => {
                let Some(header) = authorization_header else {
                    return AuthDecision::Missing;
                };
                let Some(token) = header.strip_prefix("Bearer ") else {
                    return AuthDecision::Invalid;
                };
                if expected.matches(token) {
                    AuthDecision::Allowed
                } else {
                    AuthDecision::Invalid
                }
            }
        }
    }
}
