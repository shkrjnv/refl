//! Runs the K5 cohort (`docs/11-quality.md`) on a content bundle under our
//! session builder and the two baselines, «только FSRS» and «только FSRS +
//! наш выбор задания», and prints the three reports.
//!
//! `cohort [BUNDLE] [LEARNERS] [DAYS] [SEED]`; the bundle defaults to
//! `target/bundle.json` (`just content`). Draft KCs are treated as live:
//! this is a simulation fixture, not what learners get.

use std::process::ExitCode;

use refl_core::bundle::Bundle;
use refl_core::model::KnowledgeModel;
use refl_core::plan::{Budget, Params};
use refl_sim::cohort::{CohortReport, CohortSpec, DAY_GROUPS, Policy, cohort};
use refl_sim::live_fixture;

#[allow(clippy::cast_precision_loss)] // small counts
fn mean(v: &[usize]) -> f64 {
    v.iter().sum::<usize>() as f64 / v.len().max(1) as f64
}

/// The `q`-quantile of sorted values, nearest rank.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // small counts
fn quantile(sorted: &[u32], q: f64) -> u32 {
    let i = ((q * sorted.len() as f64).ceil() as usize).max(1) - 1;
    sorted[i.min(sorted.len() - 1)]
}

/// Session minutes of a day group: quartiles and mean; the share of each
/// end reason.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // small counts
fn print_lengths(r: &CohortReport) {
    for (g, name) in DAY_GROUPS.iter().enumerate() {
        let mut m = r.minutes[g].clone();
        if m.is_empty() {
            continue;
        }
        m.sort_by(f64::total_cmp);
        let q = |p: f64| m[((p * m.len() as f64).ceil() as usize).max(1) - 1];
        let n = m.len() as f64;
        println!(
            "session minutes, {name}: median {:.1} [p25 {:.1} - p75 {:.1}], mean {:.1}",
            q(0.5),
            q(0.25),
            q(0.75),
            m.iter().sum::<f64>() / n
        );
        let ends: Vec<String> = r.ends[g]
            .iter()
            .map(|(e, c)| format!("{e} {:.0}%", 100.0 * f64::from(*c) / n))
            .collect();
        println!("  ended by: {}", ends.join(", "));
    }
}

#[allow(clippy::cast_precision_loss)] // small counts
fn print(r: &CohortReport) {
    let name = r.policy.name();
    let learners = r.learners as f64;
    println!(
        "== {name}: {} learners, {} sessions",
        r.learners, r.sessions
    );
    println!(
        "answers per learner {:.0}; minutes per session {:.1}",
        f64::from(r.answers) / learners,
        r.seconds / 60.0 / f64::from(r.sessions.max(1))
    );
    println!(
        "validator: {} checks, violations {:?}",
        r.checks, r.violations
    );
    println!("answers to mastery per KC: median [p25-p75] (mastered/learners)");
    for (kc, v) in &r.to_mastery {
        let mut v = v.clone();
        v.sort_unstable();
        println!(
            "  {kc}: {} [{}-{}] ({}/{})",
            quantile(&v, 0.5),
            quantile(&v, 0.25),
            quantile(&v, 0.75),
            v.len(),
            r.learners
        );
    }
    println!(
        "KCs mastered per learner: ever {:.2}, at the end {:.2}",
        mean(&r.mastered),
        mean(&r.mastered_now)
    );
    println!(
        "KCs truly known at the end (recall >= 0.8): {:.2}",
        mean(&r.known)
    );
    let retention = r.retention.iter().sum::<f64>() / r.retention.len().max(1) as f64;
    println!(
        "retention at the end (true recall of mastered KCs): {retention:.3} over {} pairs",
        r.retention.len()
    );
    println!(
        "frustrated sessions: {:.1}%",
        100.0 * f64::from(r.frustrated) / f64::from(r.sessions.max(1))
    );
    if !r.rules.is_empty() {
        println!("adaptation rules fired: {:?}", r.rules);
    }
    print_lengths(r);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().map_or("target/bundle.json", String::as_str);
    let num = |i: usize, default: u64| -> Option<u64> {
        args.get(i).map_or(Some(default), |a| a.parse().ok())
    };
    let (Some(learners), Some(days), Some(seed)) = (num(1, 200), num(2, 30), num(3, 1)) else {
        eprintln!("usage: cohort [BUNDLE] [LEARNERS] [DAYS] [SEED]");
        return ExitCode::FAILURE;
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e} (run `just content` first)");
            return ExitCode::FAILURE;
        }
    };
    let bundle: Bundle = match serde_json::from_slice(&bytes) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let bundle = live_fixture(bundle);
    let spec = CohortSpec {
        learners: usize::try_from(learners).expect("fits"),
        days: u32::try_from(days).expect("fits"),
        seed,
        budget: Budget::TwentyFive,
    };
    let model = KnowledgeModel::default();
    let params = Params::default();
    for policy in [Policy::Ours, Policy::FsrsOnly, Policy::FsrsNextItem] {
        print(&cohort(&model, &bundle, &params, spec, policy));
    }
    ExitCode::SUCCESS
}
