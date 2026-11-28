//! Router, shared state and the small `/health` and `/v1/content/active`
//! endpoints.

use std::fmt;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::access::{self, RegistrationMode, TelegramGateway};
use crate::auth;
use crate::content;
use crate::error::AppError;
use crate::privacy;
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
    /// Bootstrap admin email; matching logins become active admins.
    pub admin_email: Option<String>,
    /// Bootstrap Telegram user id; matching logins become active admins.
    pub admin_telegram_id: Option<i64>,
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
            admin_email: None,
            admin_telegram_id: None,
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
    /// SHA-256 of `bundle_json`, the exact bytes the client verifies and parses.
    pub digest: String,
    /// The parsed bundle, kept for older clients.
    pub bundle: Value,
    /// The exact serialized bundle the digest covers.
    pub bundle_json: String,
}

/// The checker dictionary and the metadata needed to verify it before pairing
/// it with a bundle (`docs/13-sync-versions.md`, «Обновление приложения»).
#[derive(Debug, Serialize)]
pub struct LexiconResponse {
    pub version: String,
    pub sha256: String,
    pub words: i32,
    pub bytes: usize,
    pub text: String,
    pub license: &'static str,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/content/active", get(active_content))
        .route("/v1/content/lexicon/{version}", get(active_lexicon))
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
        .route("/v1/profile", get(privacy::profile))
        .route("/v1/export", get(privacy::export))
        .route("/v1/erase", post(privacy::erase))
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
    // JSONB does not preserve the key order of the imported bytes. Hash the
    // exact string we serve instead, so the client's SHA-256 check is over the
    // bytes it receives rather than an unreproducible prior serialization.
    let bundle_json = serde_json::to_string(&active.bundle)
        .map_err(|error| AppError::internal(format!("bundle does not serialize: {error}")))?;
    let digest = content::sha256_hex(bundle_json.as_bytes());
    Ok(Json(ActiveContentResponse {
        content_version: active.content_version,
        min_core_version: state.config.min_core_version,
        digest,
        bundle: active.bundle,
        bundle_json,
    }))
}

async fn active_lexicon(
    State(state): State<AppState>,
    Path(version): Path<String>,
) -> Result<Json<LexiconResponse>, AppError> {
    if version.len() != 64 || !version.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::NotFound("unknown lexicon version"));
    }
    let stored = content::lexicon(&state.pool, &version)
        .await?
        .ok_or(AppError::NotFound("unknown lexicon version"))?;
    if content::sha256_hex(stored.body.as_bytes()) != stored.version {
        return Err(AppError::internal("stored lexicon digest mismatch"));
    }
    Ok(Json(LexiconResponse {
        version: stored.version.clone(),
        sha256: stored.version,
        words: stored.words,
        bytes: stored.body.len(),
        text: stored.body,
        license: content::SCOWL_LICENSE,
    }))
}
