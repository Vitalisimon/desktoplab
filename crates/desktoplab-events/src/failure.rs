#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureReasonCode {
    NetworkUnavailable,
    PolicyDenied,
    RuntimeUnavailable,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryClassification {
    Retryable,
    NotRetryable,
    RequiresUserAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureReason {
    code: FailureReasonCode,
    message: String,
    retry_classification: RetryClassification,
}

impl FailureReason {
    #[must_use]
    pub fn new(code: FailureReasonCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_classification: RetryClassification::NotRetryable,
        }
    }

    #[must_use]
    pub fn with_retry_classification(mut self, retry_classification: RetryClassification) -> Self {
        self.retry_classification = retry_classification;
        self
    }

    #[must_use]
    pub fn code(&self) -> FailureReasonCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn retry_classification(&self) -> RetryClassification {
        self.retry_classification
    }
}
