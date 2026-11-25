//! M1: why the knowledge model lags (`tools/sim/m1/REPORT.md`). Measurement
//! only: runs the K5 cohort with the log recorded, replays the logs through
//! a copy of the knowledge update of model v1 (`core/src/model/v1.rs`,
//! `observe` and the knowledge part of `answer`) with one change at a time,
//! and scores each variant against the synthetic learner's hidden state.
//! The copy is checked against the core fold on every answer: with no change
//! it must give the same μ, σ², prediction and mastery bit for bit. Two
//! planner-side options run on-policy through a copy of `session_ours` that
//! only swaps the chosen item. Neither the model nor the planner is changed.
//!
//! `m1 [BUNDLE] [LEARNERS] [DAYS] [SEED]`; the bundle defaults to the frozen
//! `content/bench/v1/bundle.json`.

// A measurement binary: counts and timestamps are small, every report line
// is one place, and the names follow the formulas of docs/03.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::struct_excessive_bools,
    clippy::similar_names,
    clippy::many_single_char_names
)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use refl_core::bundle::Bundle;
use refl_core::event::{AnswerContext, Confidence, Decoded, EventPayload, Verdict};
use refl_core::ids::{ItemRef, KcId};
use refl_core::item::KcRole;
use refl_core::model::math::{self, KcTerm, effective_slope, phi, sigmoid};
use refl_core::model::{Applied, FoldCtx, KnowledgeModel, LearnerModel, Profile, State};
use refl_core::plan::{Answered, Budget, Next, Outcome, Params, Phase, Plan, Planner};
use refl_core::rng::SplitMix64;
use refl_core::value::{Lang, Timestamp};
use refl_sim::cohort::{Learner, Logged, SLOPE, Track, difficulty_shift, session_ours};
use refl_sim::{DAY_MS, T0, live_fixture};

fn days(from: Timestamp, to: Timestamp) -> f64 {
    (to.0 - from.0).max(0) as f64 / DAY_MS as f64
}

fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-9, 1.0 - 1e-9);
    libm::log(p / (1.0 - p))
}

// ---------------------------------------------------------------- data

/// How a session picks its item: our planner as is, or our planner with the
/// chosen item swapped by a planner-side rule near the mastery bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Swap {
    None,
    /// While M is in [0.5, 0.8): the hardest unseen item on the KC, at most
    /// two per KC per session.
    Measure,
    /// While M ≥ 0.5: an unseen typed item on the KC whose p is closest to
    /// the step's target, instead of a choice.
    Typed,
}

#[derive(Debug)]
struct LearnerRun {
    start: Learner,
    log: Vec<Logged>,
    /// Session minutes.
    minutes: f64,
}

fn context(phase: Phase) -> AnswerContext {
    match phase {
        Phase::Review => AnswerContext::Review,
        Phase::Lesson | Phase::Block => AnswerContext::Lesson,
        Phase::Interleaved | Phase::Prerequisite | Phase::Final => AnswerContext::Practice,
    }
}

/// The session-eligible items where `kc` is primary.
fn primary<'b>(bundle: &'b Bundle, kc: &KcId) -> &'b [ItemRef] {
    bundle
        .content
        .items_by_kc
        .get(kc)
        .map_or(&[], |r| r.primary.as_slice())
}

/// Unseen-in-session primary slots of `kc`.
fn unseen(bundle: &Bundle, plan: &Plan, kc: &KcId) -> Vec<(ItemRef, u16)> {
    let shown: BTreeSet<_> = plan
        .done
        .iter()
        .filter_map(|d| d.shown.as_ref().map(|s| s.item.id.clone()))
        .collect();
    let mut out = Vec::new();
    for item in primary(bundle, kc) {
        if shown.contains(&item.id) {
            continue;
        }
        let p = bundle.projection(item).expect("in the bundle");
        for (slot, s) in (0u16..).zip(&p.slots) {
            if s.kcs
                .iter()
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
            {
                out.push((item.clone(), slot));
            }
        }
    }
    out
}

/// `session_ours` of `cohort.rs` with an optional item swap; with
/// `Swap::None` it is the same session (checked in `main`).
fn session(
    track: &mut Track<'_>,
    params: &Params,
    learner: &mut Learner,
    swap: Swap,
    swaps: &mut BTreeMap<KcId, u32>,
) {
    let pace = track.speed(params);
    let planner = Planner::new(track.model, track.bundle, params, &pace);
    let bundle = track.bundle;
    let started = track.clock;
    let seed = track.seed();
    let session = planner.build_session(
        &track.state,
        &track.history,
        track.now(),
        Budget::TwentyFive,
        Lang::Ru,
        seed,
    );
    if let Some(ev) = session.issued {
        track.plan_issued(ev);
    }
    let mut plan = session.plan;
    let limit = 3 * i64::from(Budget::TwentyFive.seconds()) * 1000;
    swaps.clear();
    while track.clock - started < limit {
        let seed = track.seed();
        match planner.next_item(&track.state, &plan, &track.history, track.now(), seed) {
            Next::Finished { .. } => break,
            Next::Lesson { kc, explanation } => {
                track.lesson(learner, &kc, &explanation);
                plan = planner
                    .adapt(&track.state, &plan, &Outcome::Lesson, track.now())
                    .plan;
            }
            Next::Item(choice) => {
                track.plan_issued(choice.issued);
                let step = plan.steps[0].clone();
                let minutes = (track.clock - started) as f64 / 60_000.0;
                let now = track.now();
                let m = planner.mastery(&track.state, &step.kc, now);
                let used = swaps.get(&step.kc).copied().unwrap_or(0);
                let target = match step.phase {
                    Phase::Interleaved | Phase::Final => params.p_interleaved,
                    Phase::Prerequisite => params.p_prerequisite,
                    _ => params.p_block,
                };
                let mut pick = (choice.item, choice.slot);
                let candidates = unseen(bundle, &plan, &step.kc);
                match swap {
                    Swap::Measure if (0.5..0.8).contains(&m) && used < 2 => {
                        if let Some(c) = candidates.iter().max_by(|a, b| {
                            let pa = bundle.projection(&a.0).expect("in the bundle");
                            let pb = bundle.projection(&b.0).expect("in the bundle");
                            pa.difficulty
                                .get()
                                .cmp(&pb.difficulty.get())
                                .then(pa.typed_input().cmp(&pb.typed_input()))
                                .then(b.0.id.cmp(&a.0.id))
                        }) {
                            pick = c.clone();
                            *swaps.entry(step.kc.clone()).or_insert(0) += 1;
                        }
                    }
                    Swap::Typed
                        if m >= 0.5
                            && !bundle
                                .projection(&pick.0)
                                .expect("in the bundle")
                                .typed_input() =>
                    {
                        let best = candidates
                            .iter()
                            .filter(|c| bundle.projection(&c.0).expect("in").typed_input())
                            .filter_map(|c| {
                                let p =
                                    track.model.predict(&track.state, bundle, &c.0, c.1, now)?;
                                Some((libm::fabs(p - target), c))
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.0.id.cmp(&b.1.0.id)));
                        if let Some((_, c)) = best {
                            pick = c.clone();
                            *swaps.entry(step.kc.clone()).or_insert(0) += 1;
                        }
                    }
                    _ => {}
                }
                let (a, outcome) =
                    track.answer(learner, &pick.0, pick.1, context(step.phase), minutes);
                let adapted = planner.adapt(
                    &track.state,
                    &plan,
                    &Outcome::Answer(Answered::new(&a, &outcome)),
                    track.now(),
                );
                if let Some(ev) = adapted.issued {
                    track.plan_issued(ev);
                }
                plan = adapted.plan;
            }
        }
    }
}

/// The cohort of `cohort()` in `cohort.rs` (same seeds), our sessions, the
/// log recorded. `exact` runs `session_ours` itself.
fn generate(
    model: &KnowledgeModel,
    bundle: &Bundle,
    params: &Params,
    learners: usize,
    days: u32,
    seed: u64,
    swap: Swap,
    exact: bool,
) -> (Vec<LearnerRun>, usize, u32) {
    let mut out = Vec::new();
    let mut mastered = 0;
    let mut swapped = 0;
    let mut seeds = SplitMix64::new(seed);
    for _ in 0..learners {
        let learner_seed = seeds.next_u64();
        let track_seed = seeds.next_u64();
        let mut learner = Learner::new(bundle, learner_seed);
        let start = learner.clone();
        let mut track = Track::new(model, bundle, T0, track_seed);
        track.log = Some(Vec::new());
        let mut jitter = SplitMix64::new(track_seed ^ 0x5EED);
        let mut swaps = BTreeMap::new();
        let mut minutes = 0.0;
        for day in 0..days {
            let start = T0.0
                + i64::from(day) * DAY_MS
                + 18 * 3_600_000
                + i64::try_from(jitter.next_u64() % 7_200_000).expect("small");
            track.clock = track.clock.max(start);
            let before = track.clock;
            if exact {
                session_ours(&mut track, params, &mut learner, Budget::TwentyFive);
            } else {
                session(&mut track, params, &mut learner, swap, &mut swaps);
                swapped += swaps.values().sum::<u32>();
            }
            minutes += (track.clock - before) as f64 / 60_000.0;
        }
        mastered += track.state.kcs.values().filter(|k| k.ever_mastered).count();
        out.push(LearnerRun {
            start,
            log: track.log.take().expect("recorded"),
            minutes,
        });
    }
    (out, mastered, swapped)
}

// ---------------------------------------------------------------- replica

/// How the secondary KCs of a slot enter the update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Secondary {
    /// As in v1: in the product and updated with weight `w_secondary`.
    Model,
    /// Left out of the slot: the primary takes the whole outcome.
    Drop,
    /// Oracle: in the product with the learner's true P*, never updated.
    Oracle,
}

/// One change to the knowledge update; `base()` is model v1.
#[derive(Debug, Clone, Copy)]
struct Variant {
    name: &'static str,
    slope: f64,
    gamma: f64,
    w_secondary: f64,
    secondary: Secondary,
    rho: f64,
    delta: f64,
    eta: f64,
    /// Process noise added to σ² of every slot KC before each answer.
    q_answer: f64,
    /// `p'` with the raw slope `a` (docs/03) instead of `a_eff` (code).
    raw_slope_in_dp: bool,
    theta_star: f64,
    confidence: f64,
    /// Only typed answers update knowledge.
    typed_only: bool,
}

impl Variant {
    fn base(p: &refl_core::model::Params) -> Self {
        Self {
            name: "v1 as is",
            slope: p.slope,
            gamma: p.unsure_gamma,
            w_secondary: p.w_secondary,
            secondary: Secondary::Model,
            rho: p.rho,
            delta: p.delta,
            eta: p.eta,
            q_answer: 0.0,
            raw_slope_in_dp: false,
            theta_star: p.mastery_theta,
            confidence: p.mastery_confidence,
            typed_only: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Belief {
    mu: f64,
    var: f64,
    at: Timestamp,
}

/// A mastery the variant declared.
#[derive(Debug, Clone)]
struct Mastery {
    learner: usize,
    kc: KcId,
    answers: u32,
    days: f64,
    mu: f64,
    sd: f64,
    /// True correct rate on the KC's items 7 days later without reviews.
    true7: f64,
    true0: f64,
    bug: bool,
}

#[derive(Debug, Default)]
struct Scores {
    masteries: Vec<Mastery>,
    /// (Σ log loss, answers).
    log_loss: (f64, u32),
    /// Calibration: (model P, truth without bug, truth with bug).
    calibration: Vec<(f64, bool, bool)>,
    /// Per primary KC answer before mastery: (typed, w·I).
    info: Vec<(bool, f64)>,
    /// Model state at the first answer after which the learner truly knew
    /// the KC (true rate now ≥ 0.8): (answers, μ, σ, M, true θ on the
    /// model's scale).
    known_at: Vec<(u32, f64, f64, f64, f64)>,
}

struct Replica<'a> {
    v: Variant,
    model: &'a KnowledgeModel,
    bundle: &'a Bundle,
    beliefs: BTreeMap<KcId, Belief>,
}

impl Replica<'_> {
    fn touch(&mut self, kc: &KcId, t: Timestamp) -> Option<&mut Belief> {
        if !self.beliefs.contains_key(kc) {
            let (mu, var) = self.model.prior(&Profile::default(), self.bundle, kc)?;
            self.beliefs.insert(kc.clone(), Belief { mu, var, at: t });
        }
        let b = self.beliefs.get_mut(kc)?;
        if t > b.at {
            b.var += self.v.eta * days(b.at, t);
            b.at = t;
        }
        Some(b)
    }

    /// μ, σ² at `now` without touching.
    fn belief(&self, kc: &KcId, now: Timestamp) -> Option<(f64, f64)> {
        match self.beliefs.get(kc) {
            Some(b) => Some((b.mu, b.var + self.v.eta * days(b.at, now))),
            None => self.model.prior(&Profile::default(), self.bundle, kc),
        }
    }

    fn recall(&self, s: &State, kc: &KcId, t: Timestamp) -> f64 {
        self.model
            .recall(s.kcs.get(kc).and_then(|k| k.card.as_ref()), t)
    }

    fn mastery_p(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        let Some((mu, var)) = self.belief(kc, now) else {
            return 0.0;
        };
        let theta = mu - self.v.delta * (1.0 - self.recall(s, kc, now));
        phi((theta - self.v.theta_star) / libm::sqrt(var))
    }

    fn term(&self, s: &State, kc: &KcId, mu: f64, var: f64, b: f64, t: Timestamp) -> KcTerm {
        let theta = mu - self.v.delta * (1.0 - self.recall(s, kc, t));
        KcTerm::new(theta, b, effective_slope(self.v.slope, var))
    }

    /// `math::evidence`, or with the raw slope in `p'`.
    fn evidence(&self, g: f64, slip: f64, y: bool, terms: &[KcTerm]) -> Vec<math::Evidence> {
        if !self.v.raw_slope_in_dp {
            return math::evidence(g, slip, y, terms);
        }
        let raw: Vec<KcTerm> = terms
            .iter()
            .map(|t| KcTerm {
                p_star: t.p_star,
                slope: self.v.slope,
            })
            .collect();
        math::evidence(g, slip, y, &raw)
    }

    /// One Newton step: `kcs` with weights (weight 0: in the product, not
    /// updated); `fixed` are oracle terms in the product. Returns w·I per KC.
    fn observe(
        &mut self,
        s: &State,
        kcs: &[(KcId, f64)],
        fixed: &[f64],
        b: f64,
        g: f64,
        slip: f64,
        y: bool,
        t: Timestamp,
    ) -> Vec<(KcId, f64)> {
        let mut terms = Vec::new();
        let mut live = Vec::new();
        for (kc, w) in kcs {
            let Some(bel) = self.touch(kc, t).copied() else {
                continue;
            };
            terms.push(self.term(s, kc, bel.mu, bel.var, b, t));
            live.push((kc.clone(), *w));
        }
        for p in fixed {
            terms.push(KcTerm {
                p_star: *p,
                slope: 0.0,
            });
        }
        let evidence = self.evidence(g, slip, y, &terms);
        let mut out = Vec::new();
        for ((kc, w), e) in live.into_iter().zip(evidence) {
            if w <= 0.0 {
                continue;
            }
            let bel = self.beliefs.get_mut(&kc).expect("touched");
            let (mu, var) = math::posterior(bel.mu, bel.var, w, e);
            bel.mu = mu;
            bel.var = var;
            out.push((kc, w * e.info));
        }
        out
    }
}

/// Mean true correct rate of `learner` on the primary slots of `kc` at `t`.
fn true_rate(bundle: &Bundle, learner: &Learner, kc: &KcId, t: Timestamp) -> f64 {
    let mut sum = 0.0;
    let mut n: f64 = 0.0;
    for item in primary(bundle, kc) {
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
    sum / n.max(1.0)
}

/// Replays every log through `v`, folding the core model alongside for the
/// memory, automaticity and production parts; `check` asserts the replica
/// equals the core knowledge state on every answer.
fn replay(
    model: &KnowledgeModel,
    bundle: &Bundle,
    runs: &[LearnerRun],
    v: Variant,
    check: bool,
) -> Scores {
    let p = &model.params;
    let mut scores = Scores::default();
    let ctx = FoldCtx::new(bundle);
    let spelling: BTreeSet<KcId> = bundle
        .content
        .subjects
        .iter()
        .map(|s| s.spelling_kc.clone())
        .collect();
    // The true rate that θ* stands for: (1 − s) σ(a θ*) on a medium typed item.
    let star = (1.0 - p.slip) * sigmoid(v.slope * v.theta_star);
    for (li, run) in runs.iter().enumerate() {
        let mut state = model.init(&Profile::default());
        let mut r = Replica {
            v,
            model,
            bundle,
            beliefs: BTreeMap::new(),
        };
        let mut ever: BTreeSet<KcId> = BTreeSet::new();
        let mut known: BTreeSet<KcId> = BTreeSet::new();
        let mut answers: BTreeMap<KcId, u32> = BTreeMap::new();
        let mut first: BTreeMap<KcId, Timestamp> = BTreeMap::new();
        let mut before = &run.start;
        for logged in &run.log {
            let Decoded::Known(e) = &logged.event else {
                continue;
            };
            let t = e.client_ts;
            let EventPayload::Answer(a) = &e.payload else {
                model
                    .apply(&mut state, &logged.event, &ctx)
                    .expect("in order");
                before = &logged.learner;
                continue;
            };
            let item = bundle.projection(&a.item).expect("in the bundle");
            let slot = &item.slots[usize::from(a.slot)];
            let shown = Timestamp(t.0 - i64::from(a.latency_ms));
            let correct = matches!(a.verdict, Verdict::Correct | Verdict::Typo);
            let expected_ms = f64::from(item.expected_time) * 1000.0;
            let rapid = !a.backgrounded
                && expected_ms > 0.0
                && f64::from(a.latency_ms) < p.rapid_share * expected_ms
                && (!item.typed_input() || a.latency_ms < p.rapid_typed_ms);
            let b = f64::from(item.difficulty.get()) - 3.0;
            let true_star = |kc: &KcId| {
                sigmoid(SLOPE * (before.skill(kc, shown) - (b + difficulty_shift(&a.item))))
            };

            // Which KCs are in the slot, with which weight.
            let mut modifier = 1.0;
            if correct && a.hints > 0 {
                modifier *= p.w_hint;
            }
            if a.attempt >= 2 {
                modifier *= p.w_retry;
            }
            if a.verdict == Verdict::Unrecognized {
                modifier *= p.w_unrecognized;
            }
            let mut weighted: Vec<(KcId, f64)> = Vec::new();
            let mut fixed: Vec<f64> = Vec::new();
            for k in &slot.kcs {
                match (k.role, v.secondary) {
                    (KcRole::Primary, _) => weighted.push((k.kc.clone(), modifier)),
                    (KcRole::Secondary, Secondary::Model) => {
                        weighted.push((k.kc.clone(), v.w_secondary * modifier));
                    }
                    (KcRole::Secondary, Secondary::Drop) => {}
                    (KcRole::Secondary, Secondary::Oracle) => fixed.push(true_star(&k.kc)),
                }
            }
            if v.q_answer > 0.0 && !rapid {
                for (kc, _) in &weighted {
                    if let Some(bel) = r.touch(kc, shown) {
                        bel.var += v.q_answer;
                    }
                }
            }

            // Prediction at display time, base g and s.
            let mut terms = Vec::new();
            for (kc, _) in &weighted {
                let (mu, var) = r.belief(kc, shown).expect("in the bundle");
                terms.push(r.term(&state, kc, mu, var, b, shown));
            }
            for p_star in &fixed {
                terms.push(KcTerm {
                    p_star: *p_star,
                    slope: 0.0,
                });
            }
            let predicted = math::predict(item.guessing(a.slot), p.slip, &terms);

            let mut touched: BTreeSet<KcId> = BTreeSet::new();
            let mut info: Vec<(KcId, f64)> = Vec::new();
            let update = !rapid && (!v.typed_only || item.typed_input());
            if !rapid {
                let y = if correct { 1.0 } else { 0.0 };
                let q = predicted.clamp(1e-9, 1.0 - 1e-9);
                scores.log_loss.0 -= y * libm::log(q) + (1.0 - y) * libm::log(1.0 - q);
                scores.log_loss.1 += 1;
            }
            if update {
                let mut g = item.guessing(a.slot);
                let mut slip = p.slip;
                if correct && a.confidence == Some(Confidence::Unsure) {
                    g += (1.0 - g) * v.gamma;
                }
                if !correct && a.confidence == Some(Confidence::Sure) {
                    slip /= 2.0;
                }
                info = r.observe(&state, &weighted, &fixed, b, g, slip, correct, t);
                touched.extend(weighted.iter().map(|(k, _)| k.clone()));
                if correct && v.rho > 0.0 {
                    let mut pseudo: BTreeMap<&KcId, f64> = BTreeMap::new();
                    let all: Vec<(KcId, f64)> = slot
                        .kcs
                        .iter()
                        .map(|k| {
                            let w = if k.role == KcRole::Primary {
                                modifier
                            } else {
                                p.w_secondary * modifier
                            };
                            (k.kc.clone(), w)
                        })
                        .collect();
                    let sources = if v.secondary == Secondary::Model {
                        &weighted
                    } else {
                        &all
                    };
                    for (kc, w) in sources {
                        for req in bundle
                            .content
                            .requires_closure
                            .get(kc)
                            .into_iter()
                            .flatten()
                        {
                            if slot.kcs.iter().any(|k| k.kc == req.kc) {
                                continue;
                            }
                            let wr = w * libm::pow(v.rho, f64::from(req.distance));
                            let e = pseudo.entry(&req.kc).or_insert(0.0);
                            *e = e.max(wr);
                        }
                    }
                    for (kc, w) in pseudo {
                        r.observe(&state, &[(kc.clone(), w)], &[], 0.0, 0.0, p.slip, true, t);
                        touched.insert(kc.clone());
                    }
                }
            }

            // The core fold of the same answer.
            let Applied::Answer(outcome) = model
                .apply(&mut state, &logged.event, &ctx)
                .expect("in order")
            else {
                panic!("answers resolve");
            };
            if check {
                assert_eq!(outcome.rapid, rapid, "rapid rule");
                assert_eq!(
                    outcome.predicted.to_bits(),
                    predicted.to_bits(),
                    "prediction"
                );
                for (kc, k) in &state.kcs {
                    if spelling.contains(kc) {
                        continue;
                    }
                    let bel = r.beliefs.get(kc).expect("replica has every KC");
                    assert_eq!(k.mu.to_bits(), bel.mu.to_bits(), "μ of {kc}");
                    assert_eq!(k.var.to_bits(), bel.var.to_bits(), "σ² of {kc}");
                }
            }
            if rapid {
                before = &logged.learner;
                continue;
            }
            let after = &logged.learner;
            for k in slot.kcs.iter().filter(|k| k.role == KcRole::Primary) {
                *answers.entry(k.kc.clone()).or_insert(0) += 1;
                first.entry(k.kc.clone()).or_insert(t);
                if !ever.contains(&k.kc) {
                    let i = info
                        .iter()
                        .find(|(x, _)| x == &k.kc)
                        .map_or(0.0, |(_, i)| *i);
                    scores.info.push((item.typed_input(), i));
                }
                if check || v.name == "v1 as is" {
                    let m = r.mastery_p(&state, &k.kc, t);
                    let truth = after.recall(&k.kc, t);
                    scores.calibration.push((
                        m,
                        truth >= star,
                        (1.0 - after.bug(&k.kc)) * truth >= star,
                    ));
                }
                if !known.contains(&k.kc) && true_rate(bundle, after, &k.kc, t) >= 0.8 {
                    known.insert(k.kc.clone());
                    let (mu, var) = r.belief(&k.kc, t).expect("in the bundle");
                    let theta = logit(after.recall(&k.kc, t) / (1.0 - p.slip)) / v.slope;
                    scores.known_at.push((
                        answers[&k.kc],
                        mu,
                        libm::sqrt(var),
                        r.mastery_p(&state, &k.kc, t),
                        theta,
                    ));
                }
            }
            for kc in &touched {
                if ever.contains(kc) {
                    continue;
                }
                let Some(k) = state.kcs.get(kc) else {
                    continue;
                };
                let mastered = r.mastery_p(&state, kc, t) >= v.confidence
                    && k.timed > 0
                    && k.tau >= p.tau_threshold
                    && k.production.len() >= p.production_needed;
                if check {
                    assert_eq!(mastered, k.ever_mastered, "mastery of {kc}");
                }
                if mastered {
                    ever.insert(kc.clone());
                    let (mu, var) = r.belief(kc, t).expect("in the bundle");
                    scores.masteries.push(Mastery {
                        learner: li,
                        kc: kc.clone(),
                        answers: answers.get(kc).copied().unwrap_or(0),
                        days: first.get(kc).map_or(0.0, |f| days(*f, t)),
                        mu,
                        sd: libm::sqrt(var),
                        true7: true_rate(bundle, after, kc, Timestamp(t.0 + 7 * DAY_MS)),
                        true0: true_rate(bundle, after, kc, t),
                        bug: after.bug(kc) > 0.0,
                    });
                }
            }
            before = &logged.learner;
        }
    }
    scores
}

// ---------------------------------------------------------------- report

fn median_u(mut v: Vec<u32>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_unstable();
    f64::from(v[v.len() / 2])
}

fn median_f(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// One row: mastered pairs, median answers and days, the paired median over
/// the reference pairs (unmastered = ∞), false mastery, log loss.
fn row(name: &str, s: &Scores, reference: &[Mastery]) -> String {
    let n = s.masteries.len();
    let fm = s.masteries.iter().filter(|m| m.true7 < 0.7).count();
    let by: BTreeMap<(usize, &KcId), u32> = s
        .masteries
        .iter()
        .map(|m| ((m.learner, &m.kc), m.answers))
        .collect();
    let mut paired: Vec<u32> = reference
        .iter()
        .map(|m| by.get(&(m.learner, &m.kc)).copied().unwrap_or(u32::MAX))
        .collect();
    paired.sort_unstable();
    let pm = paired.get(paired.len() / 2).copied().unwrap_or(u32::MAX);
    let censored = paired.iter().filter(|x| **x == u32::MAX).count();
    let fm_bug = |bug: bool| {
        let v: Vec<&Mastery> = s.masteries.iter().filter(|m| m.bug == bug).collect();
        let f = v.iter().filter(|m| m.true7 < 0.7).count();
        (f as f64 / v.len().max(1) as f64, v.len())
    };
    let (fb, nb) = fm_bug(true);
    let (fn_, nn) = fm_bug(false);
    format!(
        "| {name} | {n} | {} | {:.1} | {} ({censored} cens.) | {:.1}% | {:.1}% ({nb}) / {:.1}% ({nn}) | {:.3} | {:.4} |",
        median_u(s.masteries.iter().map(|m| m.answers).collect()),
        median_f(s.masteries.iter().map(|m| m.days).collect()),
        if pm == u32::MAX {
            "∞".to_string()
        } else {
            pm.to_string()
        },
        100.0 * fm as f64 / n.max(1) as f64,
        100.0 * fb,
        100.0 * fn_,
        median_f(s.masteries.iter().map(|m| m.true7).collect()),
        s.log_loss.0 / f64::from(s.log_loss.1.max(1)),
    )
}

const HEADER: &str = "| variant | mastered pairs | median answers | median days | paired median answers | false mastery | FM with bug / without | median true rate +7 d | log loss |\n| --- | --: | --: | --: | --: | --: | --: | --: | --: |";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .map_or("content/bench/v1/bundle.json", String::as_str);
    let num = |i: usize, d: u64| args.get(i).map_or(Some(d), |a| a.parse().ok());
    let (Some(learners), Some(days_n), Some(seed)) = (num(1, 200), num(2, 30), num(3, 1)) else {
        eprintln!("usage: m1 [BUNDLE] [LEARNERS] [DAYS] [SEED]");
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
    let days_n = u32::try_from(days_n).expect("fits");
    let model = KnowledgeModel::default();
    let params = Params::default();
    let mp = &model.params;

    println!("# M1 measurement: {path}, {learners} learners, {days_n} days, seed {seed}\n");

    // The copy of `session_ours` must be the session itself.
    let (runs, mastered, _) = generate(
        &model,
        &bundle,
        &params,
        learners,
        days_n,
        seed,
        Swap::None,
        true,
    );
    let (copy, mastered_copy, _) = generate(
        &model,
        &bundle,
        &params,
        learners,
        days_n,
        seed,
        Swap::None,
        false,
    );
    let same = runs.len() == copy.len()
        && runs
            .iter()
            .zip(&copy)
            .all(|(a, b)| format!("{:?}", a.log) == format!("{:?}", b.log));
    assert!(same, "the session copy differs from session_ours");
    println!(
        "check: session copy = session_ours on every event; mastered pairs {mastered} = {mastered_copy}"
    );
    let answers: usize = runs
        .iter()
        .map(|r| {
            r.log
                .iter()
                .filter(|l| {
                    matches!(&l.event, Decoded::Known(e) if matches!(e.payload, EventPayload::Answer(_)))
                })
                .count()
        })
        .sum();
    println!("answers in the logs: {answers}");

    let base = Variant::base(mp);
    let v1 = replay(&model, &bundle, &runs, base, true);
    println!(
        "check: replica = core fold on every answer (μ, σ², prediction, rapid, mastery bit for bit); {} masteries\n",
        v1.masteries.len()
    );

    // ---- information per answer
    println!(
        "## Information per answer (model formulas, p = 0.8 targeted, s = {})\n",
        mp.slip
    );
    println!("| g | σ | a_eff | P* | I = p'^2/(p(1-p)) | user's (p-g)^2/(1-g)^2 (1-p)/p |");
    println!("| --: | --: | --: | --: | --: | --: |");
    for g in [0.0, 0.25, 1.0 / 3.0] {
        for sd in [1.5_f64, 1.0, 0.5, 0.35] {
            let a = effective_slope(mp.slope, sd * sd);
            let ps = (0.8 - g) / (1.0 - g - mp.slip);
            let dp = (1.0 - g - mp.slip) * a * ps * (1.0 - ps);
            let i = dp * dp / (0.8 * 0.2);
            let user = (0.8 - g) * (0.8 - g) / ((1.0 - g) * (1.0 - g)) * 0.2 / 0.8;
            println!("| {g:.3} | {sd:.2} | {a:.3} | {ps:.3} | {i:.4} | {user:.4} |");
        }
    }
    println!(
        "\nAnswers to shrink σ from 1.5 (prior) to σ*, n = (1/σ*² − 1/1.5²)/Ī, and the σ* the bound needs at μ: σ* = (μ − θ*)/0.84 (R = 1):\n"
    );
    println!("| μ at mastery | σ* | n at Ī = 0.05 | n at Ī = 0.08 | n at Ī = 0.10 |");
    println!("| --: | --: | --: | --: | --: |");
    for mu in [1.8, 2.0, 2.2, 2.5, 3.0] {
        let sd = (mu - mp.mastery_theta) / 0.8416;
        let need = 1.0 / (sd * sd) - 1.0 / 2.25;
        println!(
            "| {mu:.1} | {sd:.3} | {:.0} | {:.0} | {:.0} |",
            (need / 0.05).max(0.0),
            (need / 0.08).max(0.0),
            (need / 0.10).max(0.0)
        );
    }
    let mean_info = |typed: Option<bool>| {
        let v: Vec<f64> = v1
            .info
            .iter()
            .filter(|(t, _)| typed.is_none_or(|x| x == *t))
            .map(|(_, i)| *i)
            .collect();
        (v.iter().sum::<f64>() / v.len().max(1) as f64, v.len())
    };
    let (ia, na) = mean_info(None);
    let (it, nt) = mean_info(Some(true));
    let (ic, nc) = mean_info(Some(false));
    println!(
        "\nMeasured w·I per direct answer on a primary KC before its mastery: all {ia:.4} ({na}), typed {it:.4} ({nt}), choice {ic:.4} ({nc})."
    );
    println!(
        "At mastery: median μ {:.2}, median σ {:.3}; true correct rate on the KC's items at mastery {:.3}, 7 days later {:.3}.",
        median_f(v1.masteries.iter().map(|m| m.mu).collect()),
        median_f(v1.masteries.iter().map(|m| m.sd).collect()),
        median_f(v1.masteries.iter().map(|m| m.true0).collect()),
        median_f(v1.masteries.iter().map(|m| m.true7).collect())
    );
    for bug in [true, false] {
        let v: Vec<u32> = v1
            .masteries
            .iter()
            .filter(|m| m.bug == bug)
            .map(|m| m.answers)
            .collect();
        println!(
            "Mastered KCs {} the learner's own wrong idea: {}, median answers {}.",
            if bug { "with" } else { "without" },
            v.len(),
            median_u(v)
        );
    }
    let k = &v1.known_at;
    println!(
        "At the first answer after which the learner truly knew the KC (true rate on its items ≥ 0.8), {} pairs: median answers {}, model μ {:.2}, σ {:.2}, M {:.2}; the learner's θ on the model's scale {:.2}.\n",
        k.len(),
        median_u(k.iter().map(|x| x.0).collect()),
        median_f(k.iter().map(|x| x.1).collect()),
        median_f(k.iter().map(|x| x.2).collect()),
        median_f(k.iter().map(|x| x.3).collect()),
        median_f(k.iter().map(|x| x.4).collect()),
    );

    // ---- calibration
    println!(
        "## Calibration of M = P(θ_eff > θ*), v1, after every non-rapid answer on the answered primary KC\n"
    );
    println!(
        "Truth: the learner's true correct rate on a medium typed item ≥ (1 − s)·σ(θ*) = {:.3}, which is what θ* stands for.\n",
        (1.0 - mp.slip) * sigmoid(mp.mastery_theta)
    );
    println!("| M bin | answers | mean M | true share above θ* | with the bug |");
    println!("| --- | --: | --: | --: | --: |");
    for bin in 0..10 {
        let lo = f64::from(bin) / 10.0;
        let hi = lo + 0.1;
        let v: Vec<&(f64, bool, bool)> = v1
            .calibration
            .iter()
            .filter(|(m, _, _)| *m >= lo && (*m < hi || (bin == 9 && *m <= 1.0)))
            .collect();
        if v.is_empty() {
            continue;
        }
        let n = v.len() as f64;
        println!(
            "| {lo:.1}–{hi:.1} | {} | {:.3} | {:.3} | {:.3} |",
            v.len(),
            v.iter().map(|x| x.0).sum::<f64>() / n,
            v.iter().filter(|x| x.1).count() as f64 / n,
            v.iter().filter(|x| x.2).count() as f64 / n
        );
    }

    // ---- variants by replay
    let reference = v1.masteries.clone();
    let mut variants: Vec<Variant> = Vec::new();
    let mut add = |name: &'static str, f: &dyn Fn(&mut Variant)| {
        let mut v = base;
        v.name = name;
        f(&mut v);
        variants.push(v);
    };
    add("q = 0.02 per answer", &|v| v.q_answer = 0.02);
    add("q = 0.05 per answer", &|v| v.q_answer = 0.05);
    add("q = 0.1 per answer", &|v| v.q_answer = 0.1);
    add("q = 0.2 per answer", &|v| v.q_answer = 0.2);
    add("q = 0.3 per answer", &|v| v.q_answer = 0.3);
    add("q = 0.02 per day (η)", &|v| v.eta = 0.02);
    add("q = 0.05 per day (η)", &|v| v.eta = 0.05);
    add("q = 0.1 per day (η)", &|v| v.eta = 0.1);
    add("q = 0.2 per day (η)", &|v| v.eta = 0.2);
    add("q = 0.3 per day (η)", &|v| v.eta = 0.3);
    add("slope a = 1.3", &|v| v.slope = 1.3);
    add("γ = 0", &|v| v.gamma = 0.0);
    add("secondaries left out", &|v| v.secondary = Secondary::Drop);
    add("secondaries: oracle P*", &|v| {
        v.secondary = Secondary::Oracle;
    });
    add("w_secondary = 0", &|v| v.w_secondary = 0.0);
    add("typed answers only", &|v| v.typed_only = true);
    add("confidence 0.7", &|v| v.confidence = 0.7);
    add("confidence 0.6", &|v| v.confidence = 0.6);
    add("confidence 0.5", &|v| v.confidence = 0.5);
    add("θ* = 1.2", &|v| v.theta_star = 1.2);
    add("θ* = 1.0", &|v| v.theta_star = 1.0);
    add("ρ = 0 (no propagation)", &|v| v.rho = 0.0);
    add("δ = 0", &|v| v.delta = 0.0);
    add("η = 0", &|v| v.eta = 0.0);
    add("δ = 0, η = 0", &|v| {
        v.delta = 0.0;
        v.eta = 0.0;
    });
    add("raw a in p'", &|v| v.raw_slope_in_dp = true);
    add("η 0.05 + confidence 0.7", &|v| {
        v.eta = 0.05;
        v.confidence = 0.7;
    });
    add("η 0.05 + θ* = 1.2", &|v| {
        v.eta = 0.05;
        v.theta_star = 1.2;
    });
    add("q 0.02/answer + θ* = 1.2", &|v| {
        v.q_answer = 0.02;
        v.theta_star = 1.2;
    });
    add("q 0.02/answer + η 0.05", &|v| {
        v.q_answer = 0.02;
        v.eta = 0.05;
    });
    add("q 0.05/answer + raw a", &|v| {
        v.q_answer = 0.05;
        v.raw_slope_in_dp = true;
    });
    add("q 0.05/answer + γ 0", &|v| {
        v.q_answer = 0.05;
        v.gamma = 0.0;
    });
    add("q 0.1/answer + confidence 0.7", &|v| {
        v.q_answer = 0.1;
        v.confidence = 0.7;
    });
    add("q 0.05/answer + raw a + γ 0", &|v| {
        v.q_answer = 0.05;
        v.raw_slope_in_dp = true;
        v.gamma = 0.0;
    });
    add("q 0.1/answer + raw a + γ 0", &|v| {
        v.q_answer = 0.1;
        v.raw_slope_in_dp = true;
        v.gamma = 0.0;
    });

    println!("\n## Variants, replayed on the same logs (off-policy)\n");
    println!(
        "Paired median: over the {} (learner, KC) pairs v1 mastered, a pair the variant never masters counts as ∞. False mastery: true correct rate on the KC's items (wrong idea and secondary KCs included) 7 days after mastery without reviews below 0.7.\n",
        reference.len()
    );
    println!("{HEADER}");
    println!("{}", row(base.name, &v1, &reference));
    for v in &variants {
        let s = replay(&model, &bundle, &runs, *v, false);
        println!("{}", row(v.name, &s, &reference));
    }

    // ---- on-policy runs
    println!("\n## On-policy runs: the planner sees the variant\n");
    println!(
        "Model parameters that `KnowledgeModel::Params` holds run with the planner reading them; planner-side swaps run through the session copy. Scored by the same variant's replica.\n"
    );
    println!(
        "| run | mastered per learner | minutes per learner | items swapped | mastered pairs | median answers | median days | paired median answers | false mastery | FM with bug / without | median true rate +7 d | log loss |"
    );
    println!("| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: | --: |");
    let on = |name: &str, m: &KnowledgeModel, v: Variant, swap: Swap| {
        let (runs, mastered, swapped) =
            generate(m, &bundle, &params, learners, days_n, seed, swap, false);
        let s = replay(m, &bundle, &runs, v, false);
        let minutes = runs.iter().map(|r| r.minutes).sum::<f64>() / learners as f64;
        let r = row(name, &s, &reference);
        let rest = &r[r.find(" | ").expect("a row") + 3..];
        println!(
            "| {name} | {:.2} | {minutes:.1} | {swapped} | {rest}",
            mastered as f64 / learners as f64
        );
    };
    on("v1 as is", &model, base, Swap::None);
    on("planner: measurement items", &model, base, Swap::Measure);
    on("planner: typed near mastery", &model, base, Swap::Typed);
    let mut runs_on: Vec<(&str, refl_core::model::Params)> = Vec::new();
    let mut p = mp.clone();
    p.mastery_confidence = 0.7;
    runs_on.push(("confidence 0.7", p));
    let mut p = mp.clone();
    p.mastery_confidence = 0.6;
    runs_on.push(("confidence 0.6", p));
    let mut p = mp.clone();
    p.mastery_theta = 1.2;
    runs_on.push(("θ* = 1.2", p));
    let mut p = mp.clone();
    p.unsure_gamma = 0.0;
    runs_on.push(("γ = 0", p));
    for (name, eta) in [
        ("η = 0.02 per day", 0.02),
        ("η = 0.05 per day", 0.05),
        ("η = 0.1 per day", 0.1),
        ("η = 0.2 per day", 0.2),
    ] {
        let mut p = mp.clone();
        p.eta = eta;
        runs_on.push((name, p));
    }
    let mut p = mp.clone();
    p.eta = 0.05;
    p.mastery_confidence = 0.7;
    runs_on.push(("η = 0.05, confidence 0.7", p));
    let mut p = mp.clone();
    p.eta = 0.05;
    p.mastery_theta = 1.2;
    runs_on.push(("η = 0.05, θ* = 1.2", p));
    for (name, p) in runs_on {
        let m = KnowledgeModel { params: p };
        let v = Variant {
            name: "on-policy",
            ..Variant::base(&m.params)
        };
        on(name, &m, v, Swap::None);
    }
    ExitCode::SUCCESS
}
