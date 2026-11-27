//! Email-code authentication and opaque bearer sessions (`docs/13`,
//! «Вход и токены»).

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use refl_core::ids::LearnerId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;
use crate::routes::AppState;

const CODE_TTL_SECONDS: i64 = 10 * 60;
const ACCESS_TTL_SECONDS: i64 = 15 * 60;
const MAX_CODE_ATTEMPTS: i32 = 5;

/// Delivery is deliberately a trait: production can replace the no-op stub
/// with an SMTP implementation without changing the handlers.
pub trait CodeDelivery: Send + Sync {
    fn deliver(&self, email: &str, code: &str);
}

#[derive(Debug, Default)]
pub struct DevLogDelivery;

impl CodeDelivery for DevLogDelivery {
    fn deliver(&self, email: &str, code: &str) {
        eprintln!("refl dev auth code for {email}: {code}");
    }
}

#[derive(Debug, Default)]
pub struct NoopOrSmtpStubDelivery;

impl CodeDelivery for NoopOrSmtpStubDelivery {
    fn deliver(&self, _email: &str, _code: &str) {}
}

static DEV_LOG_DELIVERY: DevLogDelivery = DevLogDelivery;
static NOOP_DELIVERY: NoopOrSmtpStubDelivery = NoopOrSmtpStubDelivery;

fn delivery(dev_auth: bool) -> &'static dyn CodeDelivery {
    if dev_auth {
        &DEV_LOG_DELIVERY
    } else {
        &NOOP_DELIVERY
    }
}

fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    let (local, domain) = email
        .split_once('@')
        .ok_or_else(|| AppError::BadRequest("invalid email".into()))?;
    if local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || !domain.contains('.')
        || email.len() > 254
        || email.chars().any(char::is_whitespace)
    {
        return Err(AppError::BadRequest("invalid email".into()));
    }
    Ok(email)
}

fn valid_code(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
}

fn code_hash(email: &str, code: &str) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(email.as_bytes());
    hash.update(b":");
    hash.update(code.as_bytes());
    hash.finalize().to_vec()
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn random_token() -> String {
    format!("{:032x}", Uuid::new_v4().as_u128())
}

fn random_code() -> String {
    format!("{:06}", Uuid::new_v4().as_u128() % 1_000_000)
}

fn new_public_code() -> String {
    let mut code = Uuid::new_v4().simple().to_string().to_uppercase();
    code.truncate(8);
    code
}

#[derive(Debug, Deserialize)]
pub struct RequestCode {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct RequestCodeResponse {
    pub sent: bool,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCode {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub learner_id: Uuid,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct Refresh {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub logged_out: bool,
}

/// # Errors
/// `400` for an invalid email; `429` while the per-email cooldown is active;
/// `500` on database errors.
pub async fn request_code(
    State(state): State<AppState>,
    Json(payload): Json<RequestCode>,
) -> Result<Json<RequestCodeResponse>, AppError> {
    let email = normalize_email(&payload.email)?;
    let mut tx = state.pool.begin().await?;
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM login_tokens WHERE email = $1 \
         AND consumed_at IS NULL AND created_at > now() - interval '60 seconds')",
    )
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;
    if recent {
        return Err(AppError::TooManyRequests);
    }
    sqlx::query("DELETE FROM login_tokens WHERE email = $1")
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    let code = random_code();
    sqlx::query(
        "INSERT INTO login_tokens (token_hash, email, expires_at) \
         VALUES ($1, $2, now() + interval '10 minutes')",
    )
    .bind(code_hash(&email, &code))
    .bind(&email)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    delivery(state.config.dev_auth).deliver(&email, &code);
    let dev_code = state.config.dev_auth.then_some(code);
    Ok(Json(RequestCodeResponse {
        sent: true,
        expires_in: CODE_TTL_SECONDS,
        code: dev_code,
    }))
}

async fn issue_session(
    tx: &mut Transaction<'_, Postgres>,
    learner_id: Uuid,
) -> Result<AuthTokens, AppError> {
    let access_token = random_token();
    let refresh_token = random_token();
    sqlx::query(
        "INSERT INTO auth_sessions \
         (id, user_id, access_token_hash, refresh_token_hash, access_expires_at, refresh_expires_at) \
         VALUES ($1, $2, $3, $4, now() + interval '15 minutes', now() + interval '90 days')",
    )
    .bind(Uuid::new_v4())
    .bind(learner_id)
    .bind(token_hash(&access_token))
    .bind(token_hash(&refresh_token))
    .execute(&mut **tx)
    .await?;
    Ok(AuthTokens {
        access_token,
        refresh_token,
        learner_id,
        expires_in: ACCESS_TTL_SECONDS,
    })
}

fn display_name(email: &str) -> &str {
    email
        .split_once('@')
        .map_or("Learner", |(local, _domain)| local)
}

enum UserLookup {
    Existing(Uuid),
    Erased,
}

async fn find_or_create_user(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<UserLookup, AppError> {
    let existing: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT u.id, EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = u.id) \
         FROM login_methods lm JOIN users u ON u.id = lm.user_id \
         WHERE lm.kind = 'email' AND lm.identifier = $1",
    )
    .bind(email)
    .fetch_optional(&mut **tx)
    .await?;

    match existing {
        Some((_, true)) => Ok(UserLookup::Erased),
        Some((learner_id, false)) => {
            sqlx::query(
                "INSERT INTO user_roles (user_id, role) VALUES ($1, 'learner') \
                 ON CONFLICT DO NOTHING",
            )
            .bind(learner_id)
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "UPDATE login_methods SET verified_at = now() \
                 WHERE kind = 'email' AND identifier = $1 AND verified_at IS NULL",
            )
            .bind(email)
            .execute(&mut **tx)
            .await?;
            Ok(UserLookup::Existing(learner_id))
        }
        None => {
            let learner_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO users (id, public_code, display_name, interface_lang) \
                 VALUES ($1, $2, $3, 'en')",
            )
            .bind(learner_id)
            .bind(new_public_code())
            .bind(display_name(email))
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "INSERT INTO login_methods (id, user_id, kind, identifier, verified_at) \
                 VALUES ($1, $2, 'email', $3, now())",
            )
            .bind(Uuid::new_v4())
            .bind(learner_id)
            .bind(email)
            .execute(&mut **tx)
            .await?;
            sqlx::query("INSERT INTO user_roles (user_id, role) VALUES ($1, 'learner')")
                .bind(learner_id)
                .execute(&mut **tx)
                .await?;
            Ok(UserLookup::Existing(learner_id))
        }
    }
}

/// # Errors
/// `400` for an invalid email; `401` for a missing, expired, or reused code;
/// `403` for an erased account; `500` on database errors.
pub async fn verify_code(
    State(state): State<AppState>,
    Json(payload): Json<VerifyCode>,
) -> Result<Json<AuthTokens>, AppError> {
    let email = normalize_email(&payload.email)?;
    if !valid_code(&payload.code) {
        return Err(AppError::Unauthorized);
    }
    let mut tx = state.pool.begin().await?;
    let row: Option<(Vec<u8>, i32)> = sqlx::query_as(
        "SELECT token_hash, attempts FROM login_tokens \
         WHERE email = $1 AND consumed_at IS NULL AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((stored_hash, attempts)) = row else {
        return Err(AppError::Unauthorized);
    };
    if stored_hash != code_hash(&email, &payload.code) {
        let next = attempts.saturating_add(1);
        if next >= MAX_CODE_ATTEMPTS {
            sqlx::query(
                "UPDATE login_tokens SET attempts = $2, consumed_at = now() \
                 WHERE token_hash = $1",
            )
            .bind(&stored_hash)
            .bind(next)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("UPDATE login_tokens SET attempts = $2 WHERE token_hash = $1")
                .bind(&stored_hash)
                .bind(next)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        return Err(AppError::Unauthorized);
    }

    sqlx::query("UPDATE login_tokens SET consumed_at = now() WHERE token_hash = $1")
        .bind(&stored_hash)
        .execute(&mut *tx)
        .await?;

    let learner_id = match find_or_create_user(&mut tx, &email).await? {
        UserLookup::Existing(learner_id) => learner_id,
        UserLookup::Erased => {
            tx.commit().await?;
            return Err(AppError::Erased);
        }
    };

    let tokens = issue_session(&mut tx, learner_id).await?;
    tx.commit().await?;
    Ok(Json(tokens))
}

/// # Errors
/// `401` for an invalid, revoked, or expired refresh token; `403` for an
/// erased account; `500` on database errors.
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<Refresh>,
) -> Result<Json<AuthTokens>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::Unauthorized);
    }
    let mut tx = state.pool.begin().await?;
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT s.id, s.user_id, \
         EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = s.user_id) \
         FROM auth_sessions s \
         WHERE s.refresh_token_hash = $1 AND s.revoked_at IS NULL \
         AND s.refresh_expires_at > now() FOR UPDATE",
    )
    .bind(token_hash(payload.refresh_token.trim()))
    .fetch_optional(&mut *tx)
    .await?;
    let Some((session_id, learner_id, erased)) = row else {
        return Err(AppError::Unauthorized);
    };
    if erased {
        sqlx::query("UPDATE auth_sessions SET revoked_at = now() WHERE id = $1")
            .bind(session_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(AppError::Erased);
    }

    let access_token = random_token();
    let refresh_token = random_token();
    sqlx::query(
        "UPDATE auth_sessions SET access_token_hash = $2, refresh_token_hash = $3, \
         access_expires_at = now() + interval '15 minutes', \
         refresh_expires_at = now() + interval '90 days', updated_at = now() \
         WHERE id = $1",
    )
    .bind(session_id)
    .bind(token_hash(&access_token))
    .bind(token_hash(&refresh_token))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(AuthTokens {
        access_token,
        refresh_token,
        learner_id,
        expires_in: ACCESS_TTL_SECONDS,
    }))
}

/// # Errors
/// `400` if the token is empty; `500` on database errors.
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<Refresh>,
) -> Result<Json<LogoutResponse>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::BadRequest("refresh_token required".into()));
    }
    sqlx::query(
        "UPDATE auth_sessions SET revoked_at = now(), updated_at = now() \
         WHERE refresh_token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(token_hash(payload.refresh_token.trim()))
    .execute(&state.pool)
    .await?;
    Ok(Json(LogoutResponse { logged_out: true }))
}

/// The authenticated learner. The access token is opaque; only its SHA-256 is
/// stored in `auth_sessions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedLearner {
    pub learner_id: LearnerId,
    pub session_id: Option<Uuid>,
    pub erased: bool,
}

fn bearer_token(parts: &Parts) -> Result<&str, AppError> {
    let value = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let (scheme, token) = value.split_once(' ').ok_or(AppError::Unauthorized)?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Ok(token)
    } else {
        Err(AppError::Unauthorized)
    }
}

impl FromRequestParts<AppState> for AuthenticatedLearner {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;
        let session: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT s.id, s.user_id, \
             EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = s.user_id) \
             FROM auth_sessions s \
             WHERE s.access_token_hash = $1 AND s.revoked_at IS NULL \
             AND s.access_expires_at > now()",
        )
        .bind(token_hash(token))
        .fetch_optional(&state.pool)
        .await?;
        if let Some((session_id, learner_id, erased)) = session {
            return Ok(Self {
                learner_id: LearnerId(learner_id),
                session_id: Some(session_id),
                erased,
            });
        }

        if state.config.allow_header_learner_id
            && let Ok(learner_id) = Uuid::parse_str(token)
        {
            let (exists, erased): (bool, bool) = sqlx::query_as(
                "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1), \
                 EXISTS(SELECT 1 FROM erasures WHERE learner_id = $1)",
            )
            .bind(learner_id)
            .fetch_one(&state.pool)
            .await?;
            if exists {
                return Ok(Self {
                    learner_id: LearnerId(learner_id),
                    session_id: None,
                    erased,
                });
            }
        }
        Err(AppError::Unauthorized)
    }
}
