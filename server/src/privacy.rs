//! Self-service profile, export and erasure (`docs/14-privacy-license.md`).
//! Export returns only the authenticated learner's own data; erasure keeps a
//! deletion-audit row with an opaque id and time and removes everything else.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::State;
use refl_core::ids::LearnerId;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthenticatedLearner;
use crate::error::AppError;
use crate::routes::AppState;

#[derive(Debug, Serialize)]
pub struct PrivacyView {
    pub profile_visibility: String,
    pub friend_requests_from: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ChatPolicy {
    /// Direct messages are never available to a minor (docs/01, docs/14).
    pub personal_messages: bool,
    /// Team chat remains available inside a team.
    pub team_chat: bool,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub learner_id: Uuid,
    pub public_code: String,
    pub display_name: String,
    pub interface_lang: String,
    pub minor: bool,
    pub access_status: String,
    pub created_at: i64,
    pub privacy: PrivacyView,
    pub chat: ChatPolicy,
}

#[derive(Debug, Serialize)]
pub struct LoginMethodView {
    pub kind: String,
    pub identifier: String,
    pub verified_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct MentorLinkView {
    pub mentor_id: Uuid,
    pub learner_id: Uuid,
    pub guardian: bool,
    pub status: String,
    pub created_at: i64,
    pub accepted_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub format: &'static str,
    pub exported_at: i64,
    pub profile: ProfileResponse,
    pub login_methods: Vec<LoginMethodView>,
    pub mentor_links: Vec<MentorLinkView>,
    /// Events exactly as stored, in fold order, with their receipt number.
    pub events: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct EraseResponse {
    pub erased: bool,
    pub status: &'static str,
    pub erased_at: Option<i64>,
}

type ProfileRow = (
    Uuid,
    String,
    String,
    String,
    bool,
    String,
    i64,
    String,
    String,
    i64,
);

type EventRow = (i64, Uuid, Option<Uuid>, i16, i64, i64, Value);

fn now_millis() -> Result<i64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::internal)?;
    i64::try_from(elapsed.as_millis()).map_err(AppError::internal)
}

async fn profile_for(pool: &PgPool, learner: LearnerId) -> Result<ProfileResponse, AppError> {
    let row: Option<ProfileRow> = sqlx::query_as(
        "SELECT u.id, u.public_code, u.display_name, u.interface_lang, u.minor, \
         u.access_status, (extract(epoch FROM u.created_at) * 1000)::bigint, \
         p.profile_visibility, p.friend_requests_from, \
         (extract(epoch FROM p.updated_at) * 1000)::bigint \
         FROM users u JOIN privacy_settings p ON p.user_id = u.id WHERE u.id = $1",
    )
    .bind(learner.0)
    .fetch_optional(pool)
    .await?;
    let Some((
        learner_id,
        public_code,
        display_name,
        interface_lang,
        minor,
        access_status,
        created_at,
        profile_visibility,
        friend_requests_from,
        privacy_updated_at,
    )) = row
    else {
        return Err(AppError::Erased);
    };
    Ok(ProfileResponse {
        learner_id,
        public_code,
        display_name,
        interface_lang,
        minor,
        access_status,
        created_at,
        privacy: PrivacyView {
            profile_visibility,
            friend_requests_from,
            updated_at: privacy_updated_at,
        },
        chat: ChatPolicy {
            personal_messages: !minor,
            team_chat: true,
        },
    })
}

/// The authenticated learner's own profile, including the derived chat policy.
///
/// # Errors
/// `403` for an erased account; database errors become `500`.
pub async fn profile(
    State(state): State<AppState>,
    auth: AuthenticatedLearner,
) -> Result<Json<ProfileResponse>, AppError> {
    if auth.erased {
        return Err(AppError::Erased);
    }
    Ok(Json(profile_for(&state.pool, auth.learner_id).await?))
}

/// A JSON copy of the learner's profile, login identifiers, mentor links and
/// event log. Auth sessions, token hashes and internal server logs are not
/// included.
///
/// # Errors
/// `403` for an erased account; database errors become `500`.
pub async fn export(
    State(state): State<AppState>,
    auth: AuthenticatedLearner,
) -> Result<Json<ExportResponse>, AppError> {
    if auth.erased {
        return Err(AppError::Erased);
    }
    let profile = profile_for(&state.pool, auth.learner_id).await?;

    let methods: Vec<(String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT kind, identifier, (extract(epoch FROM verified_at) * 1000)::bigint, \
         (extract(epoch FROM created_at) * 1000)::bigint \
         FROM login_methods WHERE user_id = $1 ORDER BY created_at, kind",
    )
    .bind(auth.learner_id.0)
    .fetch_all(&state.pool)
    .await?;
    let login_methods = methods
        .into_iter()
        .map(
            |(kind, identifier, verified_at, created_at)| LoginMethodView {
                kind,
                identifier,
                verified_at,
                created_at,
            },
        )
        .collect();

    let links: Vec<(Uuid, Uuid, bool, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT mentor_id, learner_id, guardian, status, \
         (extract(epoch FROM created_at) * 1000)::bigint, \
         (extract(epoch FROM accepted_at) * 1000)::bigint \
         FROM mentor_links WHERE mentor_id = $1 OR learner_id = $1 \
         ORDER BY created_at, mentor_id",
    )
    .bind(auth.learner_id.0)
    .fetch_all(&state.pool)
    .await?;
    let mentor_links = links
        .into_iter()
        .map(
            |(mentor_id, learner_id, guardian, status, created_at, accepted_at)| MentorLinkView {
                mentor_id,
                learner_id,
                guardian,
                status,
                created_at,
                accepted_at,
            },
        )
        .collect();

    let rows: Vec<EventRow> = sqlx::query_as(
        "SELECT server_seq, event_id, session_id, schema_version, client_ts, server_ts, payload \
         FROM events WHERE learner_id = $1 ORDER BY event_id",
    )
    .bind(auth.learner_id.0)
    .fetch_all(&state.pool)
    .await?;
    let events = rows
        .into_iter()
        .map(
            |(server_seq, event_id, session_id, schema_version, client_ts, server_ts, payload)| {
                json!({
                    "server_seq": server_seq,
                    "schema_version": schema_version,
                    "event_id": event_id,
                    "learner_id": auth.learner_id.0,
                    "session_id": session_id,
                    "client_ts": client_ts,
                    "server_ts": server_ts,
                    "payload": payload,
                })
            },
        )
        .collect();

    Ok(Json(ExportResponse {
        format: "refl-export-v1",
        exported_at: now_millis()?,
        profile,
        login_methods,
        mentor_links,
        events,
    }))
}

/// Erases the authenticated account: the event log and snapshots through
/// `erase_learner`, then login identifiers, tokens, teams and the user row.
/// The `erasures` audit row keeps only the learner id and deletion time.
///
/// # Errors
/// `403` for an erased account; database errors become `500`.
pub async fn erase(
    State(state): State<AppState>,
    auth: AuthenticatedLearner,
) -> Result<Json<EraseResponse>, AppError> {
    if auth.erased {
        return Err(AppError::Erased);
    }
    let erased_at: Option<i64> =
        sqlx::query_scalar("SELECT (extract(epoch FROM erase_account($1)) * 1000)::bigint")
            .bind(auth.learner_id.0)
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(EraseResponse {
        erased: true,
        status: "erased",
        erased_at,
    }))
}
