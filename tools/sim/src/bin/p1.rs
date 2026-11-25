//! P1: practice volume of the session builder, measured on the K5 cohort
//! (`tools/sim/bench/p1/README.md`). Runs our sessions and the «только FSRS»
//! baseline on the same learners and prints, per arm: minutes and answers
//! per session, the share of sessions per end reason, topics mastered per
//! learner, answers and calendar days to mastery, false mastery and the
//! truly known topics gained per active hour. It uses only the public
//! cohort API, so the same file builds at an older commit (the «before» run).
//!
//! `p1 [BUNDLE] [LEARNERS] [DAYS] [SEED]`; the bundle defaults to the frozen
//! `content/bench/v2/bundle.json`.
//!
//! - **False mastery**: a topic the model counted mastered whose true
//!   correct rate 7 days later, without reviews, is below 0.7. The true rate
//!   is the learner's own answer formula (no rapid guesses, fatigue or
//!   learning) averaged over the topic's items, from the learner as they
//!   were at the end of the session that mastered it.
//! - **Calendar days to mastery**: from the day of the topic's first answer
//!   to the day of the session that mastered it.
//! - **Truly known**: topics with a true recall of at least 0.8
//!   (`Learner::recall`), at the start and at the end.

// A measurement binary: counts are small.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::similar_names
)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use refl_core::bundle::Bundle;
use refl_core::ids::KcId;
use refl_core::item::KcRole;
use refl_core::model::{KnowledgeModel, LearnerModel};
use refl_core::plan::{Budget, Params, Planner, ReadingSpeed};
use refl_core::rng::SplitMix64;
use refl_core::value::Timestamp;
use refl_sim::cohort::{Learner, SessionReport, Track, session_fsrs, session_ours};
use refl_sim::{DAY_MS, T0, live_fixture};

/// Sessions of days 1–5 and of days 6 on.
const GROUPS: [&str; 2] = ["days 1-5", "days 6-30"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    Ours,
    FsrsOnly,
}

/// A topic the model counted mastered, with the learner as they were then.
struct Mastered {
    learner: Learner,
    kc: KcId,
    at: Timestamp,
    answers: u32,
    days: u32,
}

#[derive(Default)]
struct Totals {
    sessions: u32,
    seconds: f64,
    answers: [Vec<f64>; 2],
    minutes: [Vec<f64>; 2],
    ends: [BTreeMap<&'static str, u32>; 2],
    violations: BTreeMap<String, u32>,
    mastered_ever: Vec<usize>,
    mastered_now: Vec<usize>,
    known_start: Vec<usize>,
    known_end: Vec<usize>,
    pairs: Vec<Mastered>,
}

/// Nearest-rank quantile of a sorted slice.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let i = ((q * sorted.len() as f64).ceil() as usize).max(1) - 1;
    sorted[i.min(sorted.len() - 1)]
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

fn mean(v: &[usize]) -> f64 {
    v.iter().sum::<usize>() as f64 / v.len().max(1) as f64
}

/// The learner's true correct rate on the topic's items at `t`.
fn true_rate(bundle: &Bundle, learner: &Learner, kc: &KcId, t: Timestamp) -> f64 {
    let Some(refs) = bundle.content.items_by_kc.get(kc) else {
        return 0.0;
    };
    let (mut sum, mut n) = (0.0, 0.0);
    for item in &refs.primary {
        let p = bundle.projection(item).expect("in the bundle");
        for (slot, s) in (0u16..).zip(&p.slots) {
            if s.kcs
                .iter()
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
            {
                sum += learner.p_correct(bundle, item, slot, t);
                n += 1.0;
            }
        }
    }
    sum / f64::max(n, 1.0)
}

fn known(bundle: &Bundle, learner: &Learner, t: Timestamp) -> usize {
    bundle
        .content
        .items_by_kc
        .keys()
        .filter(|kc| learner.recall(kc, t) >= 0.8)
        .count()
}

fn run(
    arm: Arm,
    model: &KnowledgeModel,
    bundle: &Bundle,
    params: &Params,
    learners: usize,
    days: u32,
    seed: u64,
) -> Totals {
    let mut t = Totals::default();
    let mut seeds = SplitMix64::new(seed);
    for _ in 0..learners {
        let learner_seed = seeds.next_u64();
        let track_seed = seeds.next_u64();
        let mut learner = Learner::new(bundle, learner_seed);
        let mut track = Track::new(model, bundle, T0, track_seed);
        let mut jitter = SplitMix64::new(track_seed ^ 0x5EED);
        t.known_start.push(known(bundle, &learner, T0));
        let mut first_day: BTreeMap<KcId, u32> = BTreeMap::new();
        let mut counted: BTreeSet<KcId> = BTreeSet::new();
        for day in 0..days {
            let start = T0.0
                + i64::from(day) * DAY_MS
                + 18 * 3_600_000
                + i64::try_from(jitter.next_u64() % 7_200_000).expect("small");
            track.clock = track.clock.max(start);
            let s: SessionReport = match arm {
                Arm::Ours => session_ours(&mut track, params, &mut learner, Budget::TwentyFive),
                Arm::FsrsOnly => {
                    session_fsrs(&mut track, &mut learner, Budget::TwentyFive, params, false)
                }
            };
            let g = usize::from(day >= 5);
            t.sessions += 1;
            t.seconds += s.seconds;
            t.minutes[g].push(s.seconds / 60.0);
            t.answers[g].push(f64::from(s.answers));
            *t.ends[g].entry(s.end.unwrap_or("time_guard")).or_insert(0) += 1;
            for (c, n) in s.violations {
                *t.violations.entry(c).or_insert(0) += n;
            }
            for kc in track.answers.keys() {
                first_day.entry(kc.clone()).or_insert(day);
            }
            for (kc, k) in &track.state.kcs {
                if k.ever_mastered && counted.insert(kc.clone()) {
                    t.pairs.push(Mastered {
                        learner: learner.clone(),
                        kc: kc.clone(),
                        at: track.now(),
                        answers: track.to_mastery.get(kc).copied().unwrap_or(0),
                        days: day - first_day.get(kc).copied().unwrap_or(day),
                    });
                }
            }
        }
        let end = track.now();
        t.mastered_ever.push(counted.len());
        t.mastered_now.push(
            track
                .state
                .kcs
                .keys()
                .filter(|kc| {
                    use refl_core::model::derived::KcProgress;
                    matches!(
                        model.progress(&track.state, track.bundle, kc, end),
                        KcProgress::Mastered | KcProgress::LongTerm
                    )
                })
                .count(),
        );
        t.known_end.push(known(bundle, &learner, end));
    }
    t
}

fn print(name: &str, t: &Totals, bundle: &Bundle, learners: usize) {
    println!("### {name}\n");
    println!("| | {} | {} |", GROUPS[0], GROUPS[1]);
    println!("| --- | --: | --: |");
    let cell = |v: &[f64]| {
        let s = sorted(v.to_vec());
        format!(
            "{:.1} [{:.1}-{:.1}], mean {:.1}",
            quantile(&s, 0.5),
            quantile(&s, 0.25),
            quantile(&s, 0.75),
            s.iter().sum::<f64>() / s.len().max(1) as f64
        )
    };
    println!(
        "| minutes per session, median [p25-p75] | {} | {} |",
        cell(&t.minutes[0]),
        cell(&t.minutes[1])
    );
    println!(
        "| answers per session, median [p25-p75] | {} | {} |",
        cell(&t.answers[0]),
        cell(&t.answers[1])
    );
    let ends = |g: usize| {
        let n = t.minutes[g].len().max(1) as f64;
        t.ends[g]
            .iter()
            .map(|(e, c)| format!("{e} {:.0}%", 100.0 * f64::from(*c) / n))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("| sessions by end reason | {} | {} |", ends(0), ends(1));
    println!();
    let answers = sorted(t.pairs.iter().map(|m| f64::from(m.answers)).collect());
    let day_gaps = sorted(t.pairs.iter().map(|m| f64::from(m.days)).collect());
    let fm = t
        .pairs
        .iter()
        .filter(|m| true_rate(bundle, &m.learner, &m.kc, Timestamp(m.at.0 + 7 * DAY_MS)) < 0.7)
        .count();
    let gain = (mean(&t.known_end) - mean(&t.known_start)).max(0.0);
    let hours = t.seconds / 3600.0 / learners as f64;
    println!(
        "- topics mastered per learner: ever {:.2}, at the end {:.2}",
        mean(&t.mastered_ever),
        mean(&t.mastered_now)
    );
    println!(
        "- answers to mastery, median [p25-p75]: {:.0} [{:.0}-{:.0}] over {} (learner, topic) pairs",
        quantile(&answers, 0.5),
        quantile(&answers, 0.25),
        quantile(&answers, 0.75),
        t.pairs.len()
    );
    println!(
        "- calendar days to mastery, median [p25-p75]: {:.0} [{:.0}-{:.0}]",
        quantile(&day_gaps, 0.5),
        quantile(&day_gaps, 0.25),
        quantile(&day_gaps, 0.75)
    );
    println!(
        "- false mastery: {:.1}% ({fm} of {})",
        100.0 * fm as f64 / t.pairs.len().max(1) as f64,
        t.pairs.len()
    );
    println!(
        "- truly known topics per learner: start {:.2}, end {:.2}; gain {gain:.2} in {hours:.2} active hours = {:.2} per active hour",
        mean(&t.known_start),
        mean(&t.known_end),
        gain / hours.max(1e-9)
    );
    println!(
        "- answers per learner {:.0}; validator violations {:?}\n",
        t.answers.iter().flatten().sum::<f64>() / learners as f64,
        t.violations
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .map_or("content/bench/v2/bundle.json", String::as_str);
    let num = |i: usize, d: u64| args.get(i).map_or(Some(d), |a| a.parse().ok());
    let (Some(learners), Some(days), Some(seed)) = (num(1, 200), num(2, 30), num(3, 1)) else {
        eprintln!("usage: p1 [BUNDLE] [LEARNERS] [DAYS] [SEED]");
        return ExitCode::FAILURE;
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
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
    let learners = usize::try_from(learners).expect("fits");
    let days = u32::try_from(days).expect("fits");
    let model = KnowledgeModel::default();
    let params = Params::default();
    let speed = ReadingSpeed::default();
    let planner = Planner::new(&model, &bundle, &params, &speed);
    println!("# P1 measurement: {path}, {learners} learners, {days} days, seed {seed}\n");
    println!(
        "model {}, planner {}, budget 25 minutes\n",
        model.version(),
        planner.version()
    );
    for (arm, name) in [(Arm::Ours, "ours"), (Arm::FsrsOnly, "only FSRS")] {
        let t = run(arm, &model, &bundle, &params, learners, days, seed);
        print(name, &t, &bundle, learners);
    }
    ExitCode::SUCCESS
}
