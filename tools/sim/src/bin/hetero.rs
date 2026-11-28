//! W24 heterogeneous cohort report.
//!
//! `hetero [LIVE_BUNDLE] [FULL_BUNDLE] [LEARNERS] [STEPS] [SEED]`
//!
//! The live bundle is the frozen Articles unit; the full bundle is
//! `target/bundle.json` from `just content`, with its 27 draft KCs and
//! starter items promoted to live for this test fixture only.

use std::process::ExitCode;

use refl_core::bundle::{Bundle, KcStatus};
use refl_sim::hetero::{Remediation, RunSpec, promote, run};

fn arg<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn load(path: &str) -> Result<Bundle, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn main() -> ExitCode {
    let live_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "content/bench/v3/bundle.json".to_owned());
    let full_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "target/bundle.json".to_owned());
    let learners: usize = arg(3, 1_000);
    let steps: usize = arg(4, 200);
    let seed: u64 = arg(5, 1);
    let remediation = if std::env::args().nth(6).as_deref() == Some("s3-only") {
        Remediation::FullOnly
    } else {
        Remediation::AllStrategies
    };

    let live = match load(&live_path) {
        Ok(bundle) => bundle,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let mut full = match load(&full_path) {
        Ok(bundle) => bundle,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let live_live = live
        .content
        .kcs
        .iter()
        .filter(|e| e.status == KcStatus::Live && !e.kc.observed_only)
        .count();
    println!(
        "hetero seed={seed} learners={learners} steps={steps} live_bundle={live_path} ({live_live} live KCs) full_bundle={full_path} ({} learning KCs)",
        full.content
            .kcs
            .iter()
            .filter(|e| !e.kc.observed_only)
            .count()
    );
    promote(&mut full);
    let spec = RunSpec {
        learners,
        steps,
        seed,
        remediation,
    };
    let live_report = run(&live, "live Articles sub-graph", spec);
    print!("{}", live_report.report());
    let full_report = run(
        &full,
        "full 32-KC A1 projection (draft content promoted)",
        spec,
    );
    print!("{}", full_report.report());
    ExitCode::SUCCESS
}
