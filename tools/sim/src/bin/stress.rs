//! W24 stress runner, usable natively and as wasm32-wasip1 under wasmtime.
//!
//! `stress fold|replay|select|all [EVENTS] [SEED]`
//!
//! It uses the synthetic 32-KC A1-shaped bundle so the same log is generated
//! bit-identically on native and WASM; the printed state hash proves it.

// A measurement binary: counters and timings are allowed to lose precision.
#![allow(clippy::cast_precision_loss)]

use std::process::ExitCode;
use std::time::Instant;

use refl_core::model::{KnowledgeModel, Profile, replay};
use refl_sim::stress::{
    events, fold_native, peak_rss_kb, placement_selection, plan_selection, replay_once,
    replay_threshold, snapshot_bytes, state_after, stress_bundle,
};

fn args() -> (String, usize, u64) {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "all".to_owned());
    let count = args.next().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(7);
    (mode, count, seed)
}

fn fold(bundle: &refl_core::bundle::Bundle, count: usize) {
    let log = events(bundle, count);
    let (state, latency, start_bytes, end_bytes) = fold_native(bundle, &log);
    println!("{}", latency.line("fold"));
    println!(
        "fold hash={} events={} snapshot_bytes={}->{} peak_rss_kb={}",
        state.hash(),
        count,
        start_bytes,
        end_bytes,
        peak_rss_kb()
    );
}

fn replay_curve(bundle: &refl_core::bundle::Bundle, count: usize) {
    let log = events(bundle, count);
    for n in [100, 1_000, 10_000, count] {
        if n > log.len() {
            continue;
        }
        let (state, ms) = replay_once(bundle, &log[..n]);
        println!(
            "replay n={n} ms={ms:.3} hash={} snapshot_bytes={}",
            state.hash(),
            snapshot_bytes(&state)
        );
    }
    for target in [16.0, 50.0] {
        match replay_threshold(bundle, &log, target) {
            Some((n, ms)) => println!("threshold>{target:.0}ms n={n} ms={ms:.3}"),
            None => println!("threshold>{target:.0}ms not_reached"),
        }
    }
}

fn select(bundle: &refl_core::bundle::Bundle, count: usize, seed: u64) {
    let log = events(bundle, 2_000);
    let state = state_after(bundle, &log, 1_000);
    let planner = plan_selection(bundle, &state, count);
    println!("{}", planner.line("planner_next_item"));
    let placement = placement_selection(bundle, &state, count);
    println!("{}", placement.line("placement_next"));
    println!(
        "select seed={seed} kcs={} items={}",
        bundle.content.kcs.len(),
        bundle.content.items.len()
    );
}

fn main() -> ExitCode {
    let (mode, count, seed) = args();
    let bundle = stress_bundle();
    println!(
        "core={} mode={mode} events={count} seed={seed} kcs={} items={}",
        sim_version(),
        bundle.content.kcs.len(),
        bundle.content.items.len()
    );
    match mode.as_str() {
        "fold" => fold(&bundle, count),
        "replay" => replay_curve(&bundle, count),
        "select" => select(&bundle, count, seed),
        "all" => {
            let started = Instant::now();
            fold(&bundle, count);
            replay_curve(&bundle, count);
            select(&bundle, count.min(10_000), seed);
            let state = replay(
                &KnowledgeModel::default(),
                &Profile::default(),
                &bundle,
                &events(&bundle, count),
            );
            println!(
                "all hash={} wall_ms={:.3}",
                state.hash(),
                started.elapsed().as_secs_f64() * 1000.0
            );
        }
        other => {
            eprintln!("unknown mode {other:?}; use fold, replay, select or all");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn sim_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
