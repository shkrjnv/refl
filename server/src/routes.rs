//! Router, shared state and the small `/health` and `/v1/content/active`
//! endpoints.

use std::fmt;
use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::access::{self, RegistrationMode, TelegramGateway};
use crate::auth;
use crate::content;
use crate::error::AppError;
use crate::sync;

pub use crate::auth::AuthenticatedLearner;

/// Used by the server and served to clients until a release raises it.
pub const DEFAULT_MIN_CORE_VERSION: &str = "0.0.0";

#[derive(Debug, Clone)]
pub struct Config {
    /// P1a's isolated-test fallback: `Authorization: Bearer <learner_uuid>`.
    pub allow_header_learner_id: bool,
    /// Return and log the six-digit code from `/v1/auth/request-code`.
    pub dev_auth: bool,
    pub registration_mode: RegistrationMode,
    pub telegram_bot_token: Option<String>,
    pub admin_telegram_chat_id: Option<i64>,
    pub min_core_version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_header_learner_id: false,
            dev_auth: false,
            registration_mode: RegistrationMode::Closed,
            telegram_bot_token: None,
            admin_telegram_chat_id: None,
            min_core_version: DEFAULT_MIN_CORE_VERSION.into(),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub telegram: Arc<dyn TelegramGateway>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("pool", &self.pool)
            .field("config", &self.config)
            .field("telegram", &"configured")
            .finish()
    }
}

#[derive(Debug, Serialize)]
pub struct ActiveContentResponse {
    pub content_version: String,
    pub min_core_version: String,
    /// SHA-256 of the complete serialized bundle.
    pub digest: String,
    pub bundle: Value,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/content/active", get(active_content))
        .route("/v1/auth/request-code", post(auth::request_code))
        .route("/v1/auth/verify-code", post(auth::verify_code))
        .route("/v1/auth/telegram", post(auth::telegram))
        .route("/v1/auth/refresh", post(auth::refresh))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/admin/invites", post(access::admin_create_invite))
        .route("/v1/admin/requests", get(access::admin_requests))
        .route(
            "/v1/admin/requests/{user_id}/approve",
            post(access::admin_approve),
        )
        .route(
            "/v1/admin/requests/{user_id}/reject",
            post(access::admin_reject),
        )
        .route("/v1/sync", post(sync::sync))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn active_content(
    State(state): State<AppState>,
) -> Result<Json<ActiveContentResponse>, AppError> {
    let active = content::active_bundle(&state.pool)
        .await?
        .ok_or(AppError::NotFound("no active content bundle"))?;
    Ok(Json(ActiveContentResponse {
        content_version: active.content_version,
        min_core_version: state.config.min_core_version,
        digest: active.digest,
        bundle: active.bundle,
    }))
}
