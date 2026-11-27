//! Router, shared state, authentication extractor and the small `/health` and
//! `/v1/content/active` endpoints.

use std::future::Future;

use axum::extract::{FromRequestParts, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::routing::{get, post};
use axum::{Json, Router};
use refl_core::ids::LearnerId;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::content;
use crate::error::AppError;
use crate::sync;

/// Used by the server and served to clients until a release raises it.
pub const DEFAULT_MIN_CORE_VERSION: &str = "0.0.0";

#[derive(Debug, Clone)]
pub struct Config {
    /// The P1a development/test authentication: `Authorization: Bearer <uuid>`.
    pub allow_header_learner_id: bool,
    pub min_core_version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_header_learner_id: false,
            min_core_version: DEFAULT_MIN_CORE_VERSION.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

/// The authenticated learner. P1b replaces the implementation of the
/// [`FromRequestParts`] below with token verification; handlers keep taking
/// this extractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedLearner(pub LearnerId);

impl FromRequestParts<AppState> for AuthenticatedLearner {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(header_learner(parts, state))
    }
}

fn header_learner(parts: &Parts, state: &AppState) -> Result<AuthenticatedLearner, AppError> {
    if !state.config.allow_header_learner_id {
        return Err(AppError::Unauthorized);
    }
    let value = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let (scheme, token) = value.split_once(' ').ok_or(AppError::Unauthorized)?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return Err(AppError::Unauthorized);
    }
    let uuid = Uuid::parse_str(token).map_err(|_| AppError::Unauthorized)?;
    Ok(AuthenticatedLearner(LearnerId(uuid)))
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
