//! Typed HTTP errors. Handlers return [`AppError`]; the response is always a
//! JSON object with a stable machine-readable `error` code.

use axum::Json;
use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    Erased,
    PendingApproval,
    AccessRejected,
    InviteInvalid,
    ServiceUnavailable,
    TooManyRequests,
    NotFound(&'static str),
    UpgradeRequired { min_core_version: String },
    Internal(String),
}

impl AppError {
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal(error.to_string())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::internal(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, min_core_version) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message, None),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "authentication required".into(),
                None,
            ),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, "forbidden", message, None),
            Self::Erased => (
                StatusCode::FORBIDDEN,
                "erased",
                "account erased".into(),
                None,
            ),
            Self::PendingApproval => (
                StatusCode::FORBIDDEN,
                "pending_approval",
                "access is pending approval".into(),
                None,
            ),
            Self::AccessRejected => (
                StatusCode::FORBIDDEN,
                "access_rejected",
                "access was rejected".into(),
                None,
            ),
            Self::InviteInvalid => (
                StatusCode::FORBIDDEN,
                "invite_invalid",
                "invite is invalid, expired, or exhausted".into(),
                None,
            ),
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                "telegram login is not configured".into(),
                None,
            ),
            Self::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests".into(),
                None,
            ),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message.into(), None),
            Self::UpgradeRequired { min_core_version } => (
                StatusCode::UPGRADE_REQUIRED,
                "core_version_too_old",
                "update the app before syncing".into(),
                Some(min_core_version),
            ),
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal server error".into(),
                None,
            ),
        };
        let mut body = json!({ "error": code, "message": message });
        if let Some(min) = min_core_version {
            body["min_core_version"] = json!(min);
        }
        let mut response = (status, Json(body)).into_response();
        if status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}
