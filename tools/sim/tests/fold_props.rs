//! Properties of the fold on generated logs (`docs/02-layer1-content.md`,
//! «Порядок свёртки»): the state depends on the order of events only
//! through `event_id`; a replay is reproducible; events arriving late,
//! twice, or as corrections end in the same state as folding the whole log
//! in order.

use proptest::prelude::*;
use refl_core::bundle::Bundle;
use refl_core::event::Decoded;
use refl_core::model::{Families, FoldCtx, KnowledgeModel, LearnerModel, Profile, State, replay};
use refl_core::rng::SplitMix64;
use refl_sim::{BundleSpec, LogSpec, synthetic_bundle, synthetic_log};

fn bundle() -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 8,
        items_per_kc: 6,
    })
}

fn shuffled(events: &[Decoded], seed: u64) -> Vec<Decoded> {
    let mut out = events.to_vec();
    let mut rng = SplitMix64::new(seed);
    for i in (1..out.len()).rev() {
        let j = usize::try_from(rng.next_u64() % (i as u64 + 1)).unwrap();
        out.swap(i, j);
    }
    out
}

/// What a device or the server does as events arrive: fold each onto the
/// state, and recompute from everything received when the fold refuses.
fn ingest(model: &KnowledgeModel, bundle: &Bundle, arrivals: &[Decoded]) -> State {
    let profile = Profile::default();
    let mut received: Vec<Decoded> = Vec::new();
    let mut state = model.init(&profile);
    let families = Families::of(bundle);
    for ev in arrivals {
        received.push(ev.clone());
        if model
            .apply(&mut state, ev, &FoldCtx::new(bundle, &families))
            .is_err()
        {
            state = replay(model, &profile, bundle, &received);
        }
    }
    state
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn replay_depends_on_event_id_order_only(seed in any::<u64>(), shuffle in any::<u64>()) {
        let b = bundle();
        let log = synthetic_log(&b, LogSpec { events: 400, seed });
        let model = KnowledgeModel::default();
        let in_order = replay(&model, &Profile::default(), &b, &log);
        let any_order = replay(&model, &Profile::default(), &b, &shuffled(&log, shuffle));
        prop_assert_eq!(in_order.hash(), any_order.hash());
    }

    #[test]
    fn replay_is_reproducible_and_round_trips(seed in any::<u64>()) {
        let b = bundle();
        let log = synthetic_log(&b, LogSpec { events: 400, seed });
        let model = KnowledgeModel::default();
        let first = replay(&model, &Profile::default(), &b, &log);
        let second = replay(&model, &Profile::default(), &b, &log);
        prop_assert_eq!(first.hash(), second.hash());
        let back: State = serde_json::from_slice(&first.canonical_bytes()).unwrap();
        prop_assert_eq!(back.hash(), first.hash());
    }

    #[test]
    fn late_arrivals_end_where_the_ordered_fold_ends(seed in any::<u64>(), shuffle in any::<u64>()) {
        let b = bundle();
        let log = synthetic_log(&b, LogSpec { events: 250, seed });
        let model = KnowledgeModel::default();
        let expected = replay(&model, &Profile::default(), &b, &log).hash();
        prop_assert_eq!(ingest(&model, &b, &log).hash(), expected.clone());
        prop_assert_eq!(ingest(&model, &b, &shuffled(&log, shuffle)).hash(), expected);
    }

    #[test]
    fn one_late_event_then_recompute(seed in any::<u64>(), pick in any::<u64>()) {
        let b = bundle();
        let log = synthetic_log(&b, LogSpec { events: 250, seed });
        let k = usize::try_from(pick % log.len() as u64).unwrap();
        let mut arrivals = log.clone();
        let late = arrivals.remove(k);
        arrivals.push(late);
        let model = KnowledgeModel::default();
        let expected = replay(&model, &Profile::default(), &b, &log).hash();
        prop_assert_eq!(ingest(&model, &b, &arrivals).hash(), expected);
    }

    #[test]
    fn duplicates_fold_once(seed in any::<u64>(), shuffle in any::<u64>()) {
        let b = bundle();
        let log = synthetic_log(&b, LogSpec { events: 200, seed });
        let mut twice = log.clone();
        twice.extend(shuffled(&log, shuffle).into_iter().take(50));
        let model = KnowledgeModel::default();
        let expected = replay(&model, &Profile::default(), &b, &log).hash();
        prop_assert_eq!(replay(&model, &Profile::default(), &b, &twice).hash(), expected.clone());
        prop_assert_eq!(ingest(&model, &b, &twice).hash(), expected);
    }
}
