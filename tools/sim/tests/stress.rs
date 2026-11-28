//! W24 stress suite. The cheap determinism checks run by default; the
//! 100k-event, 100k-selection and JSON-boundary runs are `#[ignore]`:
//!
//! `cargo test --release -p refl-sim --test stress -- --ignored --nocapture`

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::time::Instant;

use refl_core::bundle::Bundle;
use refl_core::event::Decoded;
use refl_core::model::{KnowledgeModel, LearnerModel, Profile, State, replay};
use refl_sim::stress::{
    events, fold_native, peak_rss_kb, placement_selection, plan_selection, replay_once,
    replay_threshold, shuffled, snapshot_bytes, state_after, stress_bundle,
};

fn bridge_hash(bundle: &Bundle, events: &[Decoded]) -> (String, usize) {
    let bundle_json = serde_json::to_string(bundle).expect("bundle serializes");
    let raw: Vec<serde_json::Value> = events
        .iter()
        .filter_map(|e| match e {
            Decoded::Known(e) => Some(serde_json::to_value(e).expect("event serializes")),
            Decoded::Unknown(_) => None,
        })
        .collect();
    let result = refl_core_wasm::fold_state(
        &bundle_json,
        "",
        None,
        &serde_json::to_string(&raw).expect("events serialize"),
    );
    let value: serde_json::Value = serde_json::from_str(&result).expect("bridge returns JSON");
    let hash = value["state_hash"]
        .as_str()
        .expect("a state hash")
        .to_owned();
    (hash, result.len())
}

fn same_state(left: &State, right: &State) -> bool {
    left.canonical_bytes() == right.canonical_bytes()
}

#[test]
fn incremental_fold_matches_full_replay_and_the_bridge() {
    let bundle = stress_bundle();
    let log = events(&bundle, 2_000);
    let (state, latency, _, _) = fold_native(&bundle, &log);
    assert_eq!(latency.count, log.len());
    let replayed = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &bundle,
        &log,
    );
    assert!(
        same_state(&state, &replayed),
        "incremental state differs from replay"
    );
    let (hash, bytes) = bridge_hash(&bundle, &log);
    assert_eq!(hash, state.hash(), "bridge hash differs from native");
    assert!(bytes > 0);
}

#[test]
fn shuffled_replay_matches_ordered_incremental_fold() {
    let bundle = stress_bundle();
    let log = events(&bundle, 2_000);
    let (state, _, _, _) = fold_native(&bundle, &log);
    let mixed = shuffled(&log, 0x5EED);
    let replayed = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &bundle,
        &mixed,
    );
    assert!(
        same_state(&state, &replayed),
        "a shuffled replay must fold to the same state"
    );
}

#[test]
#[ignore = "W24: 10k shuffled replay and full State equality"]
fn determinism_10k_shuffled_and_full_state() {
    let bundle = stress_bundle();
    let log = events(&bundle, 10_000);
    let (ordered, _, _, _) = fold_native(&bundle, &log);
    let replayed = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &bundle,
        &log,
    );
    assert_eq!(
        ordered.canonical_bytes(),
        replayed.canonical_bytes(),
        "incremental fold and replay differ"
    );
    let mixed = shuffled(&log, 0xD1CE);
    let mixed_replay = replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        &bundle,
        &mixed,
    );
    assert_eq!(
        ordered.canonical_bytes(),
        mixed_replay.canonical_bytes(),
        "shuffled replay differs from the ordered fold"
    );
    println!(
        "10k shuffled: state_hash={} snapshot_bytes={}",
        ordered.hash(),
        snapshot_bytes(&ordered)
    );
}

#[test]
#[ignore = "W24: 100k-event fold with per-event latency and memory"]
fn fold_100k_events() {
    let bundle = stress_bundle();
    let log = events(&bundle, 100_000);
    let (state, latency, start_bytes, end_bytes) = fold_native(&bundle, &log);
    println!("{}", latency.line("fold native 100k"));
    println!(
        "fold native 100k: snapshot_bytes {} -> {} (+{}), peak_rss_kb {}",
        start_bytes,
        end_bytes,
        end_bytes.saturating_sub(start_bytes),
        peak_rss_kb()
    );
    assert!(state.position.is_some());
    assert!(end_bytes > start_bytes);
    assert_eq!(latency.count, 100_000);
}

#[test]
#[ignore = "W24: replay scaling curve and 16/50 ms compaction thresholds"]
fn replay_scaling_curve() {
    let bundle = stress_bundle();
    let log = events(&bundle, 100_000);
    for n in [100, 1_000, 10_000, 50_000, 100_000] {
        let (state, ms) = replay_once(&bundle, &log[..n]);
        println!(
            "replay n={n} medianish_ms={ms:.3} state_hash={} snapshot_bytes={}",
            state.hash(),
            snapshot_bytes(&state)
        );
    }
    for target in [16.0, 50.0] {
        match replay_threshold(&bundle, &log, target) {
            Some((n, ms)) => {
                println!("replay threshold > {target:.0} ms: first n={n}, median_ms={ms:.3}");
            }
            None => println!("replay threshold > {target:.0} ms: not reached at 100k"),
        }
    }
}

#[test]
#[ignore = "W24: 100k planner and placement selection calls on 32 KCs"]
fn selection_100k_calls() {
    let bundle = stress_bundle();
    let log = events(&bundle, 2_000);
    let state = state_after(&bundle, &log, 1_000);
    let planner = plan_selection(&bundle, &state, 100_000);
    println!("{}", planner.line("planner next_item 100k"));
    let placement = placement_selection(&bundle, &state, 100_000);
    println!("{}", placement.line("placement next 100k"));
    println!(
        "selection bundle: kcs={} items={}",
        bundle.content.kcs.len(),
        bundle.content.items.len()
    );
    assert_eq!(planner.count, 100_000);
    assert_eq!(placement.count, 100_000);
}

#[test]
#[ignore = "W24: 10k native vs core-wasm JSON boundary calls"]
fn wasm_json_boundary_10k() {
    let bundle = stress_bundle();
    let log = events(&bundle, 10_000);
    let bundle_json = serde_json::to_string(&bundle).expect("bundle serializes");
    let model = KnowledgeModel::default();
    let mut native = model.init(&Profile::default());
    let families = refl_core::model::Families::of(&bundle);
    let ctx = refl_core::model::FoldCtx::new(&bundle, &families);
    let mut native_us = Vec::with_capacity(log.len());
    let native_start = Instant::now();
    for event in &log {
        let t = Instant::now();
        model
            .apply(&mut native, event, &ctx)
            .expect("events are folded in order");
        native_us.push(t.elapsed().as_micros() as u64);
    }
    let native_total = native_start.elapsed().as_secs_f64() * 1000.0;

    let mut snapshot = refl_core_wasm::fold_state(&bundle_json, "", None, "[]");
    let bridge_start = Instant::now();
    let mut bridge_us = Vec::with_capacity(log.len());
    let mut final_hash = String::new();
    for event in &log {
        let Decoded::Known(event) = event else {
            continue;
        };
        let event_json = serde_json::to_string(event).expect("event serializes");
        let t = Instant::now();
        let result = refl_core_wasm::fold_state(
            &bundle_json,
            "",
            Some(snapshot),
            &format!("[{event_json}]"),
        );
        let value: serde_json::Value = serde_json::from_str(&result).expect("bridge returns JSON");
        final_hash = value["state_hash"]
            .as_str()
            .expect("a state hash")
            .to_owned();
        snapshot = value["snapshot"].to_string();
        bridge_us.push(t.elapsed().as_micros() as u64);
    }
    let bridge_total = bridge_start.elapsed().as_secs_f64() * 1000.0;
    native_us.sort_unstable();
    bridge_us.sort_unstable();
    let p = |v: &[u64], q: f64| -> u64 {
        if v.is_empty() {
            0
        } else {
            let r = (q * v.len() as f64).ceil() as usize;
            v[r.saturating_sub(1).min(v.len() - 1)]
        }
    };
    println!(
        "native 10k: total_ms={native_total:.3} ops_per_sec={:.0} p50_us={} p95_us={} p99_us={} max_us={}",
        log.len() as f64 * 1000.0 / native_total,
        p(&native_us, 0.50),
        p(&native_us, 0.95),
        p(&native_us, 0.99),
        native_us.last().copied().unwrap_or(0)
    );
    println!(
        "core-wasm 10k: total_ms={bridge_total:.3} ops_per_sec={:.0} p50_us={} p95_us={} p99_us={} max_us={} snapshot_bytes={}",
        log.len() as f64 * 1000.0 / bridge_total,
        p(&bridge_us, 0.50),
        p(&bridge_us, 0.95),
        p(&bridge_us, 0.99),
        bridge_us.last().copied().unwrap_or(0),
        snapshot.len()
    );
    assert_eq!(final_hash, native.hash());
    let snapshot_value: serde_json::Value =
        serde_json::from_str(&snapshot).expect("final snapshot is JSON");
    let bridge_state: State =
        serde_json::from_value(snapshot_value).expect("final snapshot is a State");
    assert_eq!(
        bridge_state.canonical_bytes(),
        native.canonical_bytes(),
        "the bridge State differs from the native State"
    );
    assert_eq!(bridge_us.len(), log.len());
}
