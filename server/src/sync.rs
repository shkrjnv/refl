//! `POST /v1/sync`: one batch up, one page down (`docs/13-sync-versions.md`).

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use refl_core::event::{Decoded, decode_event};
use refl_core::ids::{EventId, LearnerId};
use refl_core::value::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::AuthenticatedLearner;
use crate::content;
use crate::error::AppError;
use crate::events::{self, InsertError, Inserted, StoredEvent};
use crate::routes::AppState;

const MAX_EVENTS: usize = 500;
const PULL_LIMIT: i64 = 501;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub learner_id: LearnerId,
    pub core_version: String,
    pub content_version: String,
    #[serde(default)]
    pub cursor: i64,
    #[serde(default)]
    pub events: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RejectedEvent {
    pub event_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub accepted: Vec<Uuid>,
    pub rejected: Vec<RejectedEvent>,
    pub events: Vec<Value>,
    pub cursor: i64,
    pub more: bool,
    pub server_time: i64,
    pub min_core_version: String,
    pub latest_content_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventHeader {
    event_id: EventId,
    learner_id: LearnerId,
    client_ts: Timestamp,
}

struct IncomingEvent {
    header: EventHeader,
    raw: String,
}

fn parse_incoming(value: &Value) -> Result<IncomingEvent, AppError> {
    let header: EventHeader = serde_json::from_value(value.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid event envelope: {error}")))?;
    let raw = serde_json::to_string(value)
        .map_err(|error| AppError::BadRequest(format!("invalid event JSON: {error}")))?;
    Ok(IncomingEvent { header, raw })
}

fn now_millis() -> Result<Timestamp, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::internal)?;
    let millis = i64::try_from(elapsed.as_millis()).map_err(AppError::internal)?;
    Timestamp::try_from(millis).map_err(AppError::internal)
}

fn version_parts(version: &str) -> Option<Vec<u64>> {
    version
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn is_older(core_version: &str, min_core_version: &str) -> bool {
    match (version_parts(core_version), version_parts(min_core_version)) {
        (Some(mut core), Some(mut min)) => {
            let width = core.len().max(min.len());
            core.resize(width, 0);
            min.resize(width, 0);
            core < min
        }
        _ => core_version < min_core_version,
    }
}

fn page(rows: Vec<StoredEvent>, cursor: i64) -> (Vec<Value>, bool, i64) {
    let more = rows.len() > MAX_EVENTS;
    let mut rows = rows;
    if more {
        rows.truncate(MAX_EVENTS);
    }
    let cursor = rows.last().map_or(cursor, |row| row.server_seq);
    (
        rows.into_iter().map(|row| row.event).collect(),
        more,
        cursor,
    )
}

async fn push_batch(
    tx: &mut Transaction<'_, Postgres>,
    incoming: Vec<IncomingEvent>,
    learner: LearnerId,
    now: Timestamp,
    future_limit: i64,
) -> Result<(Vec<Uuid>, Vec<RejectedEvent>), AppError> {
    let mut accepted = Vec::with_capacity(incoming.len());
    let mut rejected = Vec::new();
    for event in incoming {
        let event_id = Uuid::from(event.header.event_id);
        if event.header.learner_id != learner {
            rejected.push(RejectedEvent {
                event_id,
                reason: "learner_mismatch".into(),
            });
            continue;
        }
        if event.header.client_ts.0 > future_limit {
            rejected.push(RejectedEvent {
                event_id,
                reason: "future_event".into(),
            });
            continue;
        }
        match decode_event(&event.raw) {
            Ok(Decoded::Known(_)) => {}
            Ok(Decoded::Unknown(_)) => {
                rejected.push(RejectedEvent {
                    event_id,
                    reason: "unknown_event".into(),
                });
                continue;
            }
            Err(error) => {
                rejected.push(RejectedEvent {
                    event_id,
                    reason: format!("invalid_event: {error}"),
                });
                continue;
            }
        }

        sqlx::query("SAVEPOINT sync_event")
            .execute(&mut **tx)
            .await?;
        match events::insert_event(&mut *tx, &event.raw, now).await {
            Ok(Inserted::New { .. } | Inserted::Duplicate) => {
                accepted.push(event_id);
            }
            Err(InsertError::Invalid(error)) => rejected.push(RejectedEvent {
                event_id,
                reason: format!("invalid_event: {error}"),
            }),
            Err(InsertError::Unknown) => rejected.push(RejectedEvent {
                event_id,
                reason: "unknown_event".into(),
            }),
            Err(InsertError::ServerTsSet) => rejected.push(RejectedEvent {
                event_id,
                reason: "server_ts_set".into(),
            }),
            Err(InsertError::DerivedDuplicate) => {
                sqlx::query("ROLLBACK TO SAVEPOINT sync_event")
                    .execute(&mut **tx)
                    .await?;
                rejected.push(RejectedEvent {
                    event_id,
                    reason: "derived_duplicate".into(),
                });
            }
            Err(InsertError::Db(error)) => return Err(AppError::from(error)),
        }
        sqlx::query("RELEASE SAVEPOINT sync_event")
            .execute(&mut **tx)
            .await?;
    }
    Ok((accepted, rejected))
}

fn erased_response() -> Response {
    Json(json!({ "erased": true, "status": "erased" })).into_response()
}

/// Push one batch and return the next page of the learner's log.
///
/// # Errors
/// `400` for a batch over 500 events or an invalid event envelope; `403` when
/// the request learner differs from the authenticated one; `426` when the
/// client core is below `min_core_version`; database errors become `500`. An
/// erased learner receives `200` with `{ "erased": true }`.
pub async fn sync(
    State(state): State<AppState>,
    auth: AuthenticatedLearner,
    Json(request): Json<SyncRequest>,
) -> Result<Response, AppError> {
    if auth.erased {
        return Ok(erased_response());
    }
    if request.events.len() > MAX_EVENTS {
        return Err(AppError::BadRequest(format!(
            "at most {MAX_EVENTS} events per batch"
        )));
    }
    if request.learner_id != auth.learner_id {
        return Err(AppError::forbidden(
            "learner_id does not match the authenticated learner",
        ));
    }
    if is_older(&request.core_version, &state.config.min_core_version) {
        return Err(AppError::UpgradeRequired {
            min_core_version: state.config.min_core_version.clone(),
        });
    }

    let now = now_millis()?;
    let future_limit = now.0.saturating_add(DAY_MS);
    let mut incoming = request
        .events
        .iter()
        .map(parse_incoming)
        .collect::<Result<Vec<_>, _>>()?;
    incoming.sort_by_key(|event| event.header.event_id);

    let mut tx = state.pool.begin().await?;
    let (accepted, rejected) =
        push_batch(&mut tx, incoming, request.learner_id, now, future_limit).await?;
    if let Some(session_id) = auth.session_id {
        sqlx::query(
            "UPDATE auth_sessions SET refresh_expires_at = now() + interval '90 days', \
             updated_at = now() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let rows = events::learner_events_since(
        &state.pool,
        request.learner_id.0,
        request.cursor,
        PULL_LIMIT,
    )
    .await?;
    let (events, more, cursor) = page(rows, request.cursor);
    let latest_content_version = content::latest_content_version(&state.pool).await?;

    Ok(Json(SyncResponse {
        accepted,
        rejected,
        events,
        cursor,
        more,
        server_time: now.0,
        min_core_version: state.config.min_core_version,
        latest_content_version,
    })
    .into_response())
}
