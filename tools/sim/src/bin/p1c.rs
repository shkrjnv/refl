//! P1c part B: where false mastery comes from (`tools/sim/bench/p1c/README.md`).
//! Measurement only: runs our sessions on the K5 cohort with the log
//! recorded, folds each log again through the core model, and at every
//! event attributes each KC's change of μ to its source by diffing the
//! state before and after it. The model is not copied or changed.
//!
//! `p1c [BUNDLE] [LEARNERS] [DAYS] [SEED]`.
//!
//! - **Sources of μ**: the KC primary in the answered slot, secondary in it
//!   (weight 0.3), or neither (a pseudo-observation propagated down
//!   `requires` from a slot KC that requires it). Misconceptions have no
//!   path to μ in model v1, so they get no column. The starting value μ₀ is
//!   the model's prior.
//! - **False mastery**, per answer as in m1: the learner right after the
//!   answer that mastered the KC, 7 days later without reviews, has a true
//!   correct rate below 0.7 on the KC's items.
//! - **Last condition**: the mastery conditions (confidence, τ, production)
//!   that were false just before that answer.

// A measurement binary: counts are small.
#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names
)]

use std::collections::BTreeMap;
use std::process::ExitCode;

use refl_core::bundle::Bundle;
use refl_core::event::{Decoded, EventPayload};
use refl_core::ids::KcId;
use refl_core::item::KcRole;
use refl_core::model::{Applied, FoldCtx, KnowledgeModel, LearnerModel, State};
use refl_core::plan::{Budget, Params, Planner, ReadingSpeed};
use refl_core::rng::SplitMix64;
use refl_core::value::Timestamp;
use refl_sim::cohort::{Learner, Track, session_ours};
use refl_sim::{DAY_MS, T0, live_fixture};

/// Evidence of one KC up to now.
#[derive(Default, Clone)]
struct Acc {
    mu0: f64,
    primary: f64,
    secondary: f64,
    propagation: f64,
    other: f64,
    /// Non-rapid answers where the KC is primary.
    answers: u32,
    /// Correct non-rapid production answers where the KC is primary.
    inputs: u32,
    first: Option<Timestamp>,
    /// UTC days with such an answer.
    days: Vec<i64>,
}

/// A (learner, KC) pair at the answer that mastered it.
struct Pair {
    kc: KcId,
    false_mastery: bool,
    acc: Acc,
    mu: f64,
    bound: f64,
    skill: f64,
    /// The learner's true skill 7 days later without practice.
    skill_later: f64,
    /// The true correct rate on the KC's items 7 days later.
    rate_later: f64,
    /// The lowest true skill 7 days later among the KCs secondary in the
    /// KC's items; NaN when there are none.
    secondary_later: f64,
    days: f64,
    /// FSRS stability of the model's card, days, and the model's R 7 days
    /// later.
    stability: f64,
    recall_later: f64,
    last: String,
    bug: bool,
}

/// The learner's true correct rate on the KC's items at `t`.
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

/// The KCs secondary in the slots where `kc` is primary, with how many
/// such slots each is in.
fn secondaries<'b>(bundle: &'b Bundle, kc: &KcId) -> BTreeMap<&'b KcId, u32> {
    let mut out = BTreeMap::new();
    let Some(refs) = bundle.content.items_by_kc.get(kc) else {
        return out;
    };
    for item in &refs.primary {
        let p = bundle.projection(item).expect("in the bundle");
        for s in &p.slots {
            if s.kcs
                .iter()
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
            {
                for k in s.kcs.iter().filter(|k| k.role == KcRole::Secondary) {
                    *out.entry(&k.kc).or_insert(0) += 1;
                }
            }
        }
    }
    out
}

/// Slots where `kc` is primary.
fn primary_slots(bundle: &Bundle, kc: &KcId) -> u32 {
    let Some(refs) = bundle.content.items_by_kc.get(kc) else {
        return 0;
    };
    let mut n = 0;
    for item in &refs.primary {
        let p = bundle.projection(item).expect("in the bundle");
        n += p
            .slots
            .iter()
            .filter(|s| {
                s.kcs
                    .iter()
                    .any(|k| &k.kc == kc && k.role == KcRole::Primary)
            })
            .count();
    }
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The mastery conditions false in `s` for `kc` at `t`.
fn missing(planner: &Planner<'_>, s: &State, kc: &KcId, t: Timestamp) -> String {
    let p = &planner.model.params;
    let Some(k) = s.kcs.get(kc) else {
        return String::from("confidence+τ+production");
    };
    let mut out = Vec::new();
    if planner.mastery(s, kc, t) < p.mastery_confidence {
        out.push("confidence");
    }
    if k.timed == 0 || k.tau < p.tau_threshold {
        out.push("τ");
    }
    if k.production.len() < p.production_needed {
        out.push("production");
    }
    if out.is_empty() {
        String::from("none")
    } else {
        out.join("+")
    }
}

fn run(
    model: &KnowledgeModel,
    bundle: &Bundle,
    params: &Params,
    learners: usize,
    days: u32,
    seed: u64,
) -> Vec<Pair> {
    let speed = ReadingSpeed::default();
    let planner = Planner::new(model, bundle, params, &speed);
    let ctx = FoldCtx::new(bundle);
    let spelling: Vec<&KcId> = bundle
        .content
        .subjects
        .iter()
        .map(|s| &s.spelling_kc)
        .collect();
    let mut pairs = Vec::new();
    let mut seeds = SplitMix64::new(seed);
    for _ in 0..learners {
        let learner_seed = seeds.next_u64();
        let track_seed = seeds.next_u64();
        let mut learner = Learner::new(bundle, learner_seed);
        let mut track = Track::new(model, bundle, T0, track_seed);
        track.log = Some(Vec::new());
        let mut state = track.state.clone();
        let mut jitter = SplitMix64::new(track_seed ^ 0x5EED);
        for day in 0..days {
            let start = T0.0
                + i64::from(day) * DAY_MS
                + 18 * 3_600_000
                + i64::try_from(jitter.next_u64() % 7_200_000).expect("small");
            track.clock = track.clock.max(start);
            session_ours(&mut track, params, &mut learner, Budget::TwentyFive);
        }
        let log = track.log.take().expect("recorded");
        let mut acc: BTreeMap<KcId, Acc> = BTreeMap::new();
        for logged in &log {
            let before = state.clone();
            let applied = model
                .apply(&mut state, &logged.event, &ctx)
                .expect("in order");
            let Decoded::Known(e) = &logged.event else {
                continue;
            };
            let answer = match (&e.payload, &applied) {
                (EventPayload::Answer(a), Applied::Answer(o)) => Some((a, o)),
                _ => None,
            };
            let slot = answer.map(|(a, _)| {
                let p = bundle.projection(&a.item).expect("in the bundle");
                (p, usize::from(a.slot))
            });
            let t = answer.map_or(e.client_ts, |(a, _)| {
                Timestamp(e.client_ts.0 - i64::from(a.latency_ms))
            });
            for (kc, k) in &state.kcs {
                if spelling.contains(&kc) {
                    continue;
                }
                let a = acc.entry(kc.clone()).or_insert_with(|| Acc {
                    mu0: model
                        .prior(&state.profile, bundle, kc)
                        .map_or(k.mu, |(mu, _)| mu),
                    ..Acc::default()
                });
                let prev = before.kcs.get(kc).map_or(a.mu0, |b| b.mu);
                let d = k.mu - prev;
                let role = slot.as_ref().and_then(|(p, i)| {
                    p.slots[*i].kcs.iter().find(|x| &x.kc == kc).map(|x| x.role)
                });
                match role {
                    Some(KcRole::Primary) => a.primary += d,
                    Some(KcRole::Secondary) => a.secondary += d,
                    None if answer.is_some() => a.propagation += d,
                    None => a.other += d,
                }
                if let (Some((_, o)), Some((p, _)), Some(KcRole::Primary)) = (answer, &slot, role)
                    && !o.rapid
                {
                    a.answers += 1;
                    a.first.get_or_insert(t);
                    let day = t.0.div_euclid(DAY_MS);
                    if !a.days.contains(&day) {
                        a.days.push(day);
                    }
                    if o.correct && p.is_production() {
                        a.inputs += 1;
                    }
                }
                let was = before.kcs.get(kc).is_some_and(|b| b.ever_mastered);
                if k.ever_mastered && !was {
                    let later = Timestamp(e.client_ts.0 + 7 * DAY_MS);
                    let sd = libm::sqrt(k.var);
                    let rate = true_rate(bundle, &logged.learner, kc, later);
                    pairs.push(Pair {
                        kc: kc.clone(),
                        false_mastery: rate < 0.7,
                        rate_later: rate,
                        secondary_later: secondaries(bundle, kc)
                            .keys()
                            .map(|s| logged.learner.skill(s, later))
                            .fold(f64::NAN, f64::min),
                        acc: a.clone(),
                        mu: k.mu,
                        bound: k.mu - 0.84 * sd,
                        skill: logged.learner.skill(kc, t),
                        skill_later: logged.learner.skill(kc, later),
                        days: a.first.map_or(0.0, |f| (t.0 - f.0) as f64 / DAY_MS as f64),
                        stability: k.card.map_or(0.0, |c| c.stability),
                        recall_later: model.recall(k.card.as_ref(), later),
                        last: missing(&planner, &before, kc, t),
                        bug: logged.learner.bug(kc) > 0.0,
                    });
                }
            }
        }
        // The track folds a few events it does not log, so `position` may
        // differ; the KC states must not.
        assert_eq!(state.kcs, track.state.kcs, "the refold equals the session");
    }
    pairs
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.fold((0.0, 0.0), |(s, n), x| (s + x, n + 1.0));
    s / f64::max(n, 1.0)
}

fn print(bundle: &Bundle, pairs: &[Pair]) {
    let mut kcs: Vec<&KcId> = pairs.iter().map(|p| &p.kc).collect();
    kcs.sort();
    kcs.dedup();

    println!("## Secondary KCs in each KC's items\n");
    println!("| KC | slots where primary | secondary KCs (slots) |");
    println!("| --- | --: | --- |");
    for kc in &kcs {
        let s: Vec<String> = secondaries(bundle, kc)
            .iter()
            .map(|(k, n)| format!("{} ({n})", k.as_str()))
            .collect();
        println!(
            "| {} | {} | {} |",
            kc.as_str(),
            primary_slots(bundle, kc),
            if s.is_empty() {
                String::from("–")
            } else {
                s.join(", ")
            }
        );
    }
    println!();

    println!("## False mastery per KC\n");
    println!(
        "False pairs split by the learner's own skill on the KC 7 days later: below 1.5 (the KC itself forgotten) or not (the items fail on something else, a secondary KC or a wrong idea). 1.5 is θ* on the model's scale, used here only to sort.\n"
    );
    println!("| KC | mastered pairs | false | share | own skill +7 d < 1.5 | ≥ 1.5 |");
    println!("| --- | --: | --: | --: | --: | --: |");
    for kc in kcs.iter().copied().map(Some).chain([None]) {
        let of: Vec<&Pair> = pairs
            .iter()
            .filter(|p| kc.is_none_or(|k| &p.kc == k))
            .collect();
        let f = of.iter().filter(|p| p.false_mastery).count();
        let forgot = of
            .iter()
            .filter(|p| p.false_mastery && p.skill_later < 1.5)
            .count();
        println!(
            "| {} | {} | {f} | {:.1}% | {forgot} | {} |",
            kc.map_or("all", |k| k.as_str()),
            of.len(),
            100.0 * f as f64 / of.len().max(1) as f64,
            f - forgot
        );
    }

    println!("\n## Evidence of μ at mastery: false and true pairs\n");
    println!(
        "Means over pairs: μ = μ₀ + primary + secondary + propagation + other (the sum of μ increments by source). Misconceptions have no path to μ in model v1. Bound = μ − 0.84σ.\n"
    );
    println!(
        "| KC | pairs | n | μ₀ | primary | secondary | propagation | other | μ | bound | own answers | own correct inputs |"
    );
    println!("| --- | --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |");
    groups(pairs, &kcs, |kc, label, of| {
        let m = |f: fn(&Pair) -> f64| mean(of.iter().map(|p| f(p)));
        println!(
            "| {kc} | {label} | {} | {:.2} | {:+.2} | {:+.2} | {:+.2} | {:+.2} | {:.2} | {:.2} | {:.1} | {:.1} |",
            of.len(),
            m(|p| p.acc.mu0),
            m(|p| p.acc.primary),
            m(|p| p.acc.secondary),
            m(|p| p.acc.propagation),
            m(|p| p.acc.other),
            m(|p| p.mu),
            m(|p| p.bound),
            m(|p| f64::from(p.acc.answers)),
            m(|p| f64::from(p.acc.inputs)),
        );
    });

    println!("\n## Skill, memory and spacing at mastery\n");
    println!(
        "Skill = the learner's true skill on the KC (logits, the simulator's scale) at mastery and 7 days later without practice; S = the model's FSRS stability, days; R +7 d = the model's recall 7 days later; days = UTC days with an own answer before mastery; span = days from the first own answer; bug = the learner has a wrong idea on the KC that the content does not diagnose.\n"
    );
    println!(
        "| KC | pairs | n | bound | skill | skill − bound | skill +7 d | weakest secondary +7 d | true rate +7 d | S | R +7 d | days | span | bug |"
    );
    println!(
        "| --- | --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |"
    );
    groups(pairs, &kcs, |kc, label, of| {
        let m = |f: fn(&Pair) -> f64| mean(of.iter().map(|p| f(p)).filter(|x| x.is_finite()));
        let weakest = if of.iter().any(|p| p.secondary_later.is_finite()) {
            format!("{:.2}", m(|p| p.secondary_later))
        } else {
            String::from("–")
        };
        println!(
            "| {kc} | {label} | {} | {:.2} | {:.2} | {:+.2} | {:.2} | {weakest} | {:.2} | {:.1} | {:.2} | {:.1} | {:.1} | {:.0}% |",
            of.len(),
            m(|p| p.bound),
            m(|p| p.skill),
            m(|p| p.skill - p.bound),
            m(|p| p.skill_later),
            m(|p| p.rate_later),
            m(|p| p.stability),
            m(|p| p.recall_later),
            m(|p| p.acc.days.len() as f64),
            m(|p| p.days),
            100.0 * m(|p| if p.bug { 1.0 } else { 0.0 })
        );
    });

    println!("\n## The last mastery condition to become true\n");
    println!("Conditions false just before the answer that mastered the KC.\n");
    let mut labels: Vec<&str> = pairs.iter().map(|p| p.last.as_str()).collect();
    labels.sort_unstable();
    labels.dedup();
    println!("| KC | pairs | {} |", labels.join(" | "));
    println!("| --- | --- |{}", " --: |".repeat(labels.len()));
    groups(pairs, &kcs, |kc, label, of| {
        let cells: Vec<String> = labels
            .iter()
            .map(|l| {
                let c = of.iter().filter(|p| p.last == *l).count();
                format!("{c} ({:.0}%)", 100.0 * c as f64 / of.len() as f64)
            })
            .collect();
        println!("| {kc} | {label} | {} |", cells.join(" | "));
    });
}

/// Calls `row` for the false and the true pairs of each KC, then of all.
fn groups(pairs: &[Pair], kcs: &[&KcId], mut row: impl FnMut(&str, &str, &[&Pair])) {
    for kc in kcs.iter().copied().map(Some).chain([None]) {
        for (label, want) in [("false", true), ("true", false)] {
            let of: Vec<&Pair> = pairs
                .iter()
                .filter(|p| kc.is_none_or(|k| &p.kc == k) && p.false_mastery == want)
                .collect();
            if !of.is_empty() {
                row(kc.map_or("all", |k| k.as_str()), label, &of);
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .map_or("content/bench/v2/bundle.json", String::as_str);
    let num = |i: usize, d: u64| args.get(i).map_or(Some(d), |a| a.parse().ok());
    let (Some(learners), Some(days), Some(seed)) = (num(1, 200), num(2, 30), num(3, 1)) else {
        eprintln!("usage: p1c [BUNDLE] [LEARNERS] [DAYS] [SEED]");
        return ExitCode::FAILURE;
    };
    let bundle: Bundle = match std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
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
    println!("# P1c measurement: {path}, {learners} learners, {days} days, seed {seed}\n");
    println!(
        "model {}, planner {}, budget 25 minutes; false mastery per answer (m1)\n",
        model.version(),
        planner.version()
    );
    let pairs = run(&model, &bundle, &params, learners, days, seed);
    print(&bundle, &pairs);
    ExitCode::SUCCESS
}
