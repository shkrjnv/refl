//! The event log: the single source of truth about a learner
//! (`docs/02-layer1-content.md`, «Событие и лог»; `docs/01-architecture.md`).
//!
//! Events are append-only. Every serialized event carries `schema_version`;
//! older versions are lifted to the current one by upcasters before
//! deserialization. Corrections are new events (`reclassified`), never edits.

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::ExplanationDepth;
use crate::ids::{
    ContentVersion, EventId, ExplanationId, ItemRef, LearnerId, LessonId, MisconceptionId,
    SessionId,
};
use crate::value::Timestamp;

/// Schema version written into every new event.
pub const CURRENT_SCHEMA_VERSION: u16 = 1;

/// An upcaster lifts the JSON of an event from version `n` to `n + 1`.
type Upcaster = fn(Value) -> Result<Value, DecodeError>;

/// `UPCASTERS[i]` lifts version `i + 1` to `i + 2`. Its length is always
/// `CURRENT_SCHEMA_VERSION - 1`; a schema change appends one function here and
/// bumps the constant. Existing upcasters are never edited.
const UPCASTERS: &[Upcaster] = &[];

const _: () = assert!(UPCASTERS.len() + 1 == CURRENT_SCHEMA_VERSION as usize);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Event {
    pub schema_version: u16,
    pub event_id: EventId,
    pub learner_id: LearnerId,
    pub session_id: Option<SessionId>,
    /// Time on the device.
    pub client_ts: Timestamp,
    /// Time of receipt; `None` until the server has accepted the event.
    pub server_ts: Option<Timestamp>,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    Answer(AnswerEvent),
    Reclassified(ReclassifiedEvent),
    LessonStarted {
        lesson: LessonId,
    },
    LessonFinished {
        lesson: LessonId,
    },
    ExplanationViewed {
        explanation: ExplanationId,
        depth: ExplanationDepth,
        seconds: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum AnswerContext {
    Lesson,
    Review,
    Placement,
    Practice,
    Mock,
}

/// Raw response exactly as given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Response {
    /// Index of the chosen option.
    Option(u16),
    /// Typed text, unnormalized.
    Text(String),
}

/// Always one of four verdicts (`docs/02-layer1-content.md`, «Проверка ответов на ввод»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Correct,
    Typo,
    Wrong,
    Unrecognized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Sure,
    Unsure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AnswerEvent {
    pub item: ItemRef,
    pub slot: u16,
    pub context: AnswerContext,
    pub response: Response,
    /// Check result at answer time; later corrections come as `reclassified`.
    pub verdict: Verdict,
    pub misconception: Option<MisconceptionId>,
    pub latency_ms: u32,
    /// The app went to background during the item: latency is not used
    /// (`docs/03-layer2-learner-model.md`, «Автоматизм»).
    pub backgrounded: bool,
    pub confidence: Option<Confidence>,
    pub hints: u8,
    pub attempt: u8,
}

/// Emitted by the server-side recheck after a content build that changed
/// accepted answers or error patterns. Fold applies it; it never re-checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReclassifiedEvent {
    pub original: EventId,
    pub verdict: Verdict,
    pub misconception: Option<MisconceptionId>,
    pub content_version: ContentVersion,
}

#[derive(Debug)]
pub enum DecodeError {
    Json(serde_json::Error),
    MissingSchemaVersion,
    UnknownSchemaVersion(u64),
    /// Written by a newer core; must not be guessed at.
    FutureSchemaVersion(u64),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "invalid event JSON: {e}"),
            Self::MissingSchemaVersion => f.write_str("event has no schema_version"),
            Self::UnknownSchemaVersion(v) => write!(f, "unknown schema_version {v}"),
            Self::FutureSchemaVersion(v) => write!(
                f,
                "schema_version {v} is newer than supported {CURRENT_SCHEMA_VERSION}"
            ),
        }
    }
}

impl core::error::Error for DecodeError {}

impl From<serde_json::Error> for DecodeError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Serializes an event. Output is deterministic: same event, same bytes.
///
/// # Panics
/// If `event.schema_version` is not [`CURRENT_SCHEMA_VERSION`]: only current
/// events are ever written.
pub fn encode_event(event: &Event) -> String {
    assert_eq!(
        event.schema_version, CURRENT_SCHEMA_VERSION,
        "only events of the current schema version are written"
    );
    serde_json::to_string(event).expect("event serialization is infallible")
}

/// Parses an event of any supported schema version, upcasting it to the current one.
///
/// # Errors
/// Invalid JSON or field values, or a missing, unknown or future `schema_version`.
pub fn decode_event(json: &str) -> Result<Event, DecodeError> {
    let mut value: Value = serde_json::from_str(json)?;
    let raw = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(DecodeError::MissingSchemaVersion)?;
    let version = match u16::try_from(raw) {
        Ok(0) => return Err(DecodeError::UnknownSchemaVersion(raw)),
        Ok(v) if v <= CURRENT_SCHEMA_VERSION => v,
        _ => return Err(DecodeError::FutureSchemaVersion(raw)),
    };
    for upcast in &UPCASTERS[usize::from(version - 1)..] {
        value = upcast(value)?;
    }
    Ok(serde_json::from_value(value)?)
}
