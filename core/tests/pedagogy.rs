//! Optional docs/12 pedagogical data: old content without these fields stays
//! valid, while authored refutations and representations round-trip.

use refl_core::content::{Example, Misconception, Refutation, Representation};

#[test]
fn omitted_fields_default_to_empty_and_none() {
    let misconception: Misconception = serde_json::from_value(serde_json::json!({
        "id": "en.m.one",
        "title": { "en": "one" },
        "affects": ["en.k.a"],
        "origin": "l1-transfer",
        "l1": { "uz": true, "ru": true },
        "prior": { "uz": 0.5, "ru": 0.4 },
        "remedy": "en.k.a"
    }))
    .expect("old misconception without refutations");
    assert_eq!(misconception.refutations, Vec::new());

    let example: Example = serde_json::from_value(serde_json::json!({
        "text": "I have a cat."
    }))
    .expect("old example without representation");
    assert_eq!(example.representation, None);
}

#[test]
fn representations_and_refutations_round_trip() {
    assert_eq!(
        serde_json::to_value(Representation::Pictorial).unwrap(),
        serde_json::json!("pictorial")
    );

    let refutation: Refutation = serde_json::from_value(serde_json::json!({
        "kind": "contrast",
        "correct": "I have lost my keys.",
        "incorrect": "I lost my keys.",
        "difference": { "en": "missing now", "ru": "нет сейчас" }
    }))
    .expect("contrast refutation");
    let json = serde_json::to_value(&refutation).unwrap();
    assert_eq!(json["kind"], "contrast");
    assert_eq!(json["correct"], "I have lost my keys.");

    let counterexample: Refutation = serde_json::from_value(serde_json::json!({
        "kind": "counterexample",
        "claim": "a before every vowel letter",
        "counterexample": "a university",
        "explanation": { "en": "the sound is [ju]" }
    }))
    .expect("counterexample refutation");
    assert!(matches!(
        counterexample,
        Refutation::Counterexample { ref counterexample, .. } if counterexample == "a university"
    ));
}
