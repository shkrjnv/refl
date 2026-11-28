//! Placement on the current real bundle: ten correct diagnostic answers must
//! not leave a learner at Pre-A1 because untested live KCs keep their prior.

use std::path::Path;

use refl_core::bundle::{Bundle, KcStatus};
use refl_core::event::{AnswerContext, EventPayload, Response, Verdict};
use refl_core::ids::{DomainId, ItemRef, SubjectId};
use refl_core::item::KcRole;
use refl_core::model::{KnowledgeModel, LearnerModel, Profile, replay};
use refl_core::placement::{self, Placement, PlacementNext};
use refl_core::value::Timestamp;
use refl_sim::{T0, answer, event};

fn live_bundle() -> Bundle {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = content_build::build(&content_build::Options::new(content)).expect("build");
    out.bundle.expect("the real content builds")
}

fn eligible_pairs(bundle: &Bundle, subject: &SubjectId) -> Vec<(ItemRef, u16)> {
    let mut pairs = Vec::new();
    for entry in &bundle.content.kcs {
        if entry.status != KcStatus::Live || &entry.kc.subject != subject {
            continue;
        }
        let Some(items) = bundle.content.items_by_kc.get(&entry.kc.id) else {
            continue;
        };
        for item in &items.primary {
            let Some(projection) = bundle.projection(item) else {
                continue;
            };
            for (index, slot) in projection.slots.iter().enumerate() {
                if slot
                    .kcs
                    .iter()
                    .any(|row| row.kc == entry.kc.id && row.role == KcRole::Primary)
                    && let Ok(slot) = u16::try_from(index)
                {
                    pairs.push((item.clone(), slot));
                }
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

fn correct_event(item: ItemRef, tail: u64, at: Timestamp) -> refl_core::event::Decoded {
    let mut answer = answer(item, Verdict::Correct, Response::Option(0), 8_000);
    answer.context = AnswerContext::Placement;
    event(at, tail, EventPayload::Answer(answer))
}

fn assert_live_ceiling(result: &placement::PlacementResult) {
    let level = result.level.expect("a live pool has a level");
    assert_eq!(result.answered, 10);
    assert_eq!(level.point, 2.0, "10/10 should reach A2: {level:?}");
    assert_eq!(level.low, 1.0, "the band should start at A1: {level:?}");
    assert_eq!(level.high, 2.0, "A2 is the live ceiling: {level:?}");
    assert_eq!(result.start_level, 2.0);
}

#[test]
fn adaptive_ten_correct_live_items_reach_the_live_ceiling() {
    let bundle = live_bundle();
    let model = KnowledgeModel::default();
    let subject = SubjectId::new("en").unwrap();
    let at = Timestamp(T0.0 + 20 * 86_400_000);
    let mut state = model.init(&Profile::default());
    let mut placement = Placement {
        subject: subject.clone(),
        asked: Vec::new(),
        correct: 0,
    };
    let mut events = Vec::new();
    loop {
        match placement::next(&model, &bundle, &state, &placement, at) {
            PlacementNext::Item { item, slot } => {
                events.push(correct_event(
                    item.clone(),
                    u64::try_from(events.len()).unwrap() + 1,
                    at,
                ));
                state = replay(&model, &Profile::default(), &bundle, &events);
                assert!(placement::record(&bundle, &mut placement, item, slot, true));
            }
            PlacementNext::Done(result) => {
                assert_live_ceiling(&result);
                break;
            }
            PlacementNext::Unavailable => panic!("live pool must be available"),
        }
    }
}

#[test]
fn ten_correct_live_items_reach_the_live_ceiling() {
    let bundle = live_bundle();
    let model = KnowledgeModel::default();
    let subject = SubjectId::new("en").unwrap();
    let at = Timestamp(T0.0 + 20 * 86_400_000);
    let pairs = eligible_pairs(&bundle, &subject);
    assert!(pairs.len() >= 10, "the live bank has enough checked slots");

    let mut placement = Placement {
        subject: subject.clone(),
        asked: Vec::new(),
        correct: 0,
    };
    let mut events = Vec::new();
    for (index, (item, slot)) in pairs.iter().take(10).enumerate() {
        events.push(correct_event(
            item.clone(),
            u64::try_from(index).unwrap() + 1,
            at,
        ));
        assert!(placement::record(
            &bundle,
            &mut placement,
            item.clone(),
            *slot,
            true
        ));
    }
    let state = replay(&model, &Profile::default(), &bundle, &events);
    let PlacementNext::Done(result) = placement::next(&model, &bundle, &state, &placement, at)
    else {
        panic!("ten answers must finish the check");
    };
    assert_live_ceiling(&result);
}

#[test]
fn draft_kcs_do_not_change_the_level() {
    let bundle = live_bundle();
    let mut live_only = bundle.clone();
    live_only
        .content
        .kcs
        .retain(|entry| entry.status == KcStatus::Live);
    let model = KnowledgeModel::default();
    let subject = SubjectId::new("en").unwrap();
    let domain = DomainId::new("grammar").unwrap();
    let at = Timestamp(T0.0);
    let state = model.init(&Profile::default());
    assert_eq!(
        model.level(&state, &bundle, &subject, &domain, at),
        model.level(&state, &live_only, &subject, &domain, at),
    );
}
