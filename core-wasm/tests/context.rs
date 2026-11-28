//! `CoreContext` must answer every call exactly like the stateless free
//! functions it replaces in the browser worker.

use std::fs;

use refl_core_wasm::CoreContext;
use serde_json::{Value, json};

fn bundle_json() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../content/bench/v3/bundle.json"
    );
    fs::read_to_string(path).unwrap()
}

const LEXICON: &str = "\"\"";
const NOW: f64 = 1_700_000_000_000.0;

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_whole_session_is_identical_through_the_context() {
    let bundle = bundle_json();
    let context = CoreContext::new(&bundle, Some(LEXICON.to_owned())).unwrap();

    let free = refl_core_wasm::fold_state(&bundle, LEXICON, None, "[]");
    assert_eq!(context.fold_state(None, "[]"), free);
    let snapshot = parse(&free)["snapshot"].to_string();

    for budget in [600_000.0, 900_000.0, 1_500_000.0] {
        let free = refl_core_wasm::plan_session(&bundle, &snapshot, NOW, budget);
        assert_eq!(context.plan_session(&snapshot, NOW, budget), free);
    }

    let mut plan = parse(&refl_core_wasm::plan_session(
        &bundle,
        &snapshot,
        NOW,
        1_500_000.0,
    ))["plan"]
        .to_string();
    let mut steps: u32 = 0;
    loop {
        let now = NOW + f64::from(steps) * 20_000.0;
        let free = refl_core_wasm::next_item(&bundle, &snapshot, &plan, now);
        assert_eq!(context.next_item(&snapshot, &plan, now), free);
        let next = parse(&free);
        let outcome = match next["kind"].as_str().unwrap() {
            "finished" => break,
            "lesson" => json!({ "kind": "lesson" }),
            "item" => {
                let target = json!({
                    "id": next["item"]["id"],
                    "version": next["item"]["version"],
                    "slot": next["slot"],
                });
                for response in [
                    json!({ "kind": "option", "value": 0 }),
                    json!({ "kind": "option", "value": 1 }),
                    json!({ "kind": "text", "value": "the" }),
                ] {
                    let free = refl_core_wasm::check_answer(
                        &bundle,
                        LEXICON,
                        &target.to_string(),
                        &response.to_string(),
                        4_000.0,
                    );
                    assert_eq!(
                        context.check_answer(&target.to_string(), &response.to_string(), 4_000.0),
                        free
                    );
                }
                json!({
                    "kind": "answer",
                    "item": { "id": next["item"]["id"], "version": next["item"]["version"] },
                    "slot": next["slot"],
                    "correct": !steps.is_multiple_of(3),
                    "rapid": false,
                    "sure": true,
                })
            }
            other => panic!("unexpected next kind {other}"),
        };
        let free =
            refl_core_wasm::replan_session(&bundle, &snapshot, &plan, &outcome.to_string(), now);
        assert_eq!(
            context.replan_session(&snapshot, &plan, &outcome.to_string(), now),
            free
        );
        plan = parse(&free)["plan"].to_string();
        steps += 1;
        assert!(steps < 200, "the session must finish");
    }
    assert!(steps > 3, "the session must contain several steps");
}

#[test]
fn placement_is_identical_through_the_context() {
    let bundle = bundle_json();
    let context = CoreContext::new(&bundle, None).unwrap();
    let snapshot =
        parse(&refl_core_wasm::fold_state(&bundle, LEXICON, None, "[]"))["snapshot"].to_string();

    let free = refl_core_wasm::placement_start(&bundle, &snapshot, "en", NOW);
    assert_eq!(context.placement_start(&snapshot, "en", NOW), free);
    let mut result = parse(&free);
    let mut answered = 0u16;
    while result["kind"] == json!("item") {
        let placement = result["placement"].to_string();
        let free_next = refl_core_wasm::placement_next(&bundle, &snapshot, &placement, NOW);
        assert_eq!(
            context.placement_next(&snapshot, &placement, NOW),
            free_next
        );
        let item = result["item"].to_string();
        let slot = u16::try_from(result["slot"].as_u64().unwrap()).unwrap();
        let correct = answered.is_multiple_of(2);
        let free = refl_core_wasm::placement_answer(
            &bundle, &snapshot, &placement, &item, slot, correct, NOW,
        );
        assert_eq!(
            context.placement_answer(&snapshot, &placement, &item, slot, correct, NOW),
            free
        );
        result = parse(&free);
        answered += 1;
        assert!(answered <= 10);
    }
    assert_eq!(result["kind"], json!("done"));
}

#[test]
fn a_context_without_a_lexicon_refuses_answer_checks() {
    let bundle = bundle_json();
    let context = CoreContext::new(&bundle, None).unwrap();
    let result = parse(&context.check_answer("{}", "{}", 0.0));
    assert_eq!(result["error"]["code"], json!("no_lexicon"));
}
