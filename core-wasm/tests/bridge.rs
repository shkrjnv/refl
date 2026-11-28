use std::collections::BTreeMap;
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

fn many_events(count: usize) -> Vec<Value> {
    let mut events: Vec<Value> = (1..=count)
        .map(|i| {
            json!({
                "schema_version": 2,
                "event_id": format!("01930000-0000-7000-8000-{i:012x}"),
                "learner_id": "01930000-0000-7000-8000-0000000000aa",
                "session_id": null,
                "client_ts": 1_700_000_000_000_i64 + i64::try_from(i).unwrap(),
                "server_ts": null,
                "payload": { "type": "lesson_started", "lesson": "lesson.articles" },
            })
        })
        .collect();
    // Insert an unknown event by id order.
    events.insert(
        count / 2,
        json!({
            "schema_version": 2,
            "event_id": format!("01930000-0000-7000-8000-{:012x}", count + 1),
            "learner_id": "01930000-0000-7000-8000-0000000000aa",
            "session_id": null,
            "client_ts": 1_700_000_000_000_i64,
            "server_ts": null,
            "payload": { "type": "sticker_sent" },
        }),
    );
    events
}

fn native_state(bundle: &Bundle, events: &[Value]) -> State {
    let decoded: Vec<Decoded> = events
        .iter()
        .map(|event| decode_event(&event.to_string()).unwrap())
        .collect();
    replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        bundle,
        &decoded,
    )
}

fn state_from_snapshot(session: &refl_core_wasm::WasmSession) -> State {
    serde_json::from_str(&session.snapshot_json()).unwrap()
}

#[test]
fn stateful_fold_matches_stateless_and_native() {
    let bundle_json = bundle_json();
    let bundle: Bundle = serde_json::from_str(&bundle_json).unwrap();
    let events = many_events(2_500);
    let expected = native_state(&bundle, &events);

    let stateless: Value = serde_json::from_str(&refl_core_wasm::fold_state(
        &bundle_json,
        "",
        None,
        &json!(events).to_string(),
    ))
    .unwrap();
    let stateless_state: State = serde_json::from_value(stateless["snapshot"].clone()).unwrap();
    assert_eq!(
        stateless_state.canonical_bytes(),
        expected.canonical_bytes()
    );

    let snapshot = serde_json::to_string(&expected).unwrap();
    let mut resumed = refl_core_wasm::WasmSession::new(&bundle_json, Some(snapshot)).unwrap();
    let duplicate = events
        .iter()
        .find(|event| event["payload"]["type"] == json!("sticker_sent"))
        .expect("unknown event");
    let resumed_result: Value =
        serde_json::from_str(&resumed.fold_event(&duplicate.to_string())).unwrap();
    assert_eq!(resumed_result["state_hash"], json!(expected.hash()));
    assert_eq!(resumed.state_hash(), expected.hash());

    let mut one = refl_core_wasm::WasmSession::new(&bundle_json, None).unwrap();
    let first_result: Value =
        serde_json::from_str(&one.fold_event(&events[0].to_string())).unwrap();
    assert_eq!(first_result["state_hash"], json!(one.state_hash()));
    assert!(!one.is_incomplete());

    let mut incremental = refl_core_wasm::WasmSession::new(&bundle_json, None).unwrap();
    let first = incremental.fold_events(&json!(events[..1_000]).to_string());
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["incomplete"], json!(false));
    let second = incremental.fold_events(&json!(events[1_000..]).to_string());
    let second: Value = serde_json::from_str(&second).unwrap();
    assert_eq!(second["state_hash"], json!(expected.hash()));
    assert_eq!(second["incomplete"], json!(true));
    assert!(incremental.is_incomplete());
    assert_eq!(incremental.state_hash(), expected.hash());
    assert_eq!(
        state_from_snapshot(&incremental).canonical_bytes(),
        expected.canonical_bytes()
    );

    let mut reversed = refl_core_wasm::WasmSession::new(&bundle_json, None).unwrap();
    let reversed_result: Value = serde_json::from_str(
        &reversed.fold_events(&json!(events.iter().rev().collect::<Vec<_>>()).to_string()),
    )
    .unwrap();
    assert_eq!(reversed_result["state_hash"], json!(expected.hash()));
    assert_eq!(
        state_from_snapshot(&reversed).canonical_bytes(),
        expected.canonical_bytes()
    );
}

#[test]
fn stateful_plan_and_placement_methods_are_available() {
    let bundle = bundle_json();
    let folded: Value =
        serde_json::from_str(&refl_core_wasm::fold_state(&bundle, "", None, "[]")).unwrap();
    let snapshot = folded["snapshot"].to_string();
    let session = refl_core_wasm::WasmSession::new(&bundle, None).unwrap();

    let plan: Value = serde_json::from_str(&refl_core_wasm::plan_session(
        &bundle,
        &snapshot,
        1_700_000_000_000.0,
        1_500_000.0,
    ))
    .unwrap();
    let plan_json = plan["plan"].to_string();
    let stateful = session.plan_next(&plan_json, 1_700_000_000_000.0);
    let stateless = refl_core_wasm::next_item(&bundle, &snapshot, &plan_json, 1_700_000_000_000.0);
    assert_eq!(stateful, stateless);

    let placement = json!({
        "subject": "en",
        "asked": [],
        "correct": 0,
    });
    let stateful = session.placement_next(&placement.to_string(), 1_700_000_000_000.0);
    let stateless = refl_core_wasm::placement_next(
        &bundle,
        &snapshot,
        &placement.to_string(),
        1_700_000_000_000.0,
    );
    assert_eq!(stateful, stateless);
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

#[test]
fn placement_check_uses_live_items_and_finishes_with_a_level() {
    let bundle = bundle_json();
    let folded: Value =
        serde_json::from_str(&refl_core_wasm::fold_state(&bundle, "", None, "[]")).unwrap();
    let state = folded["snapshot"].to_string();
    let mut result: Value = serde_json::from_str(&refl_core_wasm::placement_start(
        &bundle,
        &state,
        "en",
        1_700_000_000_000.0,
    ))
    .unwrap();
    let mut answered = 0u64;
    while result["kind"] == json!("item") {
        let item = result["item"].clone();
        let slot = result["slot"].as_u64().unwrap();
        let placement = result["placement"].to_string();
        result = serde_json::from_str(&refl_core_wasm::placement_answer(
            &bundle,
            &state,
            &placement,
            &item.to_string(),
            u16::try_from(slot).unwrap(),
            true,
            1_700_000_000_000.0,
        ))
        .unwrap();
        answered += 1;
        assert!(answered <= 10, "the check must stop at ten items");
    }
    assert_eq!(result["kind"], json!("done"));
    assert!(result["start_level"].as_f64().unwrap().is_finite());
    assert!(result["answered"].as_u64().unwrap() >= 3);
    assert_eq!(result["source"], json!("diagnostic"));
}

#[test]
fn placement_completed_updates_the_profile_and_matches_replay() {
    let bundle = bundle_json();
    let event = json!({
        "schema_version": 2,
        "event_id": "01930000-0000-7000-8000-000000000011",
        "learner_id": "01930000-0000-7000-8000-0000000000aa",
        "session_id": null,
        "client_ts": 1_700_000_000_000_i64,
        "server_ts": null,
        "payload": {
            "type": "placement_completed",
            "start_level": { "en": 1.0 },
            "answered": 4,
            "source": "diagnostic",
        },
    });
    let folded: Value = serde_json::from_str(&refl_core_wasm::fold_state(
        &bundle,
        "",
        None,
        &json!([event.clone()]).to_string(),
    ))
    .unwrap();
    assert_eq!(
        folded["snapshot"]["profile"]["start_level"]["en"],
        json!(1.0)
    );

    let parsed: Bundle = serde_json::from_str(&bundle).unwrap();
    let decoded = decode_event(&event.to_string()).unwrap();
    let expected = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &parsed,
        &[decoded],
    );
    assert_eq!(folded["snapshot"], serde_json::to_value(&expected).unwrap(),);
}

#[test]
fn book_coverage_uses_the_folded_lexical_cards() {
    use refl_core::book::{Book, BookChapter, BookParagraph, BookSentence, BookToken};
    use refl_core::ids::{BookId, LexemeId};
    use refl_core::model::{LearnerModel, LexicalDirection, LexicalSource};
    use refl_core::subject::LevelKey;
    use refl_core::value::Localized;

    let known = LexemeId::new("en.lex.cat.animal").unwrap();
    let unknown = LexemeId::new("en.lex.dog.animal").unwrap();
    let model = KnowledgeModel::default();
    let mut state = model.init(&Profile::default());
    let now = refl_core::value::Timestamp(1_700_000_000_000);
    model.review_lexeme(
        &mut state,
        &known,
        LexicalDirection::Recognize,
        true,
        now,
        LexicalSource::Bundle,
    );
    let token = |id: &str, lexeme: Option<&LexemeId>| BookToken {
        id: id.into(),
        text: "x".into(),
        lemma: "x".into(),
        lexeme_id: lexeme.cloned(),
        sense_ids: Vec::new(),
        kc_ids: Vec::new(),
    };
    let book = Book {
        id: BookId::new("book.first-day").unwrap(),
        version: "a".repeat(64),
        title: Localized::default(),
        level: LevelKey::new("A1").unwrap(),
        license: "original".into(),
        source: "test".into(),
        chapters: vec![BookChapter {
            id: "chapter.first-day.1".into(),
            title: Localized::default(),
            paragraphs: vec![BookParagraph {
                id: "para.first-day.1.1".into(),
                sentences: vec![BookSentence {
                    id: "sentence.first-day.1.1.1".into(),
                    text: "x x x".into(),
                    tokens: vec![
                        token("t1", Some(&known)),
                        token("t2", Some(&unknown)),
                        token("t3", None),
                    ],
                }],
            }],
        }],
    };
    let result: Value = serde_json::from_str(&refl_core_wasm::book_coverage(
        &serde_json::to_string(&book).unwrap(),
        &serde_json::to_string(&state).unwrap(),
        1_700_000_000_000.0,
    ))
    .unwrap();
    assert_eq!(result["book"], json!("book.first-day"));
    assert_eq!(result["coverage"]["known_tokens"], json!(1));
    assert_eq!(result["coverage"]["total_tokens"], json!(2));
    assert_eq!(result["coverage"]["percent"], json!(0.5));
}

#[test]
fn batch4_records_fold_through_the_bridge_and_match_replay() {
    let bundle = bundle_json();
    let note_id = "01930000-0000-7000-8000-000000000012";
    let custom_id = "01930000-0000-7000-8000-000000000013";
    let events = json!([
        {
            "schema_version": 2,
            "event_id": note_id,
            "learner_id": "01930000-0000-7000-8000-0000000000aa",
            "session_id": null,
            "client_ts": 1_700_000_000_000_i64,
            "server_ts": null,
            "payload": {
                "type": "note_saved",
                "note_id": note_id,
                "anchor": { "kind": "standalone" },
                "body": "mira's list\n\n- a cat",
            },
        },
        {
            "schema_version": 2,
            "event_id": custom_id,
            "learner_id": "01930000-0000-7000-8000-0000000000aa",
            "session_id": null,
            "client_ts": 1_700_000_001_000_i64,
            "server_ts": null,
            "payload": {
                "type": "custom_lexeme_created",
                "custom_id": custom_id,
                "source": "own",
                "lemma": "mira",
                "pos": "noun",
                "definition_en": "a name",
                "gloss_uz": "ism",
                "gloss_ru": "имя",
                "examples": [],
                "collocations": [],
            },
        },
    ]);
    let folded: Value = serde_json::from_str(&refl_core_wasm::fold_state(
        &bundle,
        "",
        None,
        &events.to_string(),
    ))
    .unwrap();
    let snapshot: State = serde_json::from_value(folded["snapshot"].clone()).unwrap();
    assert_eq!(snapshot.notes.len(), 1);
    assert_eq!(snapshot.custom_lexemes.len(), 1);
    assert_eq!(
        snapshot
            .lexical_cards
            .values()
            .map(BTreeMap::len)
            .sum::<usize>(),
        2
    );
    let parsed: Bundle = serde_json::from_str(&bundle).unwrap();
    let decoded: Vec<Decoded> = events
        .as_array()
        .unwrap()
        .iter()
        .map(|event| decode_event(&event.to_string()).unwrap())
        .collect();
    let expected = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &parsed,
        &decoded,
    );
    assert_eq!(folded["snapshot"], serde_json::to_value(&expected).unwrap());
}
