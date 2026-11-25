//! The event log: the single source of truth about a learner
//! (`docs/02-layer1-content.md`, «Событие и лог»; `docs/01-architecture.md`).
//!
//! Events are append-only. Every serialized event carries `schema_version`;
//! older versions are lifted to the current one by upcasters before
//! deserialization. Corrections are new events (`reclassified`), never edits.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::ExplanationDepth;
use crate::ids::{
    AchievementId, ContentVersion, EventId, ExplanationId, ItemRef, KcId, LearnerId, LessonId,
    MisconceptionId, PackageKey, SessionId, UserId,
};
use crate::unlock::{
    AchievementTier, CosmeticSlot, Milestone, ShowcaseKeys, ShowcaseKind, UnlockKind, UnlockSource,
};
use crate::value::{Finite, MAX_SAFE_INTEGER, Seed, Timestamp, UnitInterval, ValueError};

/// Schema version written into every new event.
///
/// When to bump it (`docs/13-sync-versions.md`, «Изменение схемы событий»):
/// an older core must read an event correctly or see it as unknown, never
/// misread it. Only two additions keep the version, because an older core
/// detects them by tag: a new [`EventPayload`] variant (listed in
/// [`EVENT_TYPES`]) and a new [`Response`] variant (in [`RESPONSE_KINDS`]).
/// Anything else bumps it and appends an upcaster: a new enum value inside an
/// existing payload, a new field (required or not), a changed meaning. Older
/// cores then see such events as unknown by version, while the server and
/// newer cores lift old versions through `UPCASTERS`.
pub const CURRENT_SCHEMA_VERSION: u16 = 2;

/// An upcaster lifts the JSON of an event from version `n` to `n + 1`.
type Upcaster = fn(Value) -> Result<Value, DecodeError>;

/// `UPCASTERS[i]` lifts version `i + 1` to `i + 2`. Its length is always
/// `CURRENT_SCHEMA_VERSION - 1`; a schema change appends one function here and
/// bumps the constant. Existing upcasters are never edited.
const UPCASTERS: &[Upcaster] = &[plan_issued_end];

/// 1 → 2: `plan_issued` gains `end`, the reason the session ended
/// (`docs/02-layer1-content.md`); an event of version 1 has none. Other
/// payloads are unchanged.
#[allow(clippy::unnecessary_wraps)] // the `Upcaster` signature
fn plan_issued_end(mut value: Value) -> Result<Value, DecodeError> {
    if let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut)
        && payload.get("type").and_then(Value::as_str) == Some("plan_issued")
        && !payload.contains_key("end")
    {
        payload.insert(String::from("end"), Value::Null);
    }
    if let Some(event) = value.as_object_mut() {
        event.insert(String::from("schema_version"), Value::from(2u16));
    }
    Ok(value)
}

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
    /// Derived by the server; unique per (learner, milestone).
    MilestoneReached {
        milestone: Milestone,
    },
    /// Derived by the server; unique per (learner, achievement, tier).
    AchievementEarned {
        achievement: AchievementId,
        #[serde(default)]
        tier: AchievementTier,
    },
    /// Whether an earned achievement is shown in the public profile.
    AchievementVisibilitySet {
        achievement: AchievementId,
        visible: bool,
    },
    /// Unique per (learner, kind, key) unless the source is an admin.
    UnlockGranted {
        kind: UnlockKind,
        key: PackageKey,
        source: UnlockSource,
    },
    /// Only admin unlocks can be revoked.
    UnlockRevoked {
        kind: UnlockKind,
        key: PackageKey,
        by: UserId,
    },
    ThemeSelected {
        theme: PackageKey,
    },
    CompanionSelected {
        companion: PackageKey,
    },
    /// Only an unlocked or base cosmetic can be selected.
    CosmeticSelected {
        slot: CosmeticSlot,
        key: PackageKey,
    },
    /// What the public profile shows, e.g. up to five topic cards.
    ShowcaseSet {
        kind: ShowcaseKind,
        keys: ShowcaseKeys,
    },
    /// A planner decision; the learner-model fold skips it
    /// (`docs/11-quality.md`, «Лог решений»).
    PlanIssued(PlanIssuedEvent),
    /// A mentor corrected a decision of the system (`docs/11-quality.md`, «К6»).
    MentorOverride(MentorOverrideEvent),
    /// Criterion scores of a written answer (stage 5).
    WritingScored(WritingScoredEvent),
}

/// Uniqueness key of an event the server derives from the log: the database
/// holds at most one event per (learner, event type, kind, key), so
/// recomputing derived events is idempotent
/// (`docs/07-themes-achievements-frames.md`, «Модель разблокировок»).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedKey {
    pub kind: &'static str,
    /// For an achievement, `id:tier` (e.g. `streak:3`): each tier is its own event.
    pub key: String,
}

impl EventPayload {
    /// The `type` tag of the serialized payload.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Answer(_) => "answer",
            Self::Reclassified(_) => "reclassified",
            Self::LessonStarted { .. } => "lesson_started",
            Self::LessonFinished { .. } => "lesson_finished",
            Self::ExplanationViewed { .. } => "explanation_viewed",
            Self::MilestoneReached { .. } => "milestone_reached",
            Self::AchievementEarned { .. } => "achievement_earned",
            Self::AchievementVisibilitySet { .. } => "achievement_visibility_set",
            Self::UnlockGranted { .. } => "unlock_granted",
            Self::UnlockRevoked { .. } => "unlock_revoked",
            Self::ThemeSelected { .. } => "theme_selected",
            Self::CompanionSelected { .. } => "companion_selected",
            Self::CosmeticSelected { .. } => "cosmetic_selected",
            Self::ShowcaseSet { .. } => "showcase_set",
            Self::PlanIssued(_) => "plan_issued",
            Self::MentorOverride(_) => "mentor_override",
            Self::WritingScored(_) => "writing_scored",
        }
    }

    /// `Some` for derived events: milestones, achievements, and unlocks whose
    /// source is a milestone or an achievement. Admin unlocks and revokes are
    /// not unique, so an admin can grant again after a revoke.
    pub fn derived_key(&self) -> Option<DerivedKey> {
        match self {
            Self::MilestoneReached { milestone } => {
                let (kind, key) = milestone.kind_key();
                Some(DerivedKey {
                    kind,
                    key: key.into(),
                })
            }
            Self::AchievementEarned { achievement, tier } => Some(DerivedKey {
                kind: "achievement",
                key: alloc::format!("{achievement}:{}", tier.get()),
            }),
            Self::UnlockGranted {
                kind,
                key,
                source: UnlockSource::Milestone { .. } | UnlockSource::Achievement { .. },
            } => Some(DerivedKey {
                kind: kind.as_str(),
                key: key.as_str().into(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum AnswerContext {
    Lesson,
    Review,
    Placement,
    Practice,
    Mock,
    /// Audit item, chosen not by value (`docs/11-quality.md`, «К4»).
    Audit,
    /// Check after a break (`docs/04-placement-planner.md`); like `Audit`,
    /// chosen not by value, so it can calibrate forgetting.
    ReturnCheck,
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
    /// Steps shown already solved (faded worked example); 0: none
    /// (`docs/10-subjects.md`, «Многошаговые решения»).
    #[serde(default)]
    pub scaffold: u8,
    /// The step was solved from the revealed correct result of a step it
    /// depends on.
    #[serde(default)]
    pub revealed_deps: bool,
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

/// What a planner decision covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum PlanScope {
    /// The plan of a whole session.
    Session,
    /// The choice of the next item.
    NextItem,
}

/// An action the planner considered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanAction {
    Learn {
        kc: KcId,
    },
    Review {
        kc: KcId,
    },
    Item {
        item: ItemRef,
        slot: u16,
    },
    /// Suggest a break (`docs/04-placement-planner.md`, «Адаптация на ходу»).
    Break,
}

/// One multiplier of a candidate's value, e.g. `("forgetting", 0.42)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanFactor {
    pub name: String,
    pub value: Finite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanCandidate {
    pub action: PlanAction,
    pub value_per_minute: Finite,
    /// The value broken down into its multipliers.
    pub factors: Vec<PlanFactor>,
}

/// Every plan and every next-item choice, so that decisions can be checked
/// and replayed (`docs/11-quality.md`, «Лог решений», «Микрорандомизация»).
/// Decoding rejects a `chosen` that indexes no candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "PlanIssuedUnchecked")]
pub struct PlanIssuedEvent {
    pub planner_version: String,
    pub model_version: String,
    pub scope: PlanScope,
    /// Top candidates by value per minute.
    pub candidates: Vec<PlanCandidate>,
    /// Index of the chosen candidate in `candidates`.
    pub chosen: u16,
    /// Probability with which `chosen` was picked.
    pub propensity: UnitInterval,
    /// Seed of the core PRNG that made the choice.
    pub seed: Seed,
    /// Codes of plan validator violations (`docs/11-quality.md`, «К3»); empty
    /// when the plan is valid.
    pub violations: Vec<String>,
    /// The dominant factor of the chosen candidate: «Почему это задание»
    /// picks its template by it (`docs/04-placement-planner.md`).
    pub dominant: String,
    /// The adaptation rule that changed the plan; `None` for a plan built
    /// afresh and for a next-item choice.
    pub trigger: Option<String>,
    /// Why the session ended, when no step is left after this decision
    /// (`docs/04-placement-planner.md`, «Конец сессии»); schema version 2.
    pub end: Option<String>,
}

/// [`PlanIssuedEvent`] as it arrives, before the `chosen` check.
#[derive(Deserialize)]
struct PlanIssuedUnchecked {
    planner_version: String,
    model_version: String,
    scope: PlanScope,
    candidates: Vec<PlanCandidate>,
    chosen: u16,
    propensity: UnitInterval,
    seed: Seed,
    violations: Vec<String>,
    dominant: String,
    trigger: Option<String>,
    end: Option<String>,
}

impl TryFrom<PlanIssuedUnchecked> for PlanIssuedEvent {
    type Error = ValueError;
    fn try_from(p: PlanIssuedUnchecked) -> Result<Self, ValueError> {
        if usize::from(p.chosen) >= p.candidates.len() {
            return Err(ValueError::ChosenOutOfRange);
        }
        Ok(Self {
            planner_version: p.planner_version,
            model_version: p.model_version,
            scope: p.scope,
            candidates: p.candidates,
            chosen: p.chosen,
            propensity: p.propensity,
            seed: p.seed,
            violations: p.violations,
            dominant: p.dominant,
            trigger: p.trigger,
            end: p.end,
        })
    }
}

/// What a mentor changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OverrideKind {
    /// Changed the plan, e.g. "no new topic today".
    PlanChange {
        /// The `plan_issued` event overridden.
        plan: EventId,
    },
    /// Marked a topic as known: a low-weight observation that later answers check.
    MarkKnown {
        kc: KcId,
        /// The model's mastery probability `M_k` at that moment
        /// (`docs/04-placement-planner.md`).
        model_mastery: UnitInterval,
    },
    /// Disputed the verdict of this answer event.
    DisputeVerdict { event: EventId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum OverrideReason {
    KnowsElsewhere,
    TooHard,
    TooEasy,
    WrongVerdict,
    Other,
}

/// The share and reasons of overrides show where the system is wrong; an
/// override is not ground truth either.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MentorOverrideEvent {
    pub mentor: UserId,
    pub kind: OverrideKind,
    pub reason: OverrideReason,
    pub note: Option<String>,
}

/// An IELTS band in half-band steps: band × 2, `0..=18` (band 6.5 is 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u8", into = "u8")]
pub struct HalfBands(u8);

impl HalfBands {
    pub fn new(v: u8) -> Option<Self> {
        (v <= 18).then_some(Self(v))
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for HalfBands {
    type Error = ValueError;
    fn try_from(v: u8) -> Result<Self, ValueError> {
        Self::new(v).ok_or(ValueError::HalfBandsOutOfRange)
    }
}

impl From<HalfBands> for u8 {
    fn from(v: HalfBands) -> u8 {
        v.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CriterionScore {
    pub criterion: String,
    pub half_bands: HalfBands,
}

/// A person's scores for a written answer by IELTS criteria; their gap to
/// the analyzer is its metric (`docs/11-quality.md`, «К6»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WritingScoredEvent {
    pub item: ItemRef,
    pub slot: u16,
    pub scorer: UserId,
    pub scores: Vec<CriterionScore>,
    /// The scorer did not see the analyzer's scores.
    pub blind: bool,
}

/// `type` tags of every [`EventPayload`] variant this core knows. An event
/// with any other tag was written by a newer core and decodes as
/// [`Decoded::Unknown`]. A unit test checks this list against the enum.
pub const EVENT_TYPES: &[&str] = &[
    "answer",
    "reclassified",
    "lesson_started",
    "lesson_finished",
    "explanation_viewed",
    "milestone_reached",
    "achievement_earned",
    "achievement_visibility_set",
    "unlock_granted",
    "unlock_revoked",
    "theme_selected",
    "companion_selected",
    "cosmetic_selected",
    "showcase_set",
    "plan_issued",
    "mentor_override",
    "writing_scored",
];

/// `kind` tags of every [`Response`] variant this core knows; checked like
/// [`EVENT_TYPES`].
pub const RESPONSE_KINDS: &[&str] = &["option", "text"];

/// Result of [`decode_event`].
///
/// The fold must skip [`Decoded::Unknown`] and mark the learner state as
/// incomplete; after a core update the state is recomputed from all stored
/// events, unknown ones included (`docs/13-sync-versions.md`, «Неизвестные события»).
// Nearly every element of a log is `Known`: boxing it would add an
// allocation per event and save no memory.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    Known(Event),
    /// Written by a newer core: stored as is, never guessed at.
    Unknown(UnknownEvent),
}

/// An event with a valid envelope whose content this core does not know: a
/// newer `schema_version`, payload `type` or `response.kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEvent {
    pub schema_version: u64,
    pub event_id: EventId,
    pub learner_id: LearnerId,
    /// The JSON exactly as received.
    pub raw: String,
}

/// Envelope fields every event of every schema version must have.
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "schema_version")]
    _schema_version: u64,
    event_id: EventId,
    learner_id: LearnerId,
    #[serde(rename = "session_id")]
    _session_id: Option<SessionId>,
    #[serde(rename = "client_ts")]
    _client_ts: Timestamp,
    #[serde(rename = "server_ts")]
    _server_ts: Option<Timestamp>,
}

/// Whether a payload of the current schema carries a tag this core does not
/// know. A missing or non-string tag is not "unknown" but malformed: serde
/// reports it.
fn has_unknown_tag(payload: &Value) -> bool {
    let Some(event_type) = payload.get("type").and_then(Value::as_str) else {
        return false;
    };
    if !EVENT_TYPES.contains(&event_type) {
        return true;
    }
    // `answer` is the only payload that carries a `Response`.
    event_type == "answer"
        && payload
            .get("response")
            .and_then(|r| r.get("kind"))
            .and_then(Value::as_str)
            .is_some_and(|kind| !RESPONSE_KINDS.contains(&kind))
}

#[derive(Debug)]
pub enum DecodeError {
    Json(serde_json::Error),
    MissingSchemaVersion,
    UnknownSchemaVersion(u64),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "invalid event JSON: {e}"),
            Self::MissingSchemaVersion => f.write_str("event has no schema_version"),
            Self::UnknownSchemaVersion(v) => write!(f, "unknown schema_version {v}"),
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

/// Parses an event of any supported schema version, upcasting it to the
/// current one. An event from a newer core — a future `schema_version`, or a
/// payload `type` or `response.kind` not in [`EVENT_TYPES`] or
/// [`RESPONSE_KINDS`] — is [`Decoded::Unknown`] as long as its envelope is
/// valid. Upcasters must pass payloads of unknown type through unchanged.
///
/// # Errors
/// Invalid JSON, an invalid envelope, invalid field values of a known type,
/// or a missing, zero or unsafe (above 2^53 - 1) `schema_version`.
pub fn decode_event(json: &str) -> Result<Decoded, DecodeError> {
    let mut value: Value = serde_json::from_str(json)?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(DecodeError::MissingSchemaVersion)?;
    if schema_version == 0 || schema_version > MAX_SAFE_INTEGER {
        return Err(DecodeError::UnknownSchemaVersion(schema_version));
    }
    let envelope = Envelope::deserialize(&value)?;
    let unknown = || {
        Decoded::Unknown(UnknownEvent {
            schema_version,
            event_id: envelope.event_id,
            learner_id: envelope.learner_id,
            raw: json.into(),
        })
    };
    let version = match u16::try_from(schema_version) {
        Ok(v) if v <= CURRENT_SCHEMA_VERSION => v,
        _ => return Ok(unknown()),
    };
    for upcast in &UPCASTERS[usize::from(version - 1)..] {
        value = upcast(value)?;
    }
    if value.get("payload").is_some_and(has_unknown_tag) {
        return Ok(unknown());
    }
    Ok(Decoded::Known(serde_json::from_value(value)?))
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use alloc::vec::Vec;

    use super::*;

    /// Variant names as serde itself lists them when it meets an unknown tag.
    fn serde_variants<T: serde::de::DeserializeOwned + fmt::Debug>(probe: &str) -> Vec<String> {
        let err = serde_json::from_str::<T>(probe).unwrap_err().to_string();
        // "… expected `a`", "… expected `a` or `b`", "… expected one of `a`, `b`, `c`"
        let (_, expected) = err
            .split_once("expected ")
            .unwrap_or_else(|| panic!("unexpected serde message: {err}"));
        let mut names: Vec<String> = expected
            .split('`')
            .skip(1)
            .step_by(2)
            .map(ToString::to_string)
            .collect();
        names.sort();
        names
    }

    fn sorted(list: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = list.iter().map(|s| (*s).into()).collect();
        v.sort();
        v
    }

    // A new payload or response variant only needs its tag added to the list:
    // older cores decode it as Unknown. Any other change to an existing payload
    // bumps CURRENT_SCHEMA_VERSION (see its doc comment).
    #[test]
    fn event_types_list_every_payload_variant() {
        assert_eq!(
            serde_variants::<EventPayload>(r#"{"type":"__probe__"}"#),
            sorted(EVENT_TYPES)
        );
    }

    #[test]
    fn response_kinds_list_every_response_variant() {
        assert_eq!(
            serde_variants::<Response>(r#"{"kind":"__probe__","value":0}"#),
            sorted(RESPONSE_KINDS)
        );
    }
}
