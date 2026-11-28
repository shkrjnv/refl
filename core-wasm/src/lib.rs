//! Browser bridge for `refl-core`.
//!
//! Every export speaks JSON strings. This keeps the wire format identical to
//! the `ts-rs` types used by the web client and avoids `serde-wasm-bindgen`'s
//! map and bigint conversion rules.

use refl_core::bundle::{BUNDLE_FORMAT_VERSION, Bundle};
use refl_core::event::{CURRENT_SCHEMA_VERSION, Decoded, Response, decode_event};
use refl_core::ids::{ItemRef, SubjectId};
use refl_core::kind::{CheckCtx, CheckOutcome, Compiled, ItemKind};
use refl_core::lexicon::Lexicon;
use refl_core::model::fold_version::FOLD_INPUTS_FORMAT;
use refl_core::model::{
    Families, FoldCtx, KnowledgeModel, LearnerModel, Profile, State, event_id, replay,
};
use refl_core::placement::{self, Placement, PlacementNext};
use refl_core::plan::{
    Answered, Budget, History, Next, Outcome, Params, Plan, Planner, ReadingSpeed,
};
use refl_core::value::{Lang, Seed, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen::prelude::wasm_bindgen;

const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct BridgeError {
    code: &'static str,
    message: String,
}

impl BridgeError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn json(error: &serde_json::Error, code: &'static str) -> Self {
        Self::new(code, error.to_string())
    }
}

fn error_value(error: &BridgeError) -> Value {
    json!({ "error": { "code": error.code, "message": error.message } })
}

fn export(result: Result<Value, BridgeError>) -> String {
    match result {
        Ok(value) => value.to_string(),
        Err(error) => error_value(&error).to_string(),
    }
}

fn decode_export(result: Result<Value, BridgeError>) -> String {
    match result {
        Ok(value) => value.to_string(),
        Err(error) => json!({
            "kind": "error",
            "error": { "code": error.code, "message": error.message },
        })
        .to_string(),
    }
}

fn to_value<T: Serialize>(value: &T) -> Result<Value, BridgeError> {
    serde_json::to_value(value).map_err(|error| BridgeError::json(&error, "serialize"))
}

fn parse_json(json: &str, code: &'static str) -> Result<Value, BridgeError> {
    serde_json::from_str(json).map_err(|error| BridgeError::json(&error, code))
}

fn parse_bundle(json: &str) -> Result<Bundle, BridgeError> {
    serde_json::from_str(json).map_err(|error| BridgeError::json(&error, "invalid_bundle"))
}

fn parse_state(json: &str) -> Result<State, BridgeError> {
    let value = parse_json(json, "invalid_state")?;
    let state_value = value
        .get("state")
        .or_else(|| value.get("snapshot"))
        .cloned()
        .unwrap_or(value);
    serde_json::from_value(state_value).map_err(|error| BridgeError::json(&error, "invalid_state"))
}

fn parse_snapshot(json: Option<&str>) -> Result<Option<State>, BridgeError> {
    let Some(json) = json else {
        return Ok(None);
    };
    if json.trim().is_empty() || json.trim() == "null" {
        return Ok(None);
    }
    parse_state(json).map(Some)
}

fn parse_plan(json: &str) -> Result<Plan, BridgeError> {
    serde_json::from_str(json).map_err(|error| BridgeError::json(&error, "invalid_plan"))
}

fn parse_response(json: &str) -> Result<Response, BridgeError> {
    serde_json::from_str(json).map_err(|error| BridgeError::json(&error, "invalid_response"))
}

fn parse_lexicon(json: &str) -> Result<Lexicon, BridgeError> {
    let text = match serde_json::from_str::<Value>(json) {
        Ok(Value::String(text)) => text,
        Ok(Value::Object(object)) => object
            .get("text")
            .and_then(Value::as_str)
            .map_or_else(|| json.to_owned(), ToOwned::to_owned),
        _ => json.to_owned(),
    };
    Lexicon::parse(text).map_err(|error| BridgeError::new("invalid_lexicon", error.to_string()))
}

fn parse_events(json: &str) -> Result<Vec<Decoded>, BridgeError> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value = parse_json(json, "invalid_events")?;
    let events = value
        .as_array()
        .ok_or_else(|| BridgeError::new("invalid_events", "events must be a JSON array"))?;
    let mut out = Vec::with_capacity(events.len());
    for (index, event) in events.iter().enumerate() {
        let raw = match event {
            Value::String(raw) => raw.clone(),
            other => serde_json::to_string(other)
                .map_err(|error| BridgeError::json(&error, "invalid_events"))?,
        };
        let decoded = decode_event(&raw)
            .map_err(|error| BridgeError::new("decode_error", format!("event {index}: {error}")))?;
        out.push(decoded);
    }
    Ok(out)
}

fn timestamp(ms: f64) -> Result<Timestamp, BridgeError> {
    let millis: i64 = format!("{ms}")
        .parse()
        .map_err(|error| BridgeError::new("invalid_time", format!("{error}")))?;
    Timestamp::try_from(millis).map_err(|error| BridgeError::new("invalid_time", error.to_string()))
}

fn budget(ms: f64) -> Result<Budget, BridgeError> {
    if !ms.is_finite() || ms <= 0.0 {
        return Err(BridgeError::new(
            "invalid_budget",
            "budget_ms must be positive",
        ));
    }
    if ms <= 750_000.0 {
        Ok(Budget::Ten)
    } else if ms <= 1_200_000.0 {
        Ok(Budget::Fifteen)
    } else {
        Ok(Budget::TwentyFive)
    }
}

fn version_parts(version: &str) -> Option<Vec<u64>> {
    version
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn version_older(a: &str, b: &str) -> bool {
    match (version_parts(a), version_parts(b)) {
        (Some(mut a), Some(mut b)) => {
            let width = a.len().max(b.len());
            a.resize(width, 0);
            b.resize(width, 0);
            a < b
        }
        _ => a < b,
    }
}

#[wasm_bindgen]
pub fn core_info() -> String {
    json!({
        "core_version": CORE_VERSION,
        "schema_version": CURRENT_SCHEMA_VERSION,
        "fold_version": format!("fold-inputs-{FOLD_INPUTS_FORMAT}"),
        "max_bundle_format": BUNDLE_FORMAT_VERSION,
    })
    .to_string()
}

fn compatibility(bundle_json: &str) -> Result<Value, BridgeError> {
    let raw = parse_json(bundle_json, "invalid_bundle")?;
    let format_version = raw
        .get("format_version")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let min_core_version = raw
        .get("min_core_version")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut reason = None;
    match format_version {
        None => reason = Some("missing_format_version".to_owned()),
        Some(0) => reason = Some("invalid_format_version".to_owned()),
        Some(version) if version > BUNDLE_FORMAT_VERSION => {
            reason = Some("bundle_format_too_new".to_owned());
        }
        Some(_) => {}
    }
    if reason.is_none()
        && let Some(min) = &min_core_version
    {
        if version_parts(min).is_none() {
            reason = Some("invalid_min_core_version".to_owned());
        } else if version_older(CORE_VERSION, min) {
            reason = Some("core_too_old".to_owned());
        }
    }
    if reason.is_none() && serde_json::from_str::<Bundle>(bundle_json).is_err() {
        reason = Some("invalid_bundle".to_owned());
    }
    let compatible = reason.is_none();
    Ok(json!({
        "compatible": compatible,
        "reason": reason,
        "format_version": format_version,
        "min_core_version": min_core_version,
        "core_version": CORE_VERSION,
        "max_bundle_format": BUNDLE_FORMAT_VERSION,
    }))
}

#[wasm_bindgen]
pub fn check_bundle_compatibility(bundle_json: &str) -> String {
    export(compatibility(bundle_json))
}

fn decode(event_json: &str) -> Result<Value, BridgeError> {
    match decode_event(event_json) {
        Ok(Decoded::Known(event)) => Ok(json!({
            "kind": "event",
            "event": to_value(&event)?,
        })),
        Ok(Decoded::Unknown(event)) => {
            let raw = serde_json::from_str::<Value>(&event.raw)
                .unwrap_or_else(|_| Value::String(event.raw.clone()));
            Ok(json!({
                "kind": "unknown",
                "event": raw,
                "raw": event.raw,
                "schema_version": event.schema_version,
                "event_id": event.event_id,
                "learner_id": event.learner_id,
            }))
        }
        Err(error) => Err(BridgeError::new("decode_error", error.to_string())),
    }
}

#[wasm_bindgen]
pub fn decode_event_json(event_json: &str) -> String {
    decode_export(decode(event_json))
}

fn fold(
    bundle_json: &str,
    snapshot_json_opt: Option<&str>,
    events_json: &str,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let snapshot = parse_snapshot(snapshot_json_opt)?;
    let mut events = parse_events(events_json)?;
    let model = KnowledgeModel::default();
    let state = match snapshot {
        None => replay(&model, &Profile::default(), &bundle, &events),
        Some(mut state) => {
            events.sort_by(|a, b| event_id(a).cmp(event_id(b)));
            let families = Families::of(&bundle);
            let context = FoldCtx::new(&bundle, &families);
            for event in &events {
                model
                    .apply(&mut state, event, &context)
                    .map_err(|reason| BridgeError::new("needs_replay", format!("{reason:?}")))?;
            }
            state
        }
    };
    Ok(json!({
        "snapshot": to_value(&state)?,
        "state_hash": state.hash(),
        "incomplete": state.incomplete,
        "last_event_id": to_value(&state.position)?,
    }))
}

#[wasm_bindgen]
pub fn fold_state(
    bundle_json: &str,
    _lexicon_json: &str,
    snapshot_json_opt: Option<String>,
    events_json: &str,
) -> String {
    let result = match snapshot_json_opt {
        Some(snapshot) => fold(bundle_json, Some(&snapshot), events_json),
        None => fold(bundle_json, None, events_json),
    };
    export(result)
}

fn plan(
    bundle_json: &str,
    snapshot_json: &str,
    now_ms: f64,
    budget_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let now = timestamp(now_ms)?;
    let budget = budget(budget_ms)?;
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading_speed = ReadingSpeed::default();
    let history = History::default();
    let plan_seed =
        Seed::new(0).ok_or_else(|| BridgeError::new("invalid_seed", "seed is unsafe"))?;
    let planner = Planner::new(&model, &bundle, &params, &reading_speed);
    let session = planner.build_session(&state, &history, now, budget, Lang::En, plan_seed);
    Ok(json!({
        "plan": to_value(&session.plan)?,
        "issued": to_value(&session.issued)?,
    }))
}

#[wasm_bindgen]
pub fn plan_session(bundle_json: &str, snapshot_json: &str, now_ms: f64, budget_ms: f64) -> String {
    export(plan(bundle_json, snapshot_json, now_ms, budget_ms))
}

fn parse_placement(json: &str) -> Result<Placement, BridgeError> {
    serde_json::from_str(json).map_err(|error| BridgeError::json(&error, "invalid_placement"))
}

fn placement_value(
    bundle: &Bundle,
    state: &State,
    placement: &Placement,
    now: Timestamp,
) -> Result<Value, BridgeError> {
    let model = KnowledgeModel::default();
    match placement::next(&model, bundle, state, placement, now) {
        PlacementNext::Item { item, slot } => Ok(json!({
            "kind": "item",
            "item": to_value(&item)?,
            "slot": slot,
            "placement": to_value(placement)?,
        })),
        PlacementNext::Done(result) => Ok(json!({
            "kind": "done",
            "placement": to_value(placement)?,
            "level": to_value(&result.level)?,
            "start_level": result.start_level,
            "answered": result.answered,
            "source": to_value(&result.source)?,
        })),
        PlacementNext::Unavailable => Ok(json!({
            "kind": "unavailable",
            "placement": to_value(placement)?,
        })),
    }
}

fn placement_start_inner(
    bundle_json: &str,
    snapshot_json: &str,
    subject: &str,
    now_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let now = timestamp(now_ms)?;
    let subject = SubjectId::new(subject)
        .ok_or_else(|| BridgeError::new("invalid_subject", "subject is not a valid id"))?;
    let placement = Placement {
        subject,
        asked: Vec::new(),
        correct: 0,
    };
    placement_value(&bundle, &state, &placement, now)
}

#[wasm_bindgen]
pub fn placement_start(
    bundle_json: &str,
    snapshot_json: &str,
    subject: &str,
    now_ms: f64,
) -> String {
    export(placement_start_inner(
        bundle_json,
        snapshot_json,
        subject,
        now_ms,
    ))
}

fn placement_next_inner(
    bundle_json: &str,
    snapshot_json: &str,
    placement_json: &str,
    now_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let now = timestamp(now_ms)?;
    let placement = parse_placement(placement_json)?;
    placement_value(&bundle, &state, &placement, now)
}

#[wasm_bindgen]
pub fn placement_next(
    bundle_json: &str,
    snapshot_json: &str,
    placement_json: &str,
    now_ms: f64,
) -> String {
    export(placement_next_inner(
        bundle_json,
        snapshot_json,
        placement_json,
        now_ms,
    ))
}

fn placement_answer_inner(
    bundle_json: &str,
    snapshot_json: &str,
    placement_json: &str,
    item_ref_json: &str,
    slot: u16,
    correct: bool,
    now_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let now = timestamp(now_ms)?;
    let mut placement = parse_placement(placement_json)?;
    let item: ItemRef = serde_json::from_str(item_ref_json)
        .map_err(|error| BridgeError::json(&error, "invalid_item"))?;
    if !placement::record(&bundle, &mut placement, item, slot, correct) {
        return Err(BridgeError::new(
            "invalid_placement_answer",
            "item was already asked or is not an eligible checked item",
        ));
    }
    placement_value(&bundle, &state, &placement, now)
}

#[wasm_bindgen]
pub fn placement_answer(
    bundle_json: &str,
    snapshot_json: &str,
    placement_json: &str,
    item_ref_json: &str,
    slot: u16,
    correct: bool,
    now_ms: f64,
) -> String {
    export(placement_answer_inner(
        bundle_json,
        snapshot_json,
        placement_json,
        item_ref_json,
        slot,
        correct,
        now_ms,
    ))
}

fn next(
    bundle_json: &str,
    snapshot_json: &str,
    plan_json: &str,
    now_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let plan = parse_plan(plan_json)?;
    let now = timestamp(now_ms)?;
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading_speed = ReadingSpeed::default();
    let history = History::default();
    let planner = Planner::new(&model, &bundle, &params, &reading_speed);
    match planner.next_item(&state, &plan, &history, now, plan.seed) {
        Next::Lesson { kc, explanation } => Ok(json!({
            "kind": "lesson",
            "kc": kc,
            "explanation": explanation,
        })),
        Next::Item(choice) => Ok(json!({
            "kind": "item",
            "item": to_value(&choice.item)?,
            "slot": choice.slot,
            "issued": to_value(&choice.issued)?,
        })),
        Next::Finished { end } => Ok(json!({
            "kind": "finished",
            "end": to_value(&end)?,
        })),
    }
}

#[wasm_bindgen]
pub fn next_item(bundle_json: &str, snapshot_json: &str, plan_json: &str, now_ms: f64) -> String {
    export(next(bundle_json, snapshot_json, plan_json, now_ms))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum OutcomeInput {
    Lesson,
    Answer {
        item: ItemRef,
        slot: u16,
        correct: bool,
        rapid: bool,
        sure: bool,
    },
}

impl From<OutcomeInput> for Outcome {
    fn from(value: OutcomeInput) -> Self {
        match value {
            OutcomeInput::Lesson => Self::Lesson,
            OutcomeInput::Answer {
                item,
                slot,
                correct,
                rapid,
                sure,
            } => Self::Answer(Answered {
                item,
                slot,
                correct,
                rapid,
                sure,
            }),
        }
    }
}

fn replan(
    bundle_json: &str,
    snapshot_json: &str,
    plan_json: &str,
    outcome_json: &str,
    now_ms: f64,
) -> Result<Value, BridgeError> {
    let bundle = parse_bundle(bundle_json)?;
    let state = parse_state(snapshot_json)?;
    let plan = parse_plan(plan_json)?;
    let outcome: OutcomeInput = serde_json::from_str(outcome_json)
        .map_err(|error| BridgeError::json(&error, "invalid_outcome"))?;
    let now = timestamp(now_ms)?;
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading_speed = ReadingSpeed::default();
    let planner = Planner::new(&model, &bundle, &params, &reading_speed);
    let adapted = planner.adapt(&state, &plan, &outcome.into(), now);
    Ok(json!({
        "plan": to_value(&adapted.plan)?,
        "rule": to_value(&adapted.rule)?,
        "issued": to_value(&adapted.issued)?,
    }))
}

#[wasm_bindgen]
pub fn replan_session(
    bundle_json: &str,
    snapshot_json: &str,
    plan_json: &str,
    outcome_json: &str,
    now_ms: f64,
) -> String {
    export(replan(
        bundle_json,
        snapshot_json,
        plan_json,
        outcome_json,
        now_ms,
    ))
}

#[derive(Debug, Deserialize)]
struct CheckTarget {
    #[serde(flatten)]
    item: ItemRef,
    #[serde(default)]
    slot: u16,
}

fn answer(
    bundle_json: &str,
    lexicon_json: &str,
    item_ref_json: &str,
    response_json: &str,
    elapsed_ms: f64,
) -> Result<Value, BridgeError> {
    if !elapsed_ms.is_finite() || elapsed_ms < 0.0 {
        return Err(BridgeError::new(
            "invalid_elapsed",
            "elapsed_ms must be finite and non-negative",
        ));
    }
    let bundle = parse_bundle(bundle_json)?;
    let lexicon = parse_lexicon(lexicon_json)?;
    let target: CheckTarget = serde_json::from_str(item_ref_json)
        .map_err(|error| BridgeError::json(&error, "invalid_item_ref"))?;
    let response = parse_response(response_json)?;
    let entry = bundle
        .content
        .items
        .iter()
        .find(|entry| entry.item.id == target.item.id && entry.item.version == target.item.version)
        .ok_or_else(|| BridgeError::new("item_not_found", "item version is not in the bundle"))?;
    let compiled = Compiled {
        slots: &entry.slots,
        tokens: &bundle.content.tokens,
    };
    let context = CheckCtx {
        compiled,
        lexicon: &lexicon,
    };
    let outcome = entry.item.body.check(target.slot, &response, &context);
    match outcome {
        CheckOutcome::Checked {
            verdict,
            misconception,
        } => Ok(json!({
            "kind": "checked",
            "item": to_value(&target.item)?,
            "slot": target.slot,
            "verdict": to_value(&verdict)?,
            "misconception": to_value(&misconception)?,
            "elapsed_ms": elapsed_ms,
        })),
        CheckOutcome::NotChecked => Ok(json!({
            "kind": "not_checked",
            "item": to_value(&target.item)?,
            "slot": target.slot,
            "elapsed_ms": elapsed_ms,
        })),
    }
}

#[wasm_bindgen]
pub fn check_answer(
    bundle_json: &str,
    lexicon_json: &str,
    item_ref_json: &str,
    response_json: &str,
    elapsed_ms: f64,
) -> String {
    export(answer(
        bundle_json,
        lexicon_json,
        item_ref_json,
        response_json,
        elapsed_ms,
    ))
}
