//! Folds a synthetic log with learner model v1 and prints the state hash.
//!
//! `fold-hash [EVENTS] [SEED]` prints `events=… model=… hash=…`. The same
//! line from the native build and from the wasm32-wasip1 build under
//! wasmtime proves the fold bit-identical (`just wasm-hash`).
//! `fold-hash --time [EVENTS]` also reports how long the fold takes
//! (native only: the clock is the tool's, never the core's).

use std::process::ExitCode;
use std::time::Instant;

use refl_core::model::{KnowledgeModel, LearnerModel, Profile, replay};
use refl_sim::{BundleSpec, LogSpec, synthetic_bundle, synthetic_log};

const DEFAULT_EVENTS: usize = 20_000;
const DEFAULT_SEED: u64 = 7;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let time = args.first().is_some_and(|a| a == "--time");
    if time {
        args.remove(0);
    }
    let parse = |i: usize, default: u64| -> Option<u64> {
        args.get(i).map_or(Some(default), |a| a.parse().ok())
    };
    let (Some(events), Some(seed)) = (parse(0, DEFAULT_EVENTS as u64), parse(1, DEFAULT_SEED))
    else {
        eprintln!("usage: fold-hash [--time] [EVENTS] [SEED]");
        return ExitCode::FAILURE;
    };
    let events = usize::try_from(events).expect("event count fits usize");

    let bundle = synthetic_bundle(BundleSpec {
        kcs: 40,
        items_per_kc: 12,
    });
    let log = synthetic_log(&bundle, LogSpec { events, seed });
    let model = KnowledgeModel::default();
    let profile = Profile::default();

    let start = time.then(Instant::now);
    let state = replay(&model, &profile, &bundle, &log);
    if let Some(start) = start {
        // Best of three, so a cold cache does not count.
        let mut best = start.elapsed();
        for _ in 0..2 {
            let t = Instant::now();
            std::hint::black_box(replay(&model, &profile, &bundle, &log));
            best = best.min(t.elapsed());
        }
        eprintln!(
            "fold of {events} events: {:.1} ms ({:.2} µs/event)",
            best.as_secs_f64() * 1e3,
            best.as_secs_f64() * 1e6 / f64::from(u32::try_from(events).unwrap_or(u32::MAX)),
        );
    }
    println!(
        "events={events} model={} fold={} hash={}",
        model.version(),
        refl_core::model::fold_version(&bundle),
        state.hash()
    );
    ExitCode::SUCCESS
}
