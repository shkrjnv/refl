//! Registration access control, invites and the Telegram notification seam.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::AuthenticatedLearner;
use crate::error::AppError;
use crate::routes::AppState;

/// Closed registration requires an invite; open registration creates active
/// users directly. Unset means closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegistrationMode {
    #[default]
    Closed,
    Open,
}

impl RegistrationMode {
    pub fn from_value(value: Option<&str>) -> Self {
        if value.is_some_and(|value| value.trim().eq_ignore_ascii_case("open")) {
            Self::Open
        } else {
            Self::Closed
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessStatus {
    Active,
    Pending,
    Rejected,
}

impl AccessStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "pending" => Some(Self::Pending),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    Active,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramButton {
    pub text: String,
    pub callback_data: String,
}

#[derive(Debug)]
pub struct TelegramError(String);

impl fmt::Display for TelegramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TelegramError {}

pub type GatewayFuture<'a> = Pin<Box<dyn Future<Output = Result<(), TelegramError>> + Send + 'a>>;

/// Outbound Telegram calls. `getUpdates` is deliberately outside this trait:
/// only the optional long-polling worker needs it.
pub trait TelegramGateway: Send + Sync {
    fn send_message<'a>(
        &'a self,
        chat_id: i64,
        text: &'a str,
        buttons: &'a [TelegramButton],
    ) -> GatewayFuture<'a>;

    fn answer_callback<'a>(&'a self, callback_id: &'a str, text: &'a str) -> GatewayFuture<'a>;

    fn clear_inline_keyboard(&self, chat_id: i64, message_id: i64) -> GatewayFuture<'_>;
}

pub struct HttpTelegramGateway {
    client: reqwest::Client,
    token: String,
}

impl HttpTelegramGateway {
    /// # Errors
    /// The HTTP client cannot be initialized.
    pub fn new(token: String) -> Result<Self, TelegramError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(35))
            .build()
            .map_err(|error| TelegramError(error.to_string()))?;
        Ok(Self { client, token })
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.token)
    }

    async fn send_api(&self, method: &str, body: &serde_json::Value) -> Result<(), TelegramError> {
        let response = self
            .client
            .post(self.api_url(method))
            .json(body)
            .send()
            .await
            .map_err(|error| TelegramError(error.to_string()))?;
        if !response.status().is_success() {
            return Err(TelegramError(format!(
                "telegram HTTP {}",
                response.status()
            )));
        }
        Ok(())
    }

    /// Long-poll `getUpdates`; the caller owns the offset loop.
    ///
    /// # Errors
    /// Network or Telegram API errors.
    pub async fn get_updates(
        &self,
        offset: i64,
        timeout: u32,
    ) -> Result<Vec<serde_json::Value>, TelegramError> {
        let response = self
            .client
            .get(self.api_url("getUpdates"))
            .query(&[
                ("offset", offset.to_string()),
                ("timeout", timeout.to_string()),
            ])
            .send()
            .await
            .map_err(|error| TelegramError(error.to_string()))?;
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|error| TelegramError(error.to_string()))?;
        if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(TelegramError("telegram getUpdates failed".into()));
        }
        value
            .get("result")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .ok_or_else(|| TelegramError("telegram getUpdates had no result".into()))
    }
}

impl fmt::Debug for HttpTelegramGateway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpTelegramGateway")
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl TelegramGateway for HttpTelegramGateway {
    fn send_message<'a>(
        &'a self,
        chat_id: i64,
        text: &'a str,
        buttons: &'a [TelegramButton],
    ) -> GatewayFuture<'a> {
        let keyboard: Vec<serde_json::Value> = buttons
            .iter()
            .map(|button| {
                serde_json::json!({
                    "text": button.text,
                    "callback_data": button.callback_data,
                })
            })
            .collect();
        let mut body = serde_json::json!({ "chat_id": chat_id, "text": text });
        if !keyboard.is_empty() {
            body["reply_markup"] = serde_json::json!({ "inline_keyboard": [keyboard] });
        }
        Box::pin(async move { self.send_api("sendMessage", &body).await })
    }

    fn answer_callback<'a>(&'a self, callback_id: &'a str, text: &'a str) -> GatewayFuture<'a> {
        let body = serde_json::json!({ "callback_query_id": callback_id, "text": text });
        Box::pin(async move { self.send_api("answerCallbackQuery", &body).await })
    }

    fn clear_inline_keyboard(&self, chat_id: i64, message_id: i64) -> GatewayFuture<'_> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "reply_markup": { "inline_keyboard": [] },
        });
        Box::pin(async move { self.send_api("editMessageReplyMarkup", &body).await })
    }
}

#[derive(Debug, Default)]
pub struct NoopTelegramGateway;

impl TelegramGateway for NoopTelegramGateway {
    fn send_message<'a>(
        &'a self,
        _chat_id: i64,
        _text: &'a str,
        _buttons: &'a [TelegramButton],
    ) -> GatewayFuture<'a> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn answer_callback<'a>(&'a self, _callback_id: &'a str, _text: &'a str) -> GatewayFuture<'a> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn clear_inline_keyboard(&self, _chat_id: i64, _message_id: i64) -> GatewayFuture<'_> {
        Box::pin(std::future::ready(Ok(())))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentMessage {
    pub chat_id: i64,
    pub text: String,
    pub buttons: Vec<TelegramButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackAnswer {
    pub callback_id: String,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct MockTelegramGateway {
    messages: Mutex<Vec<SentMessage>>,
    callbacks: Mutex<Vec<CallbackAnswer>>,
    cleared: Mutex<Vec<(i64, i64)>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl MockTelegramGateway {
    pub fn messages(&self) -> Vec<SentMessage> {
        lock(&self.messages).clone()
    }

    pub fn callbacks(&self) -> Vec<CallbackAnswer> {
        lock(&self.callbacks).clone()
    }

    pub fn cleared_keyboards(&self) -> Vec<(i64, i64)> {
        lock(&self.cleared).clone()
    }
}

impl TelegramGateway for MockTelegramGateway {
    fn send_message<'a>(
        &'a self,
        chat_id: i64,
        text: &'a str,
        buttons: &'a [TelegramButton],
    ) -> GatewayFuture<'a> {
        lock(&self.messages).push(SentMessage {
            chat_id,
            text: text.to_owned(),
            buttons: buttons.to_vec(),
        });
        Box::pin(std::future::ready(Ok(())))
    }

    fn answer_callback<'a>(&'a self, callback_id: &'a str, text: &'a str) -> GatewayFuture<'a> {
        lock(&self.callbacks).push(CallbackAnswer {
            callback_id: callback_id.to_owned(),
            text: text.to_owned(),
        });
        Box::pin(std::future::ready(Ok(())))
    }

    fn clear_inline_keyboard(&self, chat_id: i64, message_id: i64) -> GatewayFuture<'_> {
        lock(&self.cleared).push((chat_id, message_id));
        Box::pin(std::future::ready(Ok(())))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn invite_hash(code: &str) -> String {
    sha256_hex(code.as_bytes())
}

/// Redeem one use of an invite inside the caller's transaction.
///
/// # Errors
/// Database errors.
pub async fn redeem_invite(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<bool, AppError> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(false);
    }
    let hash = invite_hash(code);
    let row: Option<(i32, i32)> = sqlx::query_as(
        "SELECT max_uses, used_count FROM invites \
         WHERE code_hash = $1 AND (expires_at IS NULL OR expires_at > now()) FOR UPDATE",
    )
    .bind(&hash)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((max_uses, used_count)) = row else {
        return Ok(false);
    };
    if used_count >= max_uses {
        return Ok(false);
    }
    sqlx::query("UPDATE invites SET used_count = used_count + 1 WHERE code_hash = $1")
        .bind(&hash)
        .execute(&mut **tx)
        .await?;
    Ok(true)
}

/// Apply the closed/open registration gate and optional invite.
///
/// # Errors
/// `403` for a rejected user, an existing pending user without a usable
/// invite, or a supplied invite that is invalid or exhausted; database errors.
pub async fn decide_access(
    tx: &mut Transaction<'_, Postgres>,
    mode: RegistrationMode,
    existing: Option<(Uuid, AccessStatus)>,
    invite_code: Option<&str>,
) -> Result<AccessDecision, AppError> {
    let invite = invite_code.map(str::trim).filter(|code| !code.is_empty());
    if let Some((user_id, status)) = existing {
        return match status {
            AccessStatus::Active => Ok(AccessDecision::Active),
            AccessStatus::Rejected => Err(AppError::AccessRejected),
            AccessStatus::Pending => match invite {
                Some(code) => {
                    if redeem_invite(tx, code).await? {
                        sqlx::query("UPDATE users SET access_status = 'active' WHERE id = $1")
                            .bind(user_id)
                            .execute(&mut **tx)
                            .await?;
                        Ok(AccessDecision::Active)
                    } else {
                        Err(AppError::InviteInvalid)
                    }
                }
                None => Err(AppError::PendingApproval),
            },
        };
    }

    match mode {
        RegistrationMode::Open => Ok(AccessDecision::Active),
        RegistrationMode::Closed => match invite {
            Some(code) => {
                if redeem_invite(tx, code).await? {
                    Ok(AccessDecision::Active)
                } else {
                    Err(AppError::InviteInvalid)
                }
            }
            None => Ok(AccessDecision::Pending),
        },
    }
}

#[derive(Debug, Serialize)]
pub struct CreatedInvite {
    pub id: Uuid,
    pub invite_code: String,
    pub max_uses: i32,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    #[serde(default)]
    pub note: Option<String>,
    pub max_uses: i32,
    #[serde(default)]
    pub expires_in_hours: Option<i64>,
}

fn check_invite_request(
    max_uses: i32,
    expires_in_hours: Option<i64>,
) -> Result<Option<i32>, AppError> {
    if max_uses <= 0 {
        return Err(AppError::BadRequest("max_uses must be positive".into()));
    }
    match expires_in_hours {
        Some(hours) if hours <= 0 => Err(AppError::BadRequest(
            "expires_in_hours must be positive".into(),
        )),
        Some(hours) => i32::try_from(hours)
            .map(Some)
            .map_err(|_| AppError::BadRequest("expires_in_hours is too large".into())),
        None => Ok(None),
    }
}

/// Store a new invite, keeping only the code's SHA-256 digest.
///
/// # Errors
/// A non-positive use count or expiry, or database errors.
pub async fn create_invite(
    pool: &PgPool,
    created_by: Option<Uuid>,
    note: Option<String>,
    max_uses: i32,
    expires_in_hours: Option<i64>,
) -> Result<CreatedInvite, AppError> {
    let hours = check_invite_request(max_uses, expires_in_hours)?;
    let id = Uuid::new_v4();
    let invite_code = Uuid::new_v4().simple().to_string();
    let (id, expires_at): (Uuid, Option<i64>) = sqlx::query_as(
        "INSERT INTO invites \
         (id, code_hash, created_by, note, max_uses, expires_at) \
         VALUES ($1, $2, $3, $4, $5, \
          CASE WHEN $6::integer IS NULL THEN NULL \
               ELSE now() + make_interval(hours => $6) END) \
         RETURNING id, EXTRACT(EPOCH FROM expires_at)::bigint",
    )
    .bind(id)
    .bind(invite_hash(&invite_code))
    .bind(created_by)
    .bind(note)
    .bind(max_uses)
    .bind(hours)
    .fetch_one(pool)
    .await?;

    Ok(CreatedInvite {
        id,
        invite_code,
        max_uses,
        expires_at,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoginMethodView {
    pub kind: String,
    pub identifier: String,
    pub verified_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingUser {
    pub user_id: Uuid,
    pub display_name: String,
    pub created_at: i64,
    pub login_methods: Vec<LoginMethodView>,
}

type PendingRow = (
    Uuid,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<i64>,
);

/// List pending users with their linked login methods.
///
/// # Errors
/// Database errors.
pub async fn list_pending(pool: &PgPool) -> Result<Vec<PendingUser>, AppError> {
    let rows: Vec<PendingRow> = sqlx::query_as(
        "SELECT u.id, u.display_name, EXTRACT(EPOCH FROM u.created_at)::bigint, \
             lm.kind, lm.identifier, EXTRACT(EPOCH FROM lm.verified_at)::bigint \
             FROM users u LEFT JOIN login_methods lm ON lm.user_id = u.id \
             WHERE u.access_status = 'pending' \
             ORDER BY u.created_at, u.id, lm.kind, lm.identifier",
    )
    .fetch_all(pool)
    .await?;

    let mut pending: Vec<PendingUser> = Vec::new();
    for (user_id, display_name, created_at, kind, identifier, verified_at) in rows {
        if pending.last().is_none_or(|user| user.user_id != user_id) {
            pending.push(PendingUser {
                user_id,
                display_name,
                created_at,
                login_methods: Vec::new(),
            });
        }
        if let (Some(kind), Some(identifier)) = (kind, identifier)
            && let Some(user) = pending.last_mut()
        {
            user.login_methods.push(LoginMethodView {
                kind,
                identifier,
                verified_at,
            });
        }
    }
    Ok(pending)
}

async fn notify_user(
    pool: &PgPool,
    user_id: Uuid,
    status: AccessStatus,
    gateway: &dyn TelegramGateway,
) {
    let identifier: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
        "SELECT identifier FROM login_methods \
         WHERE user_id = $1 AND kind = 'telegram' LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await;
    let Ok(Some(identifier)) = identifier else {
        return;
    };
    let Ok(chat_id) = identifier.parse::<i64>() else {
        return;
    };
    let text = if status == AccessStatus::Active {
        "Access approved."
    } else {
        "Access rejected."
    };
    let _ = gateway.send_message(chat_id, text, &[]).await;
}

/// Approve or reject a pending user and notify their linked Telegram account.
///
/// # Errors
/// The user is not pending, or database errors.
pub async fn decide_user(
    pool: &PgPool,
    user_id: Uuid,
    status: AccessStatus,
    gateway: &dyn TelegramGateway,
) -> Result<(), AppError> {
    if status == AccessStatus::Pending {
        return Err(AppError::BadRequest("cannot set pending here".into()));
    }
    let updated = sqlx::query(
        "UPDATE users SET access_status = $2 \
         WHERE id = $1 AND access_status = 'pending'",
    )
    .bind(user_id)
    .bind(status.as_str())
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("pending request not found"));
    }
    notify_user(pool, user_id, status, gateway).await;
    Ok(())
}

/// Send the admin approval request with the two plain inline buttons.
pub async fn notify_pending(
    gateway: &dyn TelegramGateway,
    admin_chat_id: Option<i64>,
    user_id: Uuid,
    label: &str,
) {
    let Some(chat_id) = admin_chat_id else {
        return;
    };
    let buttons = [
        TelegramButton {
            text: "Approve".into(),
            callback_data: format!("approve:{user_id}"),
        },
        TelegramButton {
            text: "Reject".into(),
            callback_data: format!("reject:{user_id}"),
        },
    ];
    let text = format!("New access request from {label}.\nUser id: {user_id}");
    let _ = gateway.send_message(chat_id, &text, &buttons).await;
}

#[derive(Debug, Clone, Copy)]
pub struct AdminUser {
    pub learner_id: Uuid,
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let learner = AuthenticatedLearner::from_request_parts(parts, state).await?;
        if learner.erased {
            return Err(AppError::Erased);
        }
        let is_admin: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_roles \
             WHERE user_id = $1 AND role = 'admin')",
        )
        .bind(learner.learner_id.0)
        .fetch_one(&state.pool)
        .await?;
        if is_admin {
            Ok(Self {
                learner_id: learner.learner_id.0,
            })
        } else {
            Err(AppError::Forbidden("admin required".into()))
        }
    }
}

/// # Errors
/// Invalid invite parameters or database errors.
pub async fn admin_create_invite(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<CreateInviteRequest>,
) -> Result<Json<CreatedInvite>, AppError> {
    let invite = create_invite(
        &state.pool,
        Some(admin.learner_id),
        payload.note,
        payload.max_uses,
        payload.expires_in_hours,
    )
    .await?;
    Ok(Json(invite))
}

/// # Errors
/// Database errors.
pub async fn admin_requests(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<PendingUser>>, AppError> {
    Ok(Json(list_pending(&state.pool).await?))
}

#[derive(Debug, Serialize)]
pub struct DecisionResponse {
    pub user_id: Uuid,
    pub access_status: &'static str,
}

async fn decide(
    state: &AppState,
    user_id: Uuid,
    status: AccessStatus,
) -> Result<Json<DecisionResponse>, AppError> {
    decide_user(&state.pool, user_id, status, &*state.telegram).await?;
    Ok(Json(DecisionResponse {
        user_id,
        access_status: status.as_str(),
    }))
}

/// # Errors
/// The user is not pending, or database errors.
pub async fn admin_approve(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<DecisionResponse>, AppError> {
    decide(&state, user_id, AccessStatus::Active).await
}

/// # Errors
/// The user is not pending, or database errors.
pub async fn admin_reject(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<DecisionResponse>, AppError> {
    decide(&state, user_id, AccessStatus::Rejected).await
}
