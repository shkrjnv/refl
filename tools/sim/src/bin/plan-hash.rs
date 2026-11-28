//! Plans three daily sessions for a synthetic learner on top of a folded
//! synthetic log and prints a hash of every `plan_issued` and the last plan.
//!
//! `plan-hash [SEED]` prints `sessions=… decisions=… violations=… hash=…`;
//! the same line from the native build and from the wasm32-wasip1 build
//! under wasmtime proves the planner bit-identical (`just wasm-hash`).
//! `plan-hash --time` also reports how long `build_session` and `next_item`
//! take (the clock is the tool's, never the core's).

use std::fmt::Write as _;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use refl_core::model::{KnowledgeModel, Profile, replay};
use refl_core::plan::{Budget, History, Next, Params, Planner, ReadingSpeed};
use refl_core::value::{Lang, Seed, Timestamp};
use refl_sim::cohort::{Learner, Track, session_ours};
use refl_sim::prereq;
use refl_sim::termination;
use refl_sim::{BundleSpec, DAY_MS, LogSpec, event_time, synthetic_bundle, synthetic_log};
use sha2::{Digest, Sha256};

const DEFAULT_SEED: u64 = 11;
const LOG_EVENTS: usize = 20_000;

fn best_of(runs: usize, mut f: impl FnMut()) -> Duration {
    (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .unwrap_or_default()
}

fn time(kcs: usize, items_per_kc: usize) {
    let bundle = synthetic_bundle(BundleSpec { kcs, items_per_kc });
    let log = synthetic_log(
        &bundle,
        LogSpec {
            events: LOG_EVENTS,
            seed: 7,
        },
    );
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading = ReadingSpeed::from_log(&bundle, &params, &log);
    let planner = Planner::new(&model, &bundle, &params, &reading);
    let state = replay(&model, &Profile::default(), &bundle, &log);
    let history = History::from_log(&bundle, &log);
    let now = Timestamp(event_time(&log) + DAY_MS);
    let seed = Seed::new(1).expect("small");
    let build_plan =
        || planner.build_session(&state, &history, now, Budget::TwentyFive, Lang::Ru, seed);
    let build = best_of(20, || {
        std::hint::black_box(build_plan());
    });
    let plan = build_plan().plan;
    let calls = 200u32;
    let next = best_of(5, || {
        for i in 0..calls {
            let choice_seed = Seed::new(u64::from(i)).expect("small");
            let n = planner.next_item(&state, &plan, &history, now, choice_seed);
            std::hint::black_box(matches!(n, Next::Item(_)));
        }
    }) / calls;
    eprintln!(
        "{kcs} KCs × {items_per_kc} items, state of {LOG_EVENTS} events: build_session {:.3} ms, next_item {:.3} ms ({} steps)",
        build.as_secs_f64() * 1e3,
        next.as_secs_f64() * 1e3,
        plan.steps.len()
    );
}

#[allow(clippy::too_many_lines)] // one report with hash and Rule 3 scenarios
fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let timing = args.first().is_some_and(|a| a == "--time");
    if timing {
        args.remove(0);
    }
    let Some(seed) = args.first().map_or(Some(DEFAULT_SEED), |a| a.parse().ok()) else {
        eprintln!("usage: plan-hash [--time] [SEED]");
        return ExitCode::FAILURE;
    };

    let bundle = synthetic_bundle(BundleSpec {
        kcs: 40,
        items_per_kc: 12,
    });
    let log = synthetic_log(
        &bundle,
        LogSpec {
            events: LOG_EVENTS,
            seed: 7,
        },
    );
    let model = KnowledgeModel::default();
    let params = Params::default();
    let start = Timestamp(event_time(&log) + DAY_MS);
    let mut track = Track::new(&model, &bundle, start, seed);
    track.state = replay(&model, &Profile::default(), &bundle, &log);
    track.history = History::from_log(&bundle, &log);
    let mut learner = Learner::new(&bundle, seed);

    let mut hasher = Sha256::new();
    let (mut decisions, mut violations) = (0usize, 0u32);
    let sessions = 3;
    for day in 0..sessions {
        track.clock = track.clock.max(start.0 + day * DAY_MS);
        let r = session_ours(&mut track, &params, &mut learner, Budget::TwentyFive);
        decisions += r.issued.len();
        violations += r.violations.values().sum::<u32>();
        for ev in &r.issued {
            hasher.update(serde_json::to_vec(ev).expect("events serialize"));
        }
        hasher.update(serde_json::to_vec(&r.plan).expect("plans serialize"));
    }
    let hash = hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    if timing {
        time(40, 12);
        time(200, 15);
    }
    let reading = track.speed(&params);
    println!(
        "sessions={sessions} decisions={decisions} violations={violations} planner={} state={} hash={hash}",
        Planner::new(&model, &bundle, &params, &reading).version(),
        track.state.hash()
    );

    let scenario_a = match prereq::scenario_a() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("prerequisite scenario A: {e}");
            return ExitCode::FAILURE;
        }
    };
    let scenario_b = match prereq::scenario_b() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("prerequisite scenario B: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "prereq_a rule={} selected={:?} trigger={:?} prerequisite_steps={} subsequent_steps={} final_steps={} end={:?} continued={} post_prerequisite_steps={} post_prerequisite_prereq_steps={}",
        scenario_a.rule,
        scenario_a.selected,
        scenario_a.trigger,
        scenario_a.prerequisite_steps,
        scenario_a.subsequent_steps,
        scenario_a.final_steps,
        scenario_a.end,
        scenario_a.continued,
        scenario_a.post_prerequisite_steps,
        scenario_a.post_prerequisite_prereq_steps
    );
    println!(
        "prereq_b rule={} selected={:?} trigger={:?} prerequisite_steps={} subsequent_steps={} final_steps={} end={:?} continued={} post_prerequisite_steps={} post_prerequisite_prereq_steps={}",
        scenario_b.rule,
        scenario_b.selected,
        scenario_b.trigger,
        scenario_b.prerequisite_steps,
        scenario_b.subsequent_steps,
        scenario_b.final_steps,
        scenario_b.end,
        scenario_b.continued,
        scenario_b.post_prerequisite_steps,
        scenario_b.post_prerequisite_prereq_steps
    );

    let terminal = termination::scenario_terminal();
    let alternative = termination::scenario_review_alternative();
    println!(
        "runlimit_terminal end={:?} steps={} review_steps={} final_steps={}",
        terminal.end, terminal.steps, terminal.review_steps, terminal.final_steps
    );
    println!(
        "runlimit_alternative end={:?} steps={} review_steps={} final_steps={}",
        alternative.end, alternative.steps, alternative.review_steps, alternative.final_steps
    );

    let rule_failed = scenario_a.rule != "prerequisite_check"
        || scenario_a.selected != Some(prereq::expected_selected())
        || scenario_a.trigger.as_deref() != Some("prerequisite_check")
        || scenario_a.prerequisite_steps != 1
        || scenario_a.subsequent_steps == 0
        || !scenario_a.continued
        || scenario_a.post_prerequisite_steps == 0
        || scenario_a.post_prerequisite_prereq_steps != 0
        || scenario_b.rule != "prerequisite_check"
        || scenario_b.selected != Some(prereq::expected_selected())
        || scenario_b.trigger.as_deref() != Some("prerequisite_check")
        || scenario_b.prerequisite_steps != 0
        || scenario_b.final_steps == 0
        || scenario_b.end != prereq::expected_b_end().map(|e| e.code().to_string())
        || terminal.end.as_deref() != Some("run_limit")
        || terminal.steps != 0
        || alternative.end.is_some()
        || alternative.review_steps == 0;
    if rule_failed {
        eprintln!("prerequisite scenario assertions failed");
        return ExitCode::FAILURE;
    }
    if violations > 0 {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
