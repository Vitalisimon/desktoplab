use crate::{ControlPlaneError, ErrorCode};

pub(super) fn unauthorized(message: &str) -> (&'static str, String, bool) {
    let error = ControlPlaneError::unauthorized(message);
    (
        "401 Unauthorized",
        format!(
            r#"{{"code":"{}","message":"{}"}}"#,
            ErrorCode::as_str(error.code()),
            error.message()
        ),
        false,
    )
}

pub(super) fn forbidden(message: &str) -> String {
    let error = ControlPlaneError::forbidden(message);
    format!(
        r#"{{"code":"{}","message":"{}"}}"#,
        ErrorCode::as_str(error.code()),
        error.message()
    )
}
