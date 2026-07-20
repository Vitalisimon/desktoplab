use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
}

impl ErrorCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPlaneError {
    code: ErrorCode,
    message: String,
    http_status: u16,
}

impl ControlPlaneError {
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Unauthorized,
            message: message.into(),
            http_status: 401,
        }
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
            http_status: 404,
        }
    }

    #[must_use]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Forbidden,
            message: message.into(),
            http_status: 403,
        }
    }

    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn http_status(&self) -> u16 {
        self.http_status
    }
}

impl fmt::Display for ControlPlaneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ControlPlaneError {}
