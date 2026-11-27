use std::fs;

use refl_core::bundle::Bundle;
use refl_core::event::{Decoded, decode_event};
use refl_core::model::{KnowledgeModel, Profile, State, replay};
use serde_json::{Value, json};

fn bundle_json() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../content/bench/v3/bundle.json"
    );
    fs::read_to_string(path).unwrap()
}

fn known_event() -> Value {
    json!({
        "schema_version": 2,
        "event_id": "01930000-0000-7000-8000-000000000001",
        "learner_id": "01930000-0000-7000-8000-0000000000aa",
        "session_id": null,
        "client_ts": 1_700_000_000_000_i64,
        "server_ts": null,
        "payload": { "type": "lesson_started", "lesson": "lesson.articles" },
    })
}

fn unknown_event() -> Value {
    json!({
        "schema_version": 2,
        "event_id": "01930000-0000-7000-8000-000000000002",
        "learner_id": "01930000-0000-7000-8000-0000000000aa",
        "session_id": null,
        "client_ts": 1_700_000_001_000_i64,
        "server_ts": null,
        "payload": { "type": "sticker_sent" },
    })
}

#[test]
fn core_info_has_the_current_wire_version() {
    let info: Value = serde_json::from_str(&refl_core_wasm::core_info()).unwrap();
    assert_eq!(info["schema_version"], json!(2));
    assert_eq!(info["max_bundle_format"], json!(10));
    assert!(info["core_version"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(info["fold_version"].as_str().is_some_and(|v| !v.is_empty()));
}

#[test]
fn event_decode_classifies_known_unknown_and_invalid() {
    let known: Value = serde_json::from_str(&refl_core_wasm::decode_event_json(
        &known_event().to_string(),
    ))
    .unwrap();
    assert_eq!(known["kind"], json!("event"));

    let unknown: Value = serde_json::from_str(&refl_core_wasm::decode_event_json(
        &unknown_event().to_string(),
    ))
    .unwrap();
    assert_eq!(unknown["kind"], json!("unknown"));
    assert_eq!(unknown["event"]["payload"]["type"], json!("sticker_sent"));

    let invalid: Value = serde_json::from_str(&refl_core_wasm::decode_event_json(
        "{\"event_id\":\"broken\"}",
    ))
    .unwrap();
    assert_eq!(invalid["kind"], json!("error"));
    assert_eq!(invalid["error"]["code"], json!("decode_error"));
}

#[test]
fn fold_matches_core_replay_and_marks_unknown_events() {
    let bundle: Bundle = serde_json::from_str(&bundle_json()).unwrap();
    let raw_events = [known_event(), unknown_event()];
    let decoded: Vec<Decoded> = raw_events
        .iter()
        .map(|event| decode_event(&event.to_string()).unwrap())
        .collect();
    let expected = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &bundle,
        &decoded,
    );

    let result: Value = serde_json::from_str(&refl_core_wasm::fold_state(
        &bundle_json(),
        "",
        None,
        &json!(raw_events).to_string(),
    ))
    .unwrap();
    assert_eq!(result["state_hash"], json!(expected.hash()));
    assert_eq!(result["incomplete"], json!(expected.incomplete));
    assert_eq!(
        result["last_event_id"],
        serde_json::to_value(expected.position).unwrap()
    );
    let snapshot: State = serde_json::from_value(result["snapshot"].clone()).unwrap();
    assert_eq!(snapshot.hash(), expected.hash());

    let resumed: Value = serde_json::from_str(&refl_core_wasm::fold_state(
        &bundle_json(),
        "",
        Some(serde_json::to_string(&expected).unwrap()),
        "[]",
    ))
    .unwrap();
    assert_eq!(resumed["state_hash"], json!(expected.hash()));
    assert_eq!(resumed["incomplete"], json!(true));
}

#[test]
fn compatibility_and_answer_check_against_v3() {
    let bundle = bundle_json();
    let compatibility: Value =
        serde_json::from_str(&refl_core_wasm::check_bundle_compatibility(&bundle)).unwrap();
    assert_eq!(compatibility["compatible"], json!(true));
    assert_eq!(compatibility["format_version"], json!(10));

    let mut too_new: Value = serde_json::from_str(&bundle).unwrap();
    too_new["format_version"] = json!(999);
    let too_new: Value = serde_json::from_str(&refl_core_wasm::check_bundle_compatibility(
        &too_new.to_string(),
    ))
    .unwrap();
    assert_eq!(too_new["compatible"], json!(false));
    assert_eq!(too_new["reason"], json!("bundle_format_too_new"));

    let bundle_value: Value = serde_json::from_str(&bundle).unwrap();
    let entry = bundle_value["content"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["item"]["type"] == json!("choice"))
        .unwrap();
    let item_ref = json!({
        "id": entry["item"]["id"],
        "version": entry["item"]["version"],
    });
    let checked: Value = serde_json::from_str(&refl_core_wasm::check_answer(
        &bundle,
        "\"\"",
        &item_ref.to_string(),
        &json!({ "kind": "option", "value": 0 }).to_string(),
        500.0,
    ))
    .unwrap();
    assert_eq!(checked["kind"], json!("checked"));
    assert_eq!(checked["verdict"], json!("correct"));
    assert_eq!(checked["item"], item_ref);
}

#[test]
fn plan_session_returns_a_plan_and_issued_decision() {
    let bundle = bundle_json();
    let state: Value =
        serde_json::from_str(&refl_core_wasm::fold_state(&bundle, "", None, "[]")).unwrap();
    let session: Value = serde_json::from_str(&refl_core_wasm::plan_session(
        &bundle,
        &state["snapshot"].to_string(),
        1_700_000_000_000.0,
        1_500_000.0,
    ))
    .unwrap();
    assert_eq!(session["plan"]["budget"], json!("twenty_five"));
    assert!(session["plan"]["steps"].is_array());
    assert!(session["plan"].is_object());
    assert!(session["issued"].is_object() || session["issued"].is_null());
}
