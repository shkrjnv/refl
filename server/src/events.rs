//! Writing to and reading from the event log (`docs/02-layer1-content.md`,
//! «Событие и лог»). Events are validated by the core before insertion and
//! stored as sent; reading upcasts them to the current schema version.

use std::fmt;

use refl_core::event::DecodeError;
use refl_core::value::Timestamp;
use refl_core::{Decoded, decode_event};
use serde_json::Value;
use sqlx::{PgExecutor, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inserted {
    /// Stored with this receipt number (`docs/13-sync-versions.md`): strictly
    /// increasing, not contiguous.
    New { server_seq: i64 },
    /// The same `event_id` is already in the log: sync is idempotent.
    Duplicate,
}

#[derive(Debug)]
pub enum InsertError {
    /// The core rejected the event.
    Invalid(DecodeError),
    /// Written by a newer core than the server's: never stored
    /// (`docs/13-sync-versions.md`, «Неизвестные события»).
    Unknown,
    /// Clients never set `server_ts`; the server assigns it on insert.
    ServerTsSet,
    /// A derived event with the same (learner, type, kind, key) exists.
    DerivedDuplicate,
    Db(sqlx::Error),
}

impl fmt::Display for InsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "invalid event: {e}"),
            Self::Unknown => f.write_str("unknown event type, client newer than server"),
            Self::ServerTsSet => f.write_str("server_ts must be empty when an event is written"),
            Self::DerivedDuplicate => f.write_str("derived event already recorded"),
            Self::Db(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for InsertError {}

/// Advisory lock key of one learner's log: the two halves of the UUID
/// folded into 64 bits. Only the event log takes advisory locks, so keys of
/// different learners can only meet by a 64-bit collision, which costs a
/// wait, never correctness.
fn learner_lock_key(learner: Uuid) -> i64 {
    let (hi, lo) = learner.as_u64_pair();
    (hi ^ lo).cast_signed()
}

/// Validates `json` with the core and appends it to the log with
/// `server_ts = now`. A repeated `event_id` is ignored; a second derived event
/// with the same uniqueness key and an event unknown to this core are rejected.
///
/// The transaction first takes the learner's advisory lock, held until it
/// ends. `server_seq` is assigned at insert, not at commit; with the lock,
/// one learner's transactions run one at a time, so they commit in
/// `server_seq` order and a sync cursor never skips an event
/// (`docs/13-sync-versions.md`, «Протокол синхронизации»). Every insert into
/// the log, from sync batches, derived events or `reclassified`, goes
/// through here.
///
/// # Errors
/// See [`InsertError`]. After an error the transaction must be rolled back.
pub async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    json: &str,
    now: Timestamp,
) -> Result<Inserted, InsertError> {
    let event = match decode_event(json).map_err(InsertError::Invalid)? {
        Decoded::Known(event) => event,
        Decoded::Unknown(_) => return Err(InsertError::Unknown),
    };
    if event.server_ts.is_some() {
        return Err(InsertError::ServerTsSet);
    }
    // Stored as sent: the payload and schema_version of the original JSON.
    let raw: Value =
        serde_json::from_str(json).map_err(|e| InsertError::Invalid(DecodeError::Json(e)))?;
    // decode_event accepted it, so it is present and within 1..=CURRENT.
    let schema_version = raw
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|v| i16::try_from(v).ok())
        .ok_or(InsertError::Invalid(DecodeError::MissingSchemaVersion))?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(learner_lock_key(event.learner_id.0))
        .execute(&mut **tx)
        .await
        .map_err(InsertError::Db)?;
    let derived = event.payload.derived_key();
    let result = sqlx::query_scalar::<_, i64>(
        "INSERT INTO events (event_id, learner_id, session_id, schema_version, client_ts, \
         server_ts, event_type, payload, derived_kind, derived_key) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (event_id) DO NOTHING RETURNING server_seq",
    )
    .bind(*event.event_id.as_uuid())
    .bind(event.learner_id.0)
    .bind(event.session_id.map(|s| s.0))
    .bind(schema_version)
    .bind(event.client_ts.0)
    .bind(now.0)
    .bind(event.payload.event_type())
    .bind(&raw["payload"])
    .bind(derived.as_ref().map(|d| d.kind))
    .bind(derived.as_ref().map(|d| d.key.as_str()))
    .fetch_optional(&mut **tx)
    .await;
    match result {
        Ok(Some(server_seq)) => Ok(Inserted::New { server_seq }),
        Ok(None) => Ok(Inserted::Duplicate),
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("events_derived_unique") => {
            Err(InsertError::DerivedDuplicate)
        }
        Err(e) => Err(InsertError::Db(e)),
    }
}

#[derive(Debug)]
pub enum ReadError {
    Decode(DecodeError),
    Db(sqlx::Error),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "stored event does not decode: {e}"),
            Self::Db(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// A learner's events in fold order (`event_id`), upcast to the current
/// schema. Events unknown to this core (stored by a newer server before a
/// rollback) come back as [`Decoded::Unknown`] for the fold to skip.
///
/// # Errors
/// Database errors, or a stored event the core cannot decode.
pub async fn learner_events(
    db: impl PgExecutor<'_>,
    learner: Uuid,
) -> Result<Vec<Decoded>, ReadError> {
    let rows: Vec<(Uuid, Option<Uuid>, i16, i64, i64, Value)> = sqlx::query_as(
        "SELECT event_id, session_id, schema_version, client_ts, server_ts, payload \
         FROM events WHERE learner_id = $1 ORDER BY event_id",
    )
    .bind(learner)
    .fetch_all(db)
    .await
    .map_err(ReadError::Db)?;
    rows.into_iter()
        .map(
            |(event_id, session_id, schema_version, client_ts, server_ts, payload)| {
                let json = serde_json::json!({
                    "schema_version": schema_version,
                    "event_id": event_id,
                    "learner_id": learner,
                    "session_id": session_id,
                    "client_ts": client_ts,
                    "server_ts": server_ts,
                    "payload": payload,
                });
                decode_event(&json.to_string()).map_err(ReadError::Decode)
            },
        )
        .collect()
}
