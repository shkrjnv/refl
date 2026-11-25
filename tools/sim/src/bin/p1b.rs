//! P1b: the p1 report (`tools/sim/src/bin/p1.rs`) with what P1b needs on
//! top (`tools/sim/bench/p1b/README.md`): the share of masteries reached on
//! the UTC day of the topic's first answer, false mastery inside that group
//! and outside it, and a per-learner table for paired comparisons between
//! two runs on the same cohort seed. It uses only the public cohort API, so
//! the same file builds at older commits (the 2×2 cells of part 1).
//!
//! `p1b [BUNDLE] [LEARNERS] [DAYS] [SEED] [PER_LEARNER_PREFIX]` runs both arms
//! and, with the prefix, writes `<prefix>-ours.tsv` and
//! `<prefix>-fsrs.tsv`; `p1b --paired A.tsv B.tsv` prints the per-learner
//! differences B − A.
//!
//! Metrics are those of p1: false mastery is a topic the model counted
//! mastered whose true correct rate 7 days later, without reviews, is below
//! 0.7, from the learner as they were at the end of the session that
//! mastered it; calendar days run from the day of the first answer to that
//! session's day; truly known topics have a true recall of at least 0.8.

// A measurement binary: counts are small.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
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

/// A topic the model counted mastered.
struct Mastered {
    answers: u32,
    days: u32,
    /// True correct rate 7 days later below 0.7.
    false_mastery: bool,
}

/// One learner of a run, for paired comparisons.
#[derive(Default)]
struct PerLearner {
    early_answers: Vec<f64>,
    early_minutes: Vec<f64>,
    pairs: Vec<Mastered>,
    known_start: usize,
    known_end: usize,
    seconds: f64,
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
    learners: Vec<PerLearner>,
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

fn mean_f(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

fn mean(v: &[usize]) -> f64 {
    v.iter().sum::<usize>() as f64 / v.len().max(1) as f64
}

fn percent(k: usize, n: usize) -> String {
    format!("{:.1}% ({k} of {n})", 100.0 * k as f64 / n.max(1) as f64)
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
        let mut one = PerLearner {
            known_start: known(bundle, &learner, T0),
            ..PerLearner::default()
        };
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
            one.seconds += s.seconds;
            t.minutes[g].push(s.seconds / 60.0);
            t.answers[g].push(f64::from(s.answers));
            if g == 0 {
                one.early_minutes.push(s.seconds / 60.0);
                one.early_answers.push(f64::from(s.answers));
            }
            *t.ends[g].entry(s.end.unwrap_or("time_guard")).or_insert(0) += 1;
            for (c, n) in s.violations {
                *t.violations.entry(c).or_insert(0) += n;
            }
            for kc in track.answers.keys() {
                first_day.entry(kc.clone()).or_insert(day);
            }
            for (kc, k) in &track.state.kcs {
                if k.ever_mastered && counted.insert(kc.clone()) {
                    let later = Timestamp(track.now().0 + 7 * DAY_MS);
                    one.pairs.push(Mastered {
                        answers: track.to_mastery.get(kc).copied().unwrap_or(0),
                        days: day - first_day.get(kc).copied().unwrap_or(day),
                        false_mastery: true_rate(bundle, &learner, kc, later) < 0.7,
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
        one.known_end = known(bundle, &learner, end);
        t.learners.push(one);
    }
    t
}

fn print(name: &str, t: &Totals, learners: usize) {
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
            mean_f(&s)
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
    let pairs: Vec<&Mastered> = t.learners.iter().flat_map(|l| &l.pairs).collect();
    let answers = sorted(pairs.iter().map(|m| f64::from(m.answers)).collect());
    let day_gaps = sorted(pairs.iter().map(|m| f64::from(m.days)).collect());
    let fm = pairs.iter().filter(|m| m.false_mastery).count();
    let same: Vec<&&Mastered> = pairs.iter().filter(|m| m.days == 0).collect();
    let fm_same = same.iter().filter(|m| m.false_mastery).count();
    let start: Vec<usize> = t.learners.iter().map(|l| l.known_start).collect();
    let end: Vec<usize> = t.learners.iter().map(|l| l.known_end).collect();
    let gain = (mean(&end) - mean(&start)).max(0.0);
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
        pairs.len()
    );
    println!(
        "- calendar days to mastery, median [p25-p75]: {:.0} [{:.0}-{:.0}]",
        quantile(&day_gaps, 0.5),
        quantile(&day_gaps, 0.25),
        quantile(&day_gaps, 0.75)
    );
    println!("- false mastery: {}", percent(fm, pairs.len()));
    println!(
        "- mastered on the UTC day of the first answer: {}; false mastery there {}, on later days {}",
        percent(same.len(), pairs.len()),
        percent(fm_same, same.len()),
        percent(fm - fm_same, pairs.len() - same.len())
    );
    println!(
        "- truly known topics per learner: start {:.2}, end {:.2}; gain {gain:.2} in {hours:.2} active hours = {:.2} per active hour",
        mean(&start),
        mean(&end),
        gain / hours.max(1e-9)
    );
    println!(
        "- answers per learner {:.0}; validator violations {:?}\n",
        t.answers.iter().flatten().sum::<f64>() / learners as f64,
        t.violations
    );
}

/// Per-learner columns of the TSV, in order.
const COLUMNS: [&str; 9] = [
    "answers per session, days 1-5",
    "minutes per session, days 1-5",
    "topics mastered",
    "answers to mastery, mean",
    "calendar days to mastery, mean",
    "false masteries",
    "truly known at the end",
    "truly known gain",
    "active hours",
];

fn per_learner_tsv(t: &Totals) -> String {
    let mut out = String::from("learner");
    for c in COLUMNS {
        let _ = write!(out, "\t{c}");
    }
    out.push('\n');
    for (i, l) in t.learners.iter().enumerate() {
        let to_mastery = |f: fn(&Mastered) -> f64| {
            if l.pairs.is_empty() {
                f64::NAN
            } else {
                l.pairs.iter().map(f).sum::<f64>() / l.pairs.len() as f64
            }
        };
        let row = [
            mean_f(&l.early_answers),
            mean_f(&l.early_minutes),
            l.pairs.len() as f64,
            to_mastery(|m| f64::from(m.answers)),
            to_mastery(|m| f64::from(m.days)),
            l.pairs.iter().filter(|m| m.false_mastery).count() as f64,
            l.known_end as f64,
            l.known_end as f64 - l.known_start as f64,
            l.seconds / 3600.0,
        ];
        let _ = write!(out, "{i}");
        for v in row {
            let _ = write!(out, "\t{v}");
        }
        out.push('\n');
    }
    out
}

fn read_tsv(path: &str) -> Result<Vec<Vec<f64>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .skip(1)
        .map(|line| {
            line.split('\t')
                .skip(1)
                .map(|v| v.parse::<f64>().map_err(|e| format!("{path}: {e}")))
                .collect()
        })
        .collect()
}

/// Per-learner differences B − A over learners where both values exist:
/// mean of A and B, mean difference with its standard error, learners where
/// B is above and below A.
fn paired(a: &str, b: &str) -> Result<(), String> {
    let (ra, rb) = (read_tsv(a)?, read_tsv(b)?);
    if ra.len() != rb.len() {
        return Err(format!("{a} and {b}: different cohorts"));
    }
    println!("Paired per learner, B − A; A = {a}, B = {b}\n");
    println!("| | learners | mean A | mean B | mean B − A (± s.e.) | B above / below A |");
    println!("| --- | --: | --: | --: | --: | --: |");
    for (j, name) in COLUMNS.iter().enumerate() {
        let d: Vec<(f64, f64)> = ra
            .iter()
            .zip(&rb)
            .map(|(x, y)| (x[j], y[j]))
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .collect();
        let n = d.len() as f64;
        let diffs: Vec<f64> = d.iter().map(|(x, y)| y - x).collect();
        let m = mean_f(&diffs);
        let var = diffs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0).max(1.0);
        println!(
            "| {name} | {} | {:.2} | {:.2} | {m:+.2} (± {:.2}) | {} / {} |",
            d.len(),
            d.iter().map(|p| p.0).sum::<f64>() / n.max(1.0),
            d.iter().map(|p| p.1).sum::<f64>() / n.max(1.0),
            libm::sqrt(var / n.max(1.0)),
            diffs.iter().filter(|x| **x > 0.0).count(),
            diffs.iter().filter(|x| **x < 0.0).count()
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--paired") {
        let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
            eprintln!("usage: p1b --paired A.tsv B.tsv");
            return ExitCode::FAILURE;
        };
        return match paired(a, b) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    let path = args
        .first()
        .map_or("content/bench/v2/bundle.json", String::as_str);
    let num = |i: usize, d: u64| args.get(i).map_or(Some(d), |a| a.parse().ok());
    let (Some(learners), Some(days), Some(seed)) = (num(1, 200), num(2, 30), num(3, 1)) else {
        eprintln!("usage: p1b [BUNDLE] [LEARNERS] [DAYS] [SEED] [PER_LEARNER_PREFIX]");
        return ExitCode::FAILURE;
    };
    let prefix = args.get(4);
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
    println!("# P1b measurement: {path}, {learners} learners, {days} days, seed {seed}\n");
    println!(
        "model {}, planner {}, budget 25 minutes\n",
        model.version(),
        planner.version()
    );
    for (arm, name, file) in [
        (Arm::Ours, "ours", "ours"),
        (Arm::FsrsOnly, "only FSRS", "fsrs"),
    ] {
        let t = run(arm, &model, &bundle, &params, learners, days, seed);
        print(name, &t, learners);
        if let Some(prefix) = prefix {
            let out = format!("{prefix}-{file}.tsv");
            if let Err(e) = std::fs::write(&out, per_learner_tsv(&t)) {
                eprintln!("{out}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
