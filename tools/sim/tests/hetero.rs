//! W24 heterogeneous cohort. The cheap live-bundle smoke runs in the default
//! suite; the full 1,000 × 200 two-bundle report is `#[ignore]`:
//!
//! `cargo test --release -p refl-sim --test hetero -- --ignored --nocapture`

use std::path::Path;

use refl_core::bundle::Bundle;
use refl_sim::hetero::{BundleRun, Remediation, RunSpec, Strategy, bootstrap_mean, promote, run};

fn load(path: &Path) -> Bundle {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn live_bundle() -> Bundle {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bench/v3/bundle.json");
    load(&path)
}

fn target_bundle() -> Bundle {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/bundle.json");
    load(&path)
}

fn reach(run: &BundleRun, strategy: Strategy) -> (usize, usize) {
    let outcomes = &run
        .strategies
        .iter()
        .find(|s| s.strategy == strategy)
        .expect("strategy ran")
        .outcomes;
    let total = outcomes.iter().map(|o| o.to_learn.len()).sum();
    let reached = outcomes
        .iter()
        .map(|o| {
            o.to_learn
                .iter()
                .filter(|kc| o.mastery_step.contains_key(*kc))
                .count()
        })
        .sum();
    (reached, total)
}

#[test]
fn live_cohort_is_deterministic_and_refl_beats_the_baseline() {
    let bundle = live_bundle();
    let spec = RunSpec {
        learners: 12,
        steps: 200,
        seed: 1,
        remediation: Remediation::FullOnly,
        strategies: &Strategy::ALL,
    };
    let first = run(&bundle, "live smoke", spec);
    assert_eq!(first.strategies.len(), 3);
    assert!(first.strategies.iter().all(|s| s.outcomes.len() == 12));

    let (s1, total) = reach(&first, Strategy::Leitner);
    let (s2, _) = reach(&first, Strategy::KcAdaptive);
    let (s3, _) = reach(&first, Strategy::Full);
    assert!(total > 0);
    assert!(s3 > s1, "S3 {s3} did not beat S1 {s1}");
    assert!(s3 > s2, "S3 {s3} did not beat S2 {s2}");
    assert!(s3 > 0);

    let full = first
        .strategies
        .iter()
        .find(|s| s.strategy == Strategy::Full)
        .expect("S3 ran");
    assert!(
        full.outcomes
            .iter()
            .all(|o| o.active_at[3] <= o.active_initial),
        "S3 increased an active misconception count"
    );
    assert!(
        full.outcomes
            .iter()
            .any(|o| o.active_at[3] < o.active_initial),
        "S3 cured no planted L1 misconception"
    );

    let second = run(&bundle, "live smoke", spec);
    assert_eq!(
        first.report(),
        second.report(),
        "the cohort is not deterministic"
    );
}

#[test]
#[ignore = "W24: 1,000 learners x 200 steps on the live and full 32-KC bundles"]
fn full_heterogeneous_cohort() {
    let live = live_bundle();
    let mut full = target_bundle();
    promote(&mut full);
    let spec = RunSpec {
        learners: 1_000,
        steps: 200,
        seed: 1,
        remediation: Remediation::FullOnly,
        strategies: &Strategy::ALL,
    };
    let live_report = run(&live, "live Articles sub-graph", spec);
    let full_report = run(
        &full,
        "full 32-KC A1 projection (draft content promoted)",
        spec,
    );
    println!("{}", live_report.report());
    println!("{}", full_report.report());

    let (s1, _) = reach(&live_report, Strategy::Leitner);
    let (s3, _) = reach(&live_report, Strategy::Full);
    assert!(s3 > s1, "S3 {s3} did not beat S1 {s1} on the live bundle");
    let (s1_full, _) = reach(&full_report, Strategy::Leitner);
    let (s3_full, _) = reach(&full_report, Strategy::Full);
    assert!(
        s3_full > s1_full,
        "S3 {s3_full} did not beat S1 {s1_full} on the full bundle"
    );
}

#[test]
fn cluster_bootstrap_is_deterministic_and_suppresses_tiny_samples() {
    let values = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let first = bootstrap_mean(&values, 17, 200).expect("values");
    let second = bootstrap_mean(&values, 17, 200).expect("values");
    assert_eq!(first, second);
    assert_eq!(first.learners, 8);
    assert_eq!(first.resamples, 200);
    assert!(first.low80.unwrap() <= first.point);
    assert!(first.high80.unwrap() >= first.point);
    let tiny = bootstrap_mean(&values[..5], 17, 200).expect("values");
    assert!(tiny.low80.is_none());
}
