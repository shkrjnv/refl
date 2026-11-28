//! Simulator v2 report: a 30-day horizon (600 steps) next to the 200-step
//! stress run, with per-learner cluster bootstrap intervals
//! (`docs/11-quality.md`, «Версии симулятора», «Статистика пилота»).
//!
#![allow(clippy::cast_precision_loss)]

//! `v2 [LIVE_BUNDLE] [FULL_BUNDLE] [LEARNERS] [SEED] [all|s3-only]`
//!
//! The fifth argument says who gets the micro-explanation after a wrong
//! answer on a targeted pattern (`Remediation`). `all` is the default and the
//! fair comparison; `s3-only` reproduces the first published v2 report.

use std::fmt::Write as _;
use std::process::ExitCode;

use refl_core::bundle::{Bundle, KcStatus};
use refl_sim::DAY_MS;
use refl_sim::hetero::{
    BOOTSTRAP_RESAMPLES, BundleRun, Outcome, Remediation, RunSpec, Strategy, bootstrap_mean,
    promote, run,
};

fn load(path: &str) -> Result<Bundle, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn arg<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn revision() -> String {
    let output = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    let Ok(output) = output else {
        return "unknown".into();
    };
    if !output.status.success() {
        return "unknown".into();
    }
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn reach_values(outcomes: &[Outcome]) -> Vec<f64> {
    outcomes
        .iter()
        .filter(|outcome| !outcome.to_learn.is_empty())
        .map(|outcome| {
            let reached = outcome
                .to_learn
                .iter()
                .filter(|kc| outcome.mastery_step.contains_key(*kc))
                .count();
            reached as f64 / outcome.to_learn.len() as f64
        })
        .collect()
}

fn level_values(outcomes: &[Outcome]) -> Vec<f64> {
    outcomes
        .iter()
        .map(|outcome| (outcome.model_level - outcome.true_level).abs())
        .collect()
}

fn misconception_values(outcomes: &[Outcome]) -> Vec<f64> {
    outcomes
        .iter()
        .filter(|outcome| outcome.active_initial > 0)
        .map(|outcome| {
            let left = outcome.active_at[3];
            (outcome.active_initial.saturating_sub(left)) as f64 / outcome.active_initial as f64
        })
        .collect()
}

fn interval(value: refl_sim::hetero::BootstrapMean) -> String {
    let Some(low80) = value.low80 else {
        return format!("{:.3} (n={}, no interval <6)", value.point, value.learners);
    };
    format!(
        "{:.3} [80% {:.3}-{:.3}; 95% {:.3}-{:.3}; n={}]",
        value.point,
        low80,
        value.high80.unwrap_or(f64::NAN),
        value.low95.unwrap_or(f64::NAN),
        value.high95.unwrap_or(f64::NAN),
        value.learners
    )
}

fn median_u32(values: &mut [u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn kc_line(bundle: &Bundle, run: &BundleRun, kc: &refl_core::ids::KcId) -> String {
    let mut cells = Vec::new();
    for strategy_run in &run.strategies {
        let mut reached = 0usize;
        let mut eligible = 0usize;
        let mut steps = Vec::new();
        for outcome in &strategy_run.outcomes {
            if outcome.to_learn.contains(kc) {
                eligible += 1;
                if let Some(step) = outcome.mastery_step.get(kc) {
                    reached += 1;
                    steps.push(*step);
                }
            }
        }
        let rate = if eligible == 0 {
            0.0
        } else {
            100.0 * reached as f64 / eligible as f64
        };
        let median = median_u32(&mut steps).unwrap_or(0);
        cells.push(format!("{rate:.0}%/{median}"));
    }
    let edges: Vec<String> = bundle
        .content
        .edges
        .iter()
        .filter(|edge| &edge.from == kc || &edge.to == kc)
        .map(|edge| {
            format!(
                "{:?} {}->{}",
                edge.kind,
                edge.from.as_str(),
                edge.to.as_str()
            )
        })
        .collect();
    let labels: Vec<&str> = run
        .strategies
        .iter()
        .map(|r| r.strategy.name().split(' ').next().unwrap_or("?"))
        .collect();
    format!(
        "{}: bank {}/{} slots/families; {} reach%/median-step {}; edges: {}",
        kc.as_str(),
        run.bank.get(kc).copied().unwrap_or((0, 0)).0,
        run.bank.get(kc).copied().unwrap_or((0, 0)).1,
        labels.join("/"),
        cells.join(", "),
        if edges.is_empty() {
            "none".into()
        } else {
            edges.join("; ")
        }
    )
}

fn report_bundle(out: &mut String, run: &BundleRun, seed: u64, bundle: &Bundle) {
    let _ = writeln!(
        out,
        "\n== {}: {} learners x {} steps (target KCs {}) ==",
        run.label,
        run.learners,
        run.steps,
        run.target_kcs.len()
    );
    for strategy_run in &run.strategies {
        let strategy = strategy_run.strategy;
        let strategy_seed = seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(strategy as u64);
        let _ = writeln!(out, "\n-- {} --", strategy.name());
        let reach = bootstrap_mean(
            &reach_values(&strategy_run.outcomes),
            strategy_seed ^ 0x51A7,
            BOOTSTRAP_RESAMPLES,
        );
        let level = bootstrap_mean(
            &level_values(&strategy_run.outcomes),
            strategy_seed ^ 0x1E7E1,
            BOOTSTRAP_RESAMPLES,
        );
        let mis = bootstrap_mean(
            &misconception_values(&strategy_run.outcomes),
            strategy_seed ^ 0x315C,
            BOOTSTRAP_RESAMPLES,
        );
        if let Some(value) = reach {
            let _ = writeln!(out, "target reach: {}", interval(value));
        }
        if let Some(value) = level {
            let _ = writeln!(out, "level MAE:    {}", interval(value));
        }
        if let Some(value) = mis {
            let _ = writeln!(out, "mis reduction: {}", interval(value));
        }
        let strategy_steps: Vec<u32> = strategy_run
            .outcomes
            .iter()
            .flat_map(|outcome| outcome.mastery_step.values().copied())
            .collect();
        if !strategy_steps.is_empty() {
            let mean = strategy_steps.iter().map(|n| f64::from(*n)).sum::<f64>()
                / strategy_steps.len() as f64;
            let _ = writeln!(
                out,
                "reached KC-step mean: {mean:.1} over {} reached KCs",
                strategy_steps.len()
            );
        }
    }
    let kc = refl_core::ids::KcId::new("en.gram.article.a-an-sound").expect("valid KC");
    let _ = writeln!(out, "\n{}", kc_line(bundle, run, &kc));
}

fn main() -> ExitCode {
    let live_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "content/bench/v3/bundle.json".to_owned());
    let full_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "target/bundle.json".to_owned());
    let learners: usize = arg(3, 1_000);
    let seed: u64 = arg(4, 1);
    let remediation = match std::env::args().nth(5).as_deref() {
        None | Some("all") => Remediation::AllStrategies,
        Some("s3-only") => Remediation::FullOnly,
        Some(other) => {
            eprintln!("unknown remediation mode {other}; use all or s3-only");
            return ExitCode::FAILURE;
        }
    };

    let Ok(live) = load(&live_path) else {
        eprintln!("cannot load {live_path}");
        return ExitCode::from(1);
    };
    let Ok(mut full) = load(&full_path) else {
        eprintln!("cannot load {full_path}");
        return ExitCode::from(1);
    };
    promote(&mut full);

    let mut out = String::new();
    let _ = writeln!(out, "simulator v2");
    let _ = writeln!(out, "revision: {}", revision());
    let _ = writeln!(
        out,
        "seed={seed} learners={learners} horizon=600 steps (~{} days at 20 steps/day) + stress=200",
        600 / 20
    );
    let _ = writeln!(
        out,
        "bootstrap: learner-cluster, SplitMix64, B={BOOTSTRAP_RESAMPLES}; 80% and 95% percentiles"
    );
    let _ = writeln!(
        out,
        "live={live_path} ({} live KCs); full={full_path} ({} learning KCs)",
        live.content
            .kcs
            .iter()
            .filter(|e| e.status == KcStatus::Live && !e.kc.observed_only)
            .count(),
        full.content
            .kcs
            .iter()
            .filter(|e| !e.kc.observed_only)
            .count()
    );
    let _ = writeln!(out, "day_ms={DAY_MS}; mastery={MASTERY_LABEL}");
    let _ = writeln!(
        out,
        "micro-explanation remediation: {}",
        remediation.label()
    );
    for steps in [200, 600] {
        let spec = RunSpec {
            learners,
            steps,
            seed,
            remediation,
            strategies: if remediation == Remediation::FullOnly {
                &Strategy::ALL
            } else {
                &Strategy::WITH_PRODUCT
            },
        };
        report_bundle(
            &mut out,
            &run(&live, "live Articles sub-graph", spec),
            seed.wrapping_add(steps as u64),
            &live,
        );
        // The product planner is left out on the draft fixture: its KCs have
        // two items each, below the planner's block of four with an input
        // step (docs/04), so every session would end at once. The live gate
        // requires ten items per KC, so the product never meets such a KC.
        let full_spec = RunSpec {
            strategies: &Strategy::ALL,
            ..spec
        };
        report_bundle(
            &mut out,
            &run(
                &full,
                "full 32-KC A1 projection (draft content promoted)",
                full_spec,
            ),
            seed.wrapping_add(steps as u64).wrapping_add(1),
            &full,
        );
    }
    print!("{out}");
    ExitCode::SUCCESS
}

const MASTERY_LABEL: &str = "0.85 (simulator reach), 0.80 model criterion";
