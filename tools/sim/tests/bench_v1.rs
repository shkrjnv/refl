//! The simulator's own determinism (`docs/11-quality.md`, «Версии
//! симулятора»): the same seed and configuration give the same synthetic
//! learners and the same hidden-state trajectory under a fixed scripted
//! policy. The policy and the model are not involved, so a change to the
//! planner or the learner model cannot fail this test. Benchmark v1 itself is
//! frozen by the git tag `bench-v1` (`tools/sim/bench/v1/README.md`), not by
//! a test.

use std::fmt::Write as _;
use std::path::Path;

use refl_core::bundle::Bundle;
use refl_core::value::Timestamp;
use refl_sim::cohort::Learner;
use refl_sim::{DAY_MS, T0, live_fixture};

fn bundle() -> Bundle {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bench/v1/bundle.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    live_fixture(serde_json::from_slice(&bytes).expect("the frozen bundle reads"))
}

/// A learner under a scripted policy: three days, every session-eligible
/// item once a day in bundle order, a fixed clock. Everything the learner
/// does and every KC's recall after each day, as text.
fn trajectory(bundle: &Bundle, seed: u64) -> String {
    let mut learner = Learner::new(bundle, seed);
    let mut out = String::new();
    for day in 0..3_i64 {
        let mut at = T0.0 + day * DAY_MS;
        for entry in bundle.content.items.iter().filter(|e| e.session_eligible) {
            let item = refl_core::ids::ItemRef {
                id: entry.item.id.clone(),
                version: entry.item.version,
                seed: None,
            };
            for slot in 0..entry.item.body.slots().len() {
                let slot = u16::try_from(slot).expect("few slots");
                let reply = learner.answer(bundle, &item, slot, Timestamp(at), 1.0);
                writeln!(out, "{day} {} {slot} {reply:?}", item.id).unwrap();
                at += 20_000;
            }
        }
        for e in &bundle.content.kcs {
            let r = learner.recall(&e.kc.id, Timestamp(T0.0 + (day + 1) * DAY_MS));
            writeln!(out, "{day} {} {:016x}", e.kc.id, r.to_bits()).unwrap();
        }
    }
    out
}

#[test]
fn the_same_seed_gives_the_same_learner_and_trajectory() {
    let bundle = bundle();
    let first = trajectory(&bundle, 0x5EED);
    assert!(first.lines().count() > 100, "the script did nothing");
    assert_eq!(first, trajectory(&bundle, 0x5EED));
}

#[test]
fn other_seeds_give_other_learners() {
    let bundle = bundle();
    assert_ne!(trajectory(&bundle, 1), trajectory(&bundle, 2));
}
