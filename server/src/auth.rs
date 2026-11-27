//! Email-code authentication and opaque bearer sessions (`docs/13`,
//! «Вход и токены»).

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use refl_core::ids::LearnerId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::{self, AccessDecision, AccessStatus, RegistrationMode, notify_pending};
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
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramLogin {
    pub id: i64,
    pub first_name: String,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PendingApprovalResponse {
    pub status: &'static str,
    pub learner_id: Uuid,
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

fn is_bootstrap_admin(state: &AppState, kind: &str, identifier: &str) -> bool {
    match kind {
        "email" => state
            .config
            .admin_email
            .as_deref()
            .is_some_and(|email| email.trim().eq_ignore_ascii_case(identifier.trim())),
        "telegram" => state
            .config
            .admin_telegram_id
            .is_some_and(|id| id.to_string() == identifier),
        _ => false,
    }
}

#[derive(Debug)]
struct ExistingLogin {
    learner_id: Uuid,
    status: AccessStatus,
    erased: bool,
}

async fn find_login(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    identifier: &str,
) -> Result<Option<ExistingLogin>, AppError> {
    let row: Option<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT u.id, u.access_status, \
         EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = u.id) \
         FROM login_methods lm JOIN users u ON u.id = lm.user_id \
         WHERE lm.kind = $1 AND lm.identifier = $2",
    )
    .bind(kind)
    .bind(identifier)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((learner_id, status, erased)) = row else {
        return Ok(None);
    };
    let Some(status) = AccessStatus::parse(&status) else {
        return Err(AppError::internal("unknown access_status"));
    };
    Ok(Some(ExistingLogin {
        learner_id,
        status,
        erased,
    }))
}

async fn ensure_login_role(
    tx: &mut Transaction<'_, Postgres>,
    learner_id: Uuid,
    kind: &str,
    identifier: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_roles (user_id, role) VALUES ($1, 'learner') \
         ON CONFLICT DO NOTHING",
    )
    .bind(learner_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE login_methods SET verified_at = now() \
         WHERE kind = $1 AND identifier = $2 AND verified_at IS NULL",
    )
    .bind(kind)
    .bind(identifier)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_login_user(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    identifier: &str,
    display_name: &str,
    access_status: AccessStatus,
) -> Result<Uuid, AppError> {
    let learner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users \
         (id, public_code, display_name, interface_lang, access_status) \
         VALUES ($1, $2, $3, 'en', $4)",
    )
    .bind(learner_id)
    .bind(new_public_code())
    .bind(display_name)
    .bind(access_status.as_str())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO login_methods (id, user_id, kind, identifier, verified_at) \
         VALUES ($1, $2, $3, $4, now())",
    )
    .bind(Uuid::new_v4())
    .bind(learner_id)
    .bind(kind)
    .bind(identifier)
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO user_roles (user_id, role) VALUES ($1, 'learner')")
        .bind(learner_id)
        .execute(&mut **tx)
        .await?;
    Ok(learner_id)
}

#[derive(Debug, Clone, Copy)]
enum PreparedUser {
    Active(Uuid),
    Pending(Uuid),
}

struct LoginAttempt<'a> {
    kind: &'a str,
    identifier: &'a str,
    display_name: &'a str,
    admin: bool,
}

async fn prepare_user(
    tx: &mut Transaction<'_, Postgres>,
    mode: RegistrationMode,
    existing: Option<ExistingLogin>,
    invite_code: Option<&str>,
    login: LoginAttempt<'_>,
) -> Result<PreparedUser, AppError> {
    if let Some(existing) = existing {
        if existing.erased {
            return Err(AppError::Erased);
        }
        if login.admin {
            access::ensure_admin_role(tx, existing.learner_id).await?;
            ensure_login_role(tx, existing.learner_id, login.kind, login.identifier).await?;
            return Ok(PreparedUser::Active(existing.learner_id));
        }
        let decision = access::decide_access(
            tx,
            mode,
            Some((existing.learner_id, existing.status)),
            invite_code,
        )
        .await?;
        return match decision {
            AccessDecision::Active => {
                ensure_login_role(tx, existing.learner_id, login.kind, login.identifier).await?;
                Ok(PreparedUser::Active(existing.learner_id))
            }
            AccessDecision::Pending => Err(AppError::PendingApproval),
        };
    }

    if login.admin {
        let learner_id = create_login_user(
            tx,
            login.kind,
            login.identifier,
            login.display_name,
            AccessStatus::Active,
        )
        .await?;
        access::ensure_admin_role(tx, learner_id).await?;
        return Ok(PreparedUser::Active(learner_id));
    }

    let decision = access::decide_access(tx, mode, None, invite_code).await?;
    let status = match decision {
        AccessDecision::Active => AccessStatus::Active,
        AccessDecision::Pending => AccessStatus::Pending,
    };
    let learner_id =
        create_login_user(tx, login.kind, login.identifier, login.display_name, status).await?;
    Ok(match decision {
        AccessDecision::Active => PreparedUser::Active(learner_id),
        AccessDecision::Pending => PreparedUser::Pending(learner_id),
    })
}

/// # Errors
/// `400` for an invalid email; `401` for a missing, expired, or reused code;
/// `403` for an erased, pending, rejected, or invite-gated account; `500` on
/// database errors. A new user under closed registration receives `202` with
/// no session tokens.
pub async fn verify_code(
    State(state): State<AppState>,
    Json(payload): Json<VerifyCode>,
) -> Result<Response, AppError> {
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

    let existing = find_login(&mut tx, "email", &email).await?;
    let prepared = prepare_user(
        &mut tx,
        state.config.registration_mode,
        existing,
        payload.invite_code.as_deref(),
        LoginAttempt {
            kind: "email",
            identifier: &email,
            display_name: display_name(&email),
            admin: is_bootstrap_admin(&state, "email", &email),
        },
    )
    .await?;
    match prepared {
        PreparedUser::Active(learner_id) => {
            let tokens = issue_session(&mut tx, learner_id).await?;
            tx.commit().await?;
            Ok(Json(tokens).into_response())
        }
        PreparedUser::Pending(learner_id) => {
            tx.commit().await?;
            notify_pending(
                &*state.telegram,
                state.config.admin_telegram_chat_id,
                learner_id,
                &email,
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(PendingApprovalResponse {
                    status: "pending_approval",
                    learner_id,
                }),
            )
                .into_response())
        }
    }
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let digest = Sha256::digest(key);
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0u8; 64];
    let mut outer_pad = [0u8; 64];
    for ((inner, outer), byte) in inner_pad
        .iter_mut()
        .zip(outer_pad.iter_mut())
        .zip(key_block.iter())
    {
        *inner = byte ^ 0x36;
        *outer = byte ^ 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn telegram_data_check(login: &TelegramLogin) -> String {
    let mut fields: Vec<(&str, String)> = vec![
        ("auth_date", login.auth_date.to_string()),
        ("first_name", login.first_name.clone()),
        ("id", login.id.to_string()),
    ];
    if let Some(last_name) = &login.last_name {
        fields.push(("last_name", last_name.clone()));
    }
    if let Some(photo_url) = &login.photo_url {
        fields.push(("photo_url", photo_url.clone()));
    }
    if let Some(username) = &login.username {
        fields.push(("username", username.clone()));
    }
    fields.sort_by(|left, right| left.0.cmp(right.0));
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn unix_seconds() -> Result<i64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::internal)?;
    i64::try_from(elapsed.as_secs()).map_err(AppError::internal)
}

fn telegram_display_name(login: &TelegramLogin) -> String {
    let mut name = login.first_name.trim().to_owned();
    if let Some(last_name) = login.last_name.as_deref().map(str::trim)
        && !last_name.is_empty()
    {
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(last_name);
    }
    if !name.is_empty() {
        return name;
    }
    login
        .username
        .as_deref()
        .filter(|username| !username.trim().is_empty())
        .map_or_else(|| "Telegram".into(), str::to_owned)
}

/// # Errors
/// `400` for an invalid Telegram identity; `401` for a bad signature or stale
/// `auth_date`; `403` for an erased, pending, rejected, or invite-gated
/// account; `503` when Telegram login is not configured.
pub async fn telegram(
    State(state): State<AppState>,
    Json(payload): Json<TelegramLogin>,
) -> Result<Response, AppError> {
    let token = state
        .config
        .telegram_bot_token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or(AppError::ServiceUnavailable)?;
    if payload.id <= 0 || payload.first_name.trim().is_empty() {
        return Err(AppError::BadRequest("invalid telegram identity".into()));
    }
    let now = unix_seconds()?;
    if now.saturating_sub(payload.auth_date) > 300 {
        return Err(AppError::Unauthorized);
    }
    let secret = Sha256::digest(token.as_bytes());
    let expected = hex(&hmac_sha256(
        &secret,
        telegram_data_check(&payload).as_bytes(),
    ));
    if !expected.eq_ignore_ascii_case(&payload.hash) {
        return Err(AppError::Unauthorized);
    }

    let identifier = payload.id.to_string();
    let display_name = telegram_display_name(&payload);
    let mut tx = state.pool.begin().await?;
    let existing = find_login(&mut tx, "telegram", &identifier).await?;
    let prepared = prepare_user(
        &mut tx,
        state.config.registration_mode,
        existing,
        payload.invite_code.as_deref(),
        LoginAttempt {
            kind: "telegram",
            identifier: &identifier,
            display_name: &display_name,
            admin: is_bootstrap_admin(&state, "telegram", &identifier),
        },
    )
    .await?;
    match prepared {
        PreparedUser::Active(learner_id) => {
            let tokens = issue_session(&mut tx, learner_id).await?;
            tx.commit().await?;
            Ok(Json(tokens).into_response())
        }
        PreparedUser::Pending(learner_id) => {
            tx.commit().await?;
            notify_pending(
                &*state.telegram,
                state.config.admin_telegram_chat_id,
                learner_id,
                &display_name,
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(PendingApprovalResponse {
                    status: "pending_approval",
                    learner_id,
                }),
            )
                .into_response())
        }
    }
}

/// # Errors
/// `401` for an invalid, revoked, or expired refresh token; `403` for an
/// erased or non-active account; `500` on database errors.
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<Refresh>,
) -> Result<Json<AuthTokens>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::Unauthorized);
    }
    let mut tx = state.pool.begin().await?;
    let row: Option<(Uuid, Uuid, String, bool)> = sqlx::query_as(
        "SELECT s.id, s.user_id, u.access_status, \
         EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = s.user_id) \
         FROM auth_sessions s JOIN users u ON u.id = s.user_id \
         WHERE s.refresh_token_hash = $1 AND s.revoked_at IS NULL \
         AND s.refresh_expires_at > now() FOR UPDATE",
    )
    .bind(token_hash(payload.refresh_token.trim()))
    .fetch_optional(&mut *tx)
    .await?;
    let Some((session_id, learner_id, access_status, erased)) = row else {
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
    match AccessStatus::parse(&access_status) {
        Some(AccessStatus::Active) => {}
        Some(AccessStatus::Pending) => return Err(AppError::PendingApproval),
        Some(AccessStatus::Rejected) => return Err(AppError::AccessRejected),
        None => return Err(AppError::internal("unknown access_status")),
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
        let session: Option<(Uuid, Uuid, String, bool)> = sqlx::query_as(
            "SELECT s.id, s.user_id, u.access_status, \
             EXISTS(SELECT 1 FROM erasures e WHERE e.learner_id = s.user_id) \
             FROM auth_sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.access_token_hash = $1 AND s.revoked_at IS NULL \
             AND s.access_expires_at > now()",
        )
        .bind(token_hash(token))
        .fetch_optional(&state.pool)
        .await?;
        if let Some((session_id, learner_id, access_status, erased)) = session {
            if erased {
                return Ok(Self {
                    learner_id: LearnerId(learner_id),
                    session_id: Some(session_id),
                    erased: true,
                });
            }
            match AccessStatus::parse(&access_status) {
                Some(AccessStatus::Active) => {
                    return Ok(Self {
                        learner_id: LearnerId(learner_id),
                        session_id: Some(session_id),
                        erased: false,
                    });
                }
                Some(AccessStatus::Pending) => return Err(AppError::PendingApproval),
                Some(AccessStatus::Rejected) => return Err(AppError::AccessRejected),
                None => return Err(AppError::internal("unknown access_status")),
            }
        }

        if state.config.allow_header_learner_id
            && let Ok(learner_id) = Uuid::parse_str(token)
        {
            let (exists, erased, access_status): (bool, bool, Option<String>) = sqlx::query_as(
                "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1), \
                 EXISTS(SELECT 1 FROM erasures WHERE learner_id = $1), \
                 (SELECT access_status FROM users WHERE id = $1)",
            )
            .bind(learner_id)
            .fetch_one(&state.pool)
            .await?;
            if exists {
                if erased {
                    return Ok(Self {
                        learner_id: LearnerId(learner_id),
                        session_id: None,
                        erased: true,
                    });
                }
                return match access_status.as_deref().and_then(AccessStatus::parse) {
                    Some(AccessStatus::Active) => Ok(Self {
                        learner_id: LearnerId(learner_id),
                        session_id: None,
                        erased: false,
                    }),
                    Some(AccessStatus::Pending) => Err(AppError::PendingApproval),
                    Some(AccessStatus::Rejected) => Err(AppError::AccessRejected),
                    None => Err(AppError::internal("unknown access_status")),
                };
            }
        }
        Err(AppError::Unauthorized)
    }
}
