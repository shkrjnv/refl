//! Learner model version 1 (`docs/03-layer2-learner-model.md`; the v1
//! readings are in «Уточнения версии 1»): knowledge μ/σ with the observation
//! model, FSRS-6 memory, automaticity, misconceptions, the rapid-guess
//! filter and propagation of success down the `requires` graph.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::fsrs::{self, Card, Grade};
use super::math::{self, KcTerm};
use super::state::{KcState, MisconceptionState, State};
use super::{
    AnswerOutcome, Applied, FoldCtx, LearnerModel, NativeLanguage, NeedsReplay, Profile, event_id,
};
use crate::answer::normalize::{NormalizeOptions, normalize};
use crate::bundle::{Bundle, KcEntry};
use crate::content::{Explanation, ExplanationTarget, Misconception};
use crate::event::{
    AnswerContext, AnswerEvent, Confidence, Decoded, Event, EventPayload, Response, Verdict,
};
use crate::ids::{ExplanationId, ItemRef, KcId, MisconceptionId, SubjectId};
use crate::item::KcRole;
use crate::projection::ItemProjection;
use crate::subject::Subject;
use crate::value::Timestamp;

pub(super) const DAY_MS: i64 = 86_400_000;

/// Parameters of version 1 with their starting values
/// (`docs/03-layer2-learner-model.md`, «Самопроверка и параметры»). They
/// are part of the model version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// s: random slip.
    pub slip: f64,
    /// γ: share of correct "unsure" answers taken as guessed.
    pub unsure_gamma: f64,
    /// a: discrimination.
    pub slope: f64,
    /// σ₀: starting uncertainty.
    pub sigma0: f64,
    /// Starting μ is clamped to ±this.
    pub mu0_bound: f64,
    /// δ: strength of forgetting in the current skill.
    pub delta: f64,
    /// η: growth of σ² per day without practice.
    pub eta: f64,
    /// λ: smoothing of τ.
    pub lambda: f64,
    /// τ at or above this is automatic.
    pub tau_threshold: f64,
    /// Answer time is capped at this many expected times.
    pub time_cap: f64,
    /// ρ: propagation of success to prerequisites.
    pub rho: f64,
    pub w_secondary: f64,
    pub w_hint: f64,
    pub w_retry: f64,
    pub w_unrecognized: f64,
    /// n₀: weight of the misconception prior in pseudo-observations.
    pub n0: f64,
    /// Share of the deviation from the prior a misconception keeps per week
    /// without observations.
    pub misconception_decay: f64,
    /// α increment of a misconception shown with "sure".
    pub sure_error_weight: f64,
    pub misconception_active_m: f64,
    pub misconception_active_count: u32,
    pub misconception_cured_m: f64,
    pub misconception_cured_n: f64,
    /// θ*: mastery threshold.
    pub mastery_theta: f64,
    /// Required `P(θ_eff > θ*)`.
    pub mastery_confidence: f64,
    /// Correct production answers on distinct items.
    pub production_needed: usize,
    /// Stability for «надолго», days.
    pub long_term_stability: f64,
    /// R below this: «забывается».
    pub forgetting_r: f64,
    /// Rapid guess: time below this share of the expected time...
    pub rapid_share: f64,
    /// ...and, for typed input, below this many milliseconds.
    pub rapid_typed_ms: u32,
    /// FSRS grade: |ln(expected / actual)| beyond this is slow or fast.
    pub tempo_band: f64,
    /// FSRS-6 weights w₀…w₂₀.
    pub fsrs: [f64; 21],
    /// Monte Carlo samples for the level interval, and their fixed seed.
    pub level_samples: u32,
    pub level_seed: u64,
    /// Return after a break (`docs/03-layer2-learner-model.md`, «Перерыв и
    /// возвращение»).
    pub return_min_days: f64,
    pub return_mean_r: f64,
    pub return_forgetting_share: f64,
    pub return_always_days: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            slip: 0.05,
            unsure_gamma: 0.3,
            slope: 1.0,
            sigma0: 1.5,
            mu0_bound: 3.0,
            delta: 1.0,
            eta: 0.005,
            lambda: 0.2,
            tau_threshold: -0.2,
            time_cap: 5.0,
            rho: 0.3,
            w_secondary: 0.3,
            w_hint: 0.5,
            w_retry: 0.3,
            w_unrecognized: 0.5,
            n0: 2.0,
            misconception_decay: 0.97,
            sure_error_weight: 1.5,
            misconception_active_m: 0.35,
            misconception_active_count: 2,
            misconception_cured_m: 0.15,
            misconception_cured_n: 8.0,
            mastery_theta: 1.5,
            mastery_confidence: 0.8,
            production_needed: 2,
            long_term_stability: 21.0,
            forgetting_r: 0.8,
            rapid_share: 0.25,
            rapid_typed_ms: 2000,
            tempo_band: 0.2,
            fsrs: fsrs::DEFAULT_WEIGHTS,
            level_samples: 1000,
            level_seed: 0x5EED_1E7E_1000,
            return_min_days: 7.0,
            return_mean_r: 0.85,
            return_forgetting_share: 0.2,
            return_always_days: 14.0,
        }
    }
}

/// Learner model version 1.
#[derive(Debug, Clone, Default)]
pub struct KnowledgeModel {
    pub params: Params,
}

/// Belief about one KC at a moment: μ, σ² with the growth since the last
/// update, and R.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Belief {
    pub mu: f64,
    pub var: f64,
    pub r: f64,
}

/// Days from `from` to `to`, never negative (device clocks can step back).
#[allow(clippy::cast_precision_loss)] // timestamps are within 2^53
pub(super) fn days(from: Timestamp, to: Timestamp) -> f64 {
    (to.0 - from.0).max(0) as f64 / DAY_MS as f64
}

/// The UTC day of a timestamp: FSRS counts one answer per KC per day.
fn utc_day(t: Timestamp) -> i64 {
    t.0.div_euclid(DAY_MS)
}

fn find<'a, T, K: Ord + ?Sized>(v: &'a [T], key: &K, f: impl Fn(&T) -> &K) -> Option<&'a T> {
    v.binary_search_by(|x| f(x).cmp(key)).ok().map(|i| &v[i])
}

pub(super) fn kc_entry<'b>(bundle: &'b Bundle, kc: &KcId) -> Option<&'b KcEntry> {
    find(&bundle.content.kcs, kc, |e| &e.kc.id)
}

pub(super) fn subject<'b>(bundle: &'b Bundle, id: &SubjectId) -> Option<&'b Subject> {
    find(&bundle.content.subjects, id, |s| &s.id)
}

fn misconception<'b>(bundle: &'b Bundle, id: &MisconceptionId) -> Option<&'b Misconception> {
    find(&bundle.content.misconceptions, id, |m| &m.id)
}

fn explanation<'b>(bundle: &'b Bundle, id: &ExplanationId) -> Option<&'b Explanation> {
    find(&bundle.content.explanations, id, |e| &e.id)
}

/// A typed answer whose normalized words are all letters only: it shows
/// spelling, unlike numbers, contractions or hyphenated forms.
fn letter_only(response: &Response) -> bool {
    let Response::Text(text) = response else {
        return false;
    };
    let words = normalize(text, NormalizeOptions::default());
    !words.is_empty() && words.iter().all(|w| w.chars().all(char::is_alphabetic))
}

/// IRT difficulty from the author's 1–5: b ∈ {−2, …, 2}.
fn difficulty(item: &ItemProjection) -> f64 {
    f64::from(item.difficulty.get()) - 3.0
}

/// One observation of the knowledge model: the slot's KCs with weights, or
/// a single pseudo-observation.
struct Observation<'a> {
    kcs: &'a [(KcId, f64)],
    b: f64,
    g: f64,
    slip: f64,
    correct: bool,
    /// Counts as a real answer on these KCs (not propagation).
    direct: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tempo {
    Slow,
    Normal,
    Fast,
}

impl KnowledgeModel {
    /// Starting belief: `μ₀ = clamp(h (L − c), ±3)`, `σ₀ = 1.5`; `None` if
    /// the KC or its subject is not in the bundle.
    pub fn prior(&self, profile: &Profile, bundle: &Bundle, kc: &KcId) -> Option<(f64, f64)> {
        let entry = kc_entry(bundle, kc)?;
        let subject = subject(bundle, &entry.kc.subject)?;
        let level = profile
            .start_level
            .get(&subject.id)
            .map_or(0.0, |l| l.get());
        let h = subject.scale.step_logit.get();
        let bound = self.params.mu0_bound;
        let mu = (h * (level - f64::from(entry.level_rank))).clamp(-bound, bound);
        Some((mu, self.params.sigma0 * self.params.sigma0))
    }

    /// R of a card at `now`; 1 without a card (nothing to forget).
    pub fn recall(&self, card: Option<&Card>, now: Timestamp) -> f64 {
        card.map_or(1.0, |c| {
            fsrs::retrievability(&self.params.fsrs, days(c.last_review, now), c.stability)
        })
    }

    pub fn belief(&self, s: &State, bundle: &Bundle, kc: &KcId, now: Timestamp) -> Option<Belief> {
        if let Some(k) = s.kcs.get(kc) {
            Some(Belief {
                mu: k.mu,
                var: k.var + self.params.eta * days(k.var_at, now),
                r: self.recall(k.card.as_ref(), now),
            })
        } else {
            let (mu, var) = self.prior(&s.profile, bundle, kc)?;
            Some(Belief { mu, var, r: 1.0 })
        }
    }

    /// `θ_eff = μ − δ(1 − R)`.
    pub fn theta_eff(&self, b: Belief) -> f64 {
        b.mu - self.params.delta * (1.0 - b.r)
    }

    /// `M = P(θ_eff > θ*) = Φ((θ_eff − θ*) / σ)`.
    pub fn mastery_probability(&self, b: Belief) -> f64 {
        math::phi((self.theta_eff(b) - self.params.mastery_theta) / libm::sqrt(b.var))
    }

    fn term(&self, b: Belief, item_b: f64) -> KcTerm {
        KcTerm::new(
            self.theta_eff(b),
            item_b,
            math::effective_slope(self.params.slope, b.var),
        )
    }

    fn predict_slot(
        &self,
        s: &State,
        bundle: &Bundle,
        item: &ItemProjection,
        slot_index: u16,
        now: Timestamp,
    ) -> Option<f64> {
        let slot = item.slots.get(usize::from(slot_index))?;
        let b = difficulty(item);
        let terms = slot
            .kcs
            .iter()
            .map(|k| Some(self.term(self.belief(s, bundle, &k.kc, now)?, b)))
            .collect::<Option<Vec<_>>>()?;
        Some(math::predict(
            item.guessing(slot_index),
            self.params.slip,
            &terms,
        ))
    }

    /// The KC's state with σ² grown to `t`, created from the prior on first
    /// touch; `None` if the KC is not in the bundle.
    fn touch<'s>(
        &self,
        s: &'s mut State,
        bundle: &Bundle,
        kc: &KcId,
        t: Timestamp,
    ) -> Option<&'s mut KcState> {
        if !s.kcs.contains_key(kc) {
            let (mu, var) = self.prior(&s.profile, bundle, kc)?;
            s.kcs.insert(
                kc.clone(),
                KcState {
                    mu,
                    var,
                    var_at: t,
                    observations: 0,
                    card: None,
                    tau: 0.0,
                    timed: 0,
                    production: Vec::new(),
                    ever_mastered: false,
                },
            );
        }
        let k = s.kcs.get_mut(kc)?;
        if t > k.var_at {
            k.var += self.params.eta * days(k.var_at, t);
            k.var_at = t;
        }
        Some(k)
    }

    /// One Newton step on every KC of the observation at time `t`.
    fn observe(&self, s: &mut State, bundle: &Bundle, o: &Observation<'_>, t: Timestamp) {
        let mut kcs = Vec::with_capacity(o.kcs.len());
        let mut terms = Vec::with_capacity(o.kcs.len());
        for (kc, w) in o.kcs {
            let Some(k) = self.touch(s, bundle, kc, t) else {
                continue;
            };
            let belief = Belief {
                mu: k.mu,
                var: k.var,
                r: self.recall(k.card.as_ref(), t),
            };
            terms.push(self.term(belief, o.b));
            kcs.push((kc, *w));
        }
        let evidence = math::evidence(o.g, o.slip, o.correct, &terms);
        for ((kc, w), e) in kcs.into_iter().zip(evidence) {
            let k = s.kcs.get_mut(kc).expect("touched above");
            let (mu, var) = math::posterior(k.mu, k.var, w, e);
            k.mu = mu;
            k.var = var;
            if o.direct && w > 0.0 {
                k.observations += 1;
            }
        }
    }

    /// The criterion of «Когда тема считается освоенной»: confident
    /// knowledge, automaticity and production.
    pub fn is_mastered(&self, k: &KcState, now: Timestamp) -> bool {
        let p = &self.params;
        let belief = Belief {
            mu: k.mu,
            var: k.var + p.eta * days(k.var_at, now),
            r: self.recall(k.card.as_ref(), now),
        };
        self.mastery_probability(belief) >= p.mastery_confidence
            && k.timed > 0
            && k.tau >= p.tau_threshold
            && k.production.len() >= p.production_needed
    }

    fn tempo(&self, a: &AnswerEvent, expected_ms: f64) -> Tempo {
        if a.backgrounded || expected_ms <= 0.0 {
            return Tempo::Normal;
        }
        let x = libm::log(expected_ms / f64::from(a.latency_ms.max(1)));
        if x < -self.params.tempo_band {
            Tempo::Slow
        } else if x > self.params.tempo_band {
            Tempo::Fast
        } else {
            Tempo::Normal
        }
    }

    /// The misconception's state with decay applied up to `t`, created from
    /// the content prior on first touch.
    fn touch_misconception<'s>(
        &self,
        s: &'s mut State,
        bundle: &Bundle,
        id: &MisconceptionId,
        t: Timestamp,
    ) -> Option<&'s mut MisconceptionState> {
        if !s.misconceptions.contains_key(id) {
            let prior = misconception(bundle, id)?.prior;
            let pi = match s.profile.l1 {
                Some(NativeLanguage::Uz) => prior.uz.get(),
                Some(NativeLanguage::Ru) => prior.ru.get(),
                None => f64::midpoint(prior.uz.get(), prior.ru.get()),
            };
            let (alpha0, beta0) = (self.params.n0 * pi, self.params.n0 * (1.0 - pi));
            s.misconceptions.insert(
                id.clone(),
                MisconceptionState {
                    alpha: alpha0,
                    beta: beta0,
                    alpha0,
                    beta0,
                    updated_at: t,
                    manifested: 0,
                },
            );
        }
        let m = s.misconceptions.get_mut(id)?;
        let (alpha, beta) = self.decayed(m, t);
        m.alpha = alpha;
        m.beta = beta;
        m.updated_at = m.updated_at.max(t);
        Some(m)
    }

    /// α and β at `now`: the deviation from the prior shrinks by the decay
    /// factor for every whole week since the last update.
    pub fn decayed(&self, m: &MisconceptionState, now: Timestamp) -> (f64, f64) {
        let weeks = libm::floor(days(m.updated_at, now) / 7.0);
        let f = libm::pow(self.params.misconception_decay, weeks);
        (
            m.alpha0 + (m.alpha - m.alpha0) * f,
            m.beta0 + (m.beta - m.beta0) * f,
        )
    }

    // The five updates of one answer, in order; names follow docs/03.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn answer(&self, s: &mut State, ev: &Event, a: &AnswerEvent, ctx: &FoldCtx<'_>) -> Applied {
        let p = &self.params;
        let bundle = ctx.bundle;
        let t = ev.client_ts;
        let shown = Timestamp(t.0 - i64::from(a.latency_ms));
        let Some(item) = bundle.projection(&a.item) else {
            s.unresolved += 1;
            return Applied::Unresolved;
        };
        let (Some(slot), Some(predicted)) = (
            item.slots.get(usize::from(a.slot)),
            self.predict_slot(s, bundle, item, a.slot, shown),
        ) else {
            s.unresolved += 1;
            return Applied::Unresolved;
        };
        let (verdict, misconception) = match ctx.corrections.get(&ev.event_id) {
            Some(c) => (c.verdict, c.misconception.clone()),
            None => (a.verdict, a.misconception.clone()),
        };
        let correct = matches!(verdict, Verdict::Correct | Verdict::Typo);
        let last = s.last_answer.entry(item.subject.clone()).or_insert(t);
        *last = (*last).max(t);

        let expected_ms = f64::from(item.expected_time) * 1000.0;
        let latency = f64::from(a.latency_ms);
        let rapid = !a.backgrounded
            && expected_ms > 0.0
            && latency < p.rapid_share * expected_ms
            && (!item.typed_input() || a.latency_ms < p.rapid_typed_ms);
        let outcome = AnswerOutcome {
            predicted,
            correct,
            rapid,
            subject: item.subject.clone(),
            context: a.context,
            ladder: item.ladder(),
        };
        if rapid {
            s.rapid.total += 1;
            s.rapid.streak += 1;
            return Applied::Answer(outcome);
        }
        s.rapid.streak = 0;

        // Knowledge: confidence adjusts g and s, hints and attempts adjust w.
        let mut g = item.guessing(a.slot);
        let mut slip = p.slip;
        if correct && a.confidence == Some(Confidence::Unsure) {
            g += (1.0 - g) * p.unsure_gamma;
        }
        let sure_error = !correct && a.confidence == Some(Confidence::Sure);
        if sure_error {
            slip /= 2.0;
        }
        let mut modifier = 1.0;
        if correct && a.hints > 0 {
            modifier *= p.w_hint;
        }
        if a.attempt >= 2 {
            modifier *= p.w_retry;
        }
        if verdict == Verdict::Unrecognized {
            modifier *= p.w_unrecognized;
        }
        let weighted: Vec<(KcId, f64)> = slot
            .kcs
            .iter()
            .map(|k| {
                let role = match k.role {
                    KcRole::Primary => 1.0,
                    KcRole::Secondary => p.w_secondary,
                };
                (k.kc.clone(), role * modifier)
            })
            .collect();
        let b = difficulty(item);
        self.observe(
            s,
            bundle,
            &Observation {
                kcs: &weighted,
                b,
                g,
                slip,
                correct,
                direct: true,
            },
            t,
        );
        let mut touched: BTreeSet<KcId> = weighted.iter().map(|(k, _)| k.clone()).collect();
        // Spelling: a typo fails the subject's spelling KC with the primary
        // weight; a correct letter-only typed answer succeeds on it with the
        // secondary weight.
        let spelling = match verdict {
            Verdict::Typo => Some((false, modifier)),
            Verdict::Correct if item.typed_input() && letter_only(&a.response) => {
                Some((true, p.w_secondary * modifier))
            }
            _ => None,
        };
        if let Some((spelled, w)) = spelling
            && let Some(subject) = subject(bundle, &item.subject)
        {
            self.observe(
                s,
                bundle,
                &Observation {
                    kcs: &[(subject.spelling_kc.clone(), w)],
                    b: 0.0,
                    g: 0.0,
                    slip: p.slip,
                    correct: spelled,
                    direct: true,
                },
                t,
            );
            touched.insert(subject.spelling_kc.clone());
        }

        // Memory, automaticity and production: the slot's primary KCs.
        let grade = if correct {
            let tempo = self.tempo(a, expected_ms);
            if a.confidence == Some(Confidence::Unsure) || a.hints > 0 || tempo == Tempo::Slow {
                Grade::Hard
            } else if tempo == Tempo::Fast {
                Grade::Easy
            } else {
                Grade::Good
            }
        } else {
            Grade::Again
        };
        let today = utc_day(t);
        for kc in slot.kcs.iter().filter(|k| k.role == KcRole::Primary) {
            let may_start = a.context != AnswerContext::Lesson || s.explained.contains(&kc.kc);
            let Some(k) = s.kcs.get_mut(&kc.kc) else {
                continue;
            };
            match k.card {
                Some(card) => {
                    let last_day = utc_day(card.last_review);
                    if today > last_day {
                        #[allow(clippy::cast_precision_loss)] // whole days
                        let elapsed = (today - last_day) as f64;
                        k.card = Some(fsrs::review(&p.fsrs, &card, elapsed, grade, t));
                    }
                }
                None if correct && may_start => k.card = Some(fsrs::start(&p.fsrs, grade, t)),
                None => {}
            }
            if correct && !a.backgrounded && expected_ms > 0.0 {
                let time = latency.max(1.0).min(p.time_cap * expected_ms);
                k.tau += p.lambda * (libm::log(expected_ms / time) - k.tau);
                k.timed += 1;
            }
            if correct
                && item.is_production()
                && k.production.len() < p.production_needed
                && !k.production.contains(&a.item.id)
            {
                k.production.push(a.item.id.clone());
            }
        }

        // Misconceptions: a shown one gains α, a resisted one gains β.
        if verdict == Verdict::Wrong
            && let Some(x) = &misconception
            && let Some(m) = self.touch_misconception(s, bundle, x, t)
        {
            m.alpha += if sure_error { p.sure_error_weight } else { 1.0 };
            m.manifested += 1;
        }
        if correct {
            for x in &slot.misconceptions {
                if let Some(m) = self.touch_misconception(s, bundle, x, t) {
                    m.beta += 1.0;
                }
            }
        }

        // Propagation: success flows down `requires` with weight w·ρ^d.
        if correct {
            let mut pseudo: BTreeMap<&KcId, f64> = BTreeMap::new();
            for (kc, w) in &weighted {
                for r in bundle
                    .content
                    .requires_closure
                    .get(kc)
                    .into_iter()
                    .flatten()
                {
                    if slot.kcs.iter().any(|k| k.kc == r.kc) {
                        continue;
                    }
                    let wr = w * libm::pow(p.rho, f64::from(r.distance));
                    let e = pseudo.entry(&r.kc).or_insert(0.0);
                    *e = e.max(wr);
                }
            }
            for (kc, w) in pseudo {
                self.observe(
                    s,
                    bundle,
                    &Observation {
                        kcs: &[(kc.clone(), w)],
                        b: 0.0,
                        g: 0.0,
                        slip: p.slip,
                        correct: true,
                        direct: false,
                    },
                    t,
                );
                touched.insert(kc.clone());
            }
        }

        for kc in &touched {
            if let Some(k) = s.kcs.get(kc)
                && !k.ever_mastered
                && self.is_mastered(k, t)
            {
                s.kcs.get_mut(kc).expect("present").ever_mastered = true;
            }
        }
        Applied::Answer(outcome)
    }
}

impl LearnerModel for KnowledgeModel {
    type State = State;

    fn version(&self) -> String {
        let params = serde_json::to_vec(&self.params).expect("params serialize");
        let hash = Sha256::digest(params);
        let short = hash[..8].iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        });
        format!("knowledge-v1+{short}")
    }

    fn init(&self, profile: &Profile) -> State {
        State::new(profile.clone())
    }

    fn predict(
        &self,
        s: &State,
        bundle: &Bundle,
        item: &ItemRef,
        slot: u16,
        now: Timestamp,
    ) -> Option<f64> {
        self.predict_slot(s, bundle, bundle.projection(item)?, slot, now)
    }

    fn apply(
        &self,
        s: &mut State,
        ev: &Decoded,
        ctx: &FoldCtx<'_>,
    ) -> Result<Applied, NeedsReplay> {
        let id = *event_id(ev);
        if let Some(position) = s.position {
            if id == position {
                return Ok(Applied::Duplicate);
            }
            if id < position {
                return Err(NeedsReplay::Late);
            }
        }
        if let Decoded::Known(e) = ev
            && let EventPayload::Reclassified(r) = &e.payload
            && ctx.corrections.get(&r.original).is_none_or(|c| c.by < id)
        {
            return Err(NeedsReplay::Reclassified);
        }
        s.position = Some(id);
        let Decoded::Known(e) = ev else {
            s.incomplete = true;
            return Ok(Applied::Unknown);
        };
        Ok(match &e.payload {
            EventPayload::Answer(a) => self.answer(s, e, a, ctx),
            EventPayload::ExplanationViewed { explanation: id, .. } => {
                if let Some(ExplanationTarget::Kc(kc)) =
                    explanation(ctx.bundle, id).map(|x| &x.target)
                {
                    s.explained.insert(kc.clone());
                }
                Applied::Skipped
            }
            // Applied through the corrections, in the place of the answer.
            EventPayload::Reclassified(_)
            // A planner decision (docs/11, «Лог решений»).
            | EventPayload::PlanIssued(_)
            // mark_known has no weight yet (docs/03, open question 8).
            | EventPayload::MentorOverride(_)
            | EventPayload::LessonStarted { .. }
            | EventPayload::LessonFinished { .. }
            | EventPayload::MilestoneReached { .. }
            | EventPayload::AchievementEarned { .. }
            | EventPayload::AchievementVisibilitySet { .. }
            | EventPayload::UnlockGranted { .. }
            | EventPayload::UnlockRevoked { .. }
            | EventPayload::ThemeSelected { .. }
            | EventPayload::CompanionSelected { .. }
            | EventPayload::CosmeticSelected { .. }
            | EventPayload::ShowcaseSet { .. }
            | EventPayload::WritingScored(_) => Applied::Skipped,
        })
    }
}
