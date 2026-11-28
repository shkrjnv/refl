//! Learner model version 1.3 (`docs/03-layer2-learner-model.md`; the v1
//! readings are in «Уточнения версии 1»; 1.1 added the process noise q, 1.2
//! applies it before the prediction, 1.3 replaces automaticity by fluency
//! from retrieval time): knowledge μ/σ with the observation model, FSRS-6
//! memory, fluency, misconceptions, the rapid-guess filter and propagation
//! of success down the `requires` graph.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::fluency;
use super::fsrs::{self, Card, Grade};
use super::math::{self, KcTerm};
use super::state::{
    BookMarkRecord, CustomLexemeRecord, KcState, LexicalCard, LexicalDirection, LexicalSource,
    MisconceptionState, NoteRecord, State,
};
use super::{
    AnswerOutcome, Applied, FoldCtx, LearnerModel, NativeLanguage, NeedsReplay, Profile, event_id,
};
use crate::answer::normalize::{NormalizeOptions, normalize};
use crate::bundle::{Bundle, KcEntry};
use crate::content::{Explanation, ExplanationTarget, Misconception};
use crate::event::OverrideKind;
use crate::event::{
    AnswerContext, AnswerEvent, BookMarkCreatedEvent, BookMarkDeletedEvent, Confidence,
    CustomLexemeArchivedEvent, CustomLexemeEvent, Decoded, Event, EventPayload, InputDevice,
    NoteAnchor, NoteDeletedEvent, NoteSavedEvent, Response, Verdict,
};
use crate::ids::{BookId, ExplanationId, ItemRef, KcId, LexemeId, MisconceptionId, SubjectId};
use crate::item::KcRole;
use crate::projection::ItemProjection;
use crate::subject::Subject;
use crate::value::{Finite, Timestamp};

pub(super) const DAY_MS: i64 = 86_400_000;
const DAY_MS_F: f64 = 86_400_000.0;

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
    /// q: process noise, σ² added to every slot KC at the start of
    /// processing an answer, before the prediction (versions 1.1–1.2).
    pub process_noise: f64,
    /// λ: smoothing of τ.
    pub lambda: f64,
    /// Fluency, branch 1: τ at or above this.
    pub tau_threshold: f64,
    /// Fluency, branch 2: τ − bⱼ at or above this...
    pub tau_relative_threshold: f64,
    /// ...and the spread at most this.
    pub spread_max: f64,
    /// Counted answers on a KC before it can be fluent.
    pub fluency_min_counted: u32,
    /// Residuals of a KC the spread is taken over.
    pub spread_window: usize,
    /// bⱼ takes counted answers whose primary KCs all have M above this...
    pub baseline_mastery: f64,
    /// ...shrunk to 0 with this many virtual answers.
    pub baseline_n0: f64,
    /// An item answered within this many days is not counted: the planner's
    /// seen-family window ν (a prior, like the instruction rule).
    pub unseen_days: f64,
    /// Typing beyond this many times the learner's usual typing time counts
    /// as retrieval.
    pub typing_tolerance: f64,
    /// Typing speed: samples kept per input device...
    pub typing_window: usize,
    /// ...shrunk to the prior with this many virtual samples (proposal)...
    pub typing_n0: f64,
    /// ...priors v₀, characters per second (proposal).
    pub typing_v0_touch: f64,
    pub typing_v0_keyboard: f64,
    /// Retrieval time is capped at this many `exp(βᶠ)`.
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
    /// FSRS grade: a counted answer's |r| beyond this is slow or fast.
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
            process_noise: 0.02,
            lambda: 0.2,
            tau_threshold: -0.2,
            tau_relative_threshold: -0.2,
            spread_max: 0.35,
            fluency_min_counted: 6,
            spread_window: 8,
            baseline_mastery: 0.8,
            baseline_n0: 20.0,
            unseen_days: 3.0,
            typing_tolerance: 1.5,
            typing_window: 30,
            typing_n0: 5.0,
            typing_v0_touch: 1.5,
            typing_v0_keyboard: 3.0,
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

/// When a lexical card becomes due: `at + ceil(stability)` days. FSRS caps
/// stability, so the millisecond value fits; the cast saturates anyway.
#[allow(clippy::cast_possible_truncation)]
fn lexical_due(at: Timestamp, stability: f64) -> Timestamp {
    Timestamp(at.0.saturating_add(libm::ceil(stability.max(0.0) * DAY_MS_F) as i64))
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

/// The parts of the answer's time exceed the whole: `first_input_ms`,
/// `first_input_ms + typing_ms` or `hidden_ms` above `latency_ms`
/// (`docs/03-layer2-learner-model.md`, «Согласованность времени»).
fn timing_inconsistent(a: &AnswerEvent) -> bool {
    let whole = u64::from(a.latency_ms);
    let part = |ms: Option<u32>| u64::from(ms.unwrap_or(0));
    part(a.first_input_ms) + part(a.typing_ms) > whole || part(a.hidden_ms) > whole
}

/// The fluency condition of one KC and its parts
/// (`docs/03-layer2-learner-model.md`, «Беглость: время извлечения»).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fluency {
    pub counted: u32,
    pub tau: f64,
    /// bⱼ of the KC's subject.
    pub baseline: f64,
    /// Over the KC's latest counted answers; `None` without any.
    pub spread: Option<f64>,
    /// Branch 1: enough counted answers and τ ≥ the threshold.
    pub absolute: bool,
    /// Branch 2: enough counted answers, τ − bⱼ ≥ its threshold and a small
    /// spread.
    pub relative: bool,
}

impl Fluency {
    pub fn holds(&self) -> bool {
        self.absolute || self.relative
    }
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

    /// The belief of `kc` at `now` widened by `noise`: the σ² of an answer
    /// not yet processed, whose process noise is not in the state yet.
    fn noisy_belief(
        &self,
        s: &State,
        bundle: &Bundle,
        kc: &KcId,
        now: Timestamp,
        noise: f64,
    ) -> Option<Belief> {
        let mut b = self.belief(s, bundle, kc, now)?;
        b.var += noise;
        Some(b)
    }

    /// `noise` is `q` for a state where the answer's process noise is not
    /// applied yet, and 0 after the fold applied it.
    fn predict_slot(
        &self,
        s: &State,
        bundle: &Bundle,
        item: &ItemProjection,
        slot_index: u16,
        now: Timestamp,
        noise: f64,
    ) -> Option<f64> {
        let slot = item.slots.get(usize::from(slot_index))?;
        let b = difficulty(item);
        let terms = slot
            .kcs
            .iter()
            .map(|k| Some(self.term(self.noisy_belief(s, bundle, &k.kc, now, noise)?, b)))
            .collect::<Option<Vec<_>>>()?;
        Some(math::predict(
            item.guessing(slot_index),
            self.params.slip,
            &terms,
        ))
    }

    /// Fisher information of `slot` of `item` about `kc` at `now`, from the
    /// same response model as [`LearnerModel::predict`]; the planner's Ī
    /// (`docs/04-placement-planner.md`, «Следующее задание внутри блока»).
    /// `None` if the bundle has no such item version or slot, or `kc` is not
    /// in the slot.
    pub fn information(
        &self,
        s: &State,
        bundle: &Bundle,
        item: &ItemRef,
        slot: u16,
        kc: &KcId,
        now: Timestamp,
    ) -> Option<f64> {
        let item = bundle.projection(item)?;
        let kcs = &item.slots.get(usize::from(slot))?.kcs;
        let index = kcs.iter().position(|k| &k.kc == kc)?;
        let b = difficulty(item);
        let noise = self.params.process_noise;
        let terms = kcs
            .iter()
            .map(|k| Some(self.term(self.noisy_belief(s, bundle, &k.kc, now, noise)?, b)))
            .collect::<Option<Vec<_>>>()?;
        // The information does not depend on the outcome.
        let evidence = math::evidence(item.guessing(slot), self.params.slip, true, &terms);
        Some(evidence[index].info)
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
                    counted: 0,
                    residuals: Vec::new(),
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

    /// A mentor's mark «тема известна»: one correct observation of the KC at
    /// medium difficulty (b = 0, g = 0) with the secondary weight. Only μ and
    /// σ change: no memory, fluency, production, propagation or
    /// misconception update, so a mark alone never masters a KC
    /// (`docs/03-layer2-learner-model.md`, «Уточнения версии 1»).
    fn mark_known(&self, s: &mut State, bundle: &Bundle, kc: &KcId, t: Timestamp) {
        let p = &self.params;
        self.observe(
            s,
            bundle,
            &Observation {
                kcs: &[(kc.clone(), p.w_secondary)],
                b: 0.0,
                g: 0.0,
                slip: p.slip,
                correct: true,
                direct: true,
            },
            t,
        );
    }

    /// The criterion of «Когда тема считается освоенной»: confident
    /// knowledge, fluency and production. `false` for a KC without updates.
    pub fn is_mastered(&self, s: &State, bundle: &Bundle, kc: &KcId, now: Timestamp) -> bool {
        let p = &self.params;
        let Some(k) = s.kcs.get(kc) else {
            return false;
        };
        let belief = Belief {
            mu: k.mu,
            var: k.var + p.eta * days(k.var_at, now),
            r: self.recall(k.card.as_ref(), now),
        };
        self.mastery_probability(belief) >= p.mastery_confidence
            && self.fluency(s, bundle, kc).holds()
            && k.production.len() >= p.production_needed
    }

    /// bⱼ of `subject`: `Σr / (n + n₀)`, 0 without counted answers.
    pub fn baseline(&self, s: &State, subject: &SubjectId) -> f64 {
        s.baseline
            .get(subject)
            .map_or(0.0, |b| b.sum / (f64::from(b.n) + self.params.baseline_n0))
    }

    /// The fluency condition of `kc`; a KC without updates has no counted
    /// answers and is not fluent.
    pub fn fluency(&self, s: &State, bundle: &Bundle, kc: &KcId) -> Fluency {
        let p = &self.params;
        let baseline = kc_entry(bundle, kc).map_or(0.0, |e| self.baseline(s, &e.kc.subject));
        let (counted, tau, spread) = s.kcs.get(kc).map_or((0, 0.0, None), |k| {
            (k.counted, k.tau, fluency::spread(&k.residuals))
        });
        let enough = counted >= p.fluency_min_counted;
        Fluency {
            counted,
            tau,
            baseline,
            spread,
            absolute: enough && tau >= p.tau_threshold,
            relative: enough
                && tau - baseline >= p.tau_relative_threshold
                && spread.is_some_and(|x| x <= p.spread_max),
        }
    }

    /// vⱼ of `device` from the counted typed answers so far.
    pub fn typing_speed(&self, s: &State, device: InputDevice) -> f64 {
        let p = &self.params;
        let v0 = match device {
            InputDevice::Touch => p.typing_v0_touch,
            InputDevice::Keyboard => p.typing_v0_keyboard,
        };
        let samples = s.typing.get(&device).map_or(&[][..], Vec::as_slice);
        fluency::shrunk_speed(samples, v0, p.typing_n0)
    }

    /// `t_ret` in seconds, before the cap; `None` if the answer lacks a
    /// timing field its kind needs.
    fn retrieval_seconds(&self, s: &State, a: &AnswerEvent, typed: bool) -> Option<f64> {
        let device = a.input_device?;
        let first = f64::from(a.first_input_ms?) / 1000.0;
        if !typed {
            return Some(first);
        }
        let typing = f64::from(a.typing_ms?) / 1000.0;
        let chars = f64::from(a.chars_typed?);
        let usual = self.params.typing_tolerance * chars / self.typing_speed(s, device);
        Some(first + (typing - usual).max(0.0))
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

    fn save_custom_lexeme(s: &mut State, ev: &Event, c: &CustomLexemeEvent) {
        if s.custom_lexemes
            .get(&c.custom_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        let record = CustomLexemeRecord {
            custom_id: c.custom_id,
            source: c.source,
            bundle_lexeme_id: c.bundle_lexeme_id.clone(),
            lemma: c.lemma.clone(),
            pos: c.pos.clone(),
            definition_en: c.definition_en.clone(),
            gloss_uz: c.gloss_uz.clone(),
            gloss_ru: c.gloss_ru.clone(),
            examples: c.examples.clone(),
            collocations: c.collocations.clone(),
            ipa: c.ipa.clone(),
            audio_key: c.audio_key.clone(),
            context: c.context.clone(),
            archived: c.archived,
            event_id: ev.event_id,
            updated_at: ev.client_ts,
        };
        s.custom_lexemes.insert(c.custom_id, record);
        if c.bundle_lexeme_id.is_none() {
            let lexeme = LexemeId::new(c.custom_id.0.to_string()).expect("a UUID is a valid slug");
            for direction in [LexicalDirection::Recognize, LexicalDirection::Produce] {
                if !s.has_lexical_card(&lexeme, direction) {
                    s.insert_lexical_card(
                        &lexeme,
                        LexicalCard {
                            stability: 1.0,
                            difficulty: 5.0,
                            last_review: ev.client_ts,
                            due: Timestamp(ev.client_ts.0 + DAY_MS),
                            direction,
                            source: LexicalSource::Custom,
                            exposures: 0.0,
                            review_count: 0,
                            lapse_count: 0,
                        },
                    );
                }
            }
        }
    }

    fn archive_custom_lexeme(s: &mut State, ev: &Event, c: &CustomLexemeArchivedEvent) {
        if s.custom_lexemes
            .get(&c.custom_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        let record = s
            .custom_lexemes
            .entry(c.custom_id)
            .or_insert_with(|| CustomLexemeRecord {
                custom_id: c.custom_id,
                source: crate::event::CustomLexemeSource::Own,
                bundle_lexeme_id: None,
                lemma: String::new(),
                pos: None,
                definition_en: String::new(),
                gloss_uz: String::new(),
                gloss_ru: String::new(),
                examples: Vec::new(),
                collocations: Vec::new(),
                ipa: None,
                audio_key: None,
                context: None,
                archived: true,
                event_id: ev.event_id,
                updated_at: ev.client_ts,
            });
        record.archived = c.archived;
        record.event_id = ev.event_id;
        record.updated_at = ev.client_ts;
    }

    fn save_book_mark(s: &mut State, ev: &Event, m: &BookMarkCreatedEvent) {
        if s.book_marks
            .get(&m.mark_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        s.book_marks.insert(
            m.mark_id,
            BookMarkRecord {
                mark_id: m.mark_id,
                book_id: m.book_id.clone(),
                book_version: m.book_version.clone(),
                kind: m.kind,
                chapter_id: m.chapter_id.clone(),
                paragraph_id: m.paragraph_id.clone(),
                sentence_id: m.sentence_id.clone(),
                start_token: m.start_token,
                end_token: m.end_token,
                quote_hash: m.quote_hash.clone(),
                color: m.color.clone(),
                deleted: false,
                event_id: ev.event_id,
                updated_at: ev.client_ts,
            },
        );
    }

    fn delete_book_mark(s: &mut State, ev: &Event, m: &BookMarkDeletedEvent) {
        if s.book_marks
            .get(&m.mark_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        let record = s
            .book_marks
            .entry(m.mark_id)
            .or_insert_with(|| BookMarkRecord {
                mark_id: m.mark_id,
                book_id: BookId::new("book.deleted").expect("valid placeholder"),
                book_version: String::new(),
                kind: crate::event::BookMarkKind::Bookmark,
                chapter_id: String::new(),
                paragraph_id: String::new(),
                sentence_id: String::new(),
                start_token: 0,
                end_token: 0,
                quote_hash: String::new(),
                color: None,
                deleted: true,
                event_id: ev.event_id,
                updated_at: ev.client_ts,
            });
        record.deleted = true;
        record.event_id = ev.event_id;
        record.updated_at = ev.client_ts;
    }

    fn save_note(s: &mut State, ev: &Event, n: &NoteSavedEvent) {
        if s.notes
            .get(&n.note_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        s.notes.insert(
            n.note_id,
            NoteRecord {
                note_id: n.note_id,
                anchor: n.anchor.clone(),
                body: n.body.clone(),
                deleted: false,
                event_id: ev.event_id,
                updated_at: ev.client_ts,
            },
        );
    }

    fn delete_note(s: &mut State, ev: &Event, n: &NoteDeletedEvent) {
        if s.notes
            .get(&n.note_id)
            .is_some_and(|record| record.event_id >= ev.event_id)
        {
            return;
        }
        let record = s.notes.entry(n.note_id).or_insert_with(|| NoteRecord {
            note_id: n.note_id,
            anchor: NoteAnchor {
                kind: crate::event::NoteAnchorKind::Standalone,
                id: None,
            },
            body: String::new(),
            deleted: true,
            event_id: ev.event_id,
            updated_at: ev.client_ts,
        });
        record.deleted = true;
        record.event_id = ev.event_id;
        record.updated_at = ev.client_ts;
    }

    /// Direct lexical evidence: a placement value check, a lexical review,
    /// or an explicit "add to vocabulary".
    pub fn review_lexeme(
        &self,
        s: &mut State,
        lexeme: &LexemeId,
        direction: LexicalDirection,
        correct: bool,
        at: Timestamp,
        source: LexicalSource,
    ) -> LexicalCard {
        let grade = if correct { Grade::Good } else { Grade::Again };
        let card = match s.lexical_card(lexeme, direction) {
            Some(old) if old.review_count > 0 => {
                let elapsed = days(old.last_review, at).max(1.0);
                let fsrs_card = Card {
                    stability: old.stability,
                    difficulty: old.difficulty,
                    last_review: old.last_review,
                };
                let next = fsrs::review(&self.params.fsrs, &fsrs_card, elapsed, grade, at);
                LexicalCard {
                    stability: next.stability,
                    difficulty: next.difficulty,
                    last_review: at,
                    due: lexical_due(at, next.stability),
                    direction,
                    source,
                    exposures: old.exposures,
                    review_count: old.review_count.saturating_add(1),
                    lapse_count: old.lapse_count + u32::from(!correct),
                }
            }
            _ => {
                let next = fsrs::start(&self.params.fsrs, grade, at);
                LexicalCard {
                    stability: next.stability,
                    difficulty: next.difficulty,
                    last_review: at,
                    due: lexical_due(at, next.stability),
                    direction,
                    source,
                    exposures: 0.0,
                    review_count: 1,
                    lapse_count: u32::from(!correct),
                }
            }
        };
        s.insert_lexical_card(lexeme, card.clone());
        card
    }

    /// Placement Stage 2 hook: seed the two cards from a frequency-band
    /// answer. Unknown claims create nothing; a checked meaning is a direct
    /// review.
    pub fn placement_lexeme_prior(
        &self,
        s: &mut State,
        lexeme: &LexemeId,
        frequency_band: u16,
        known: bool,
        at: Timestamp,
    ) {
        if !known {
            return;
        }
        let stability = 1.0 + f64::from(frequency_band);
        for direction in [LexicalDirection::Recognize, LexicalDirection::Produce] {
            if s.has_lexical_card(lexeme, direction) {
                continue;
            }
            s.insert_lexical_card(
                lexeme,
                LexicalCard {
                    stability,
                    difficulty: 5.0,
                    last_review: at,
                    due: lexical_due(at, stability),
                    direction,
                    source: LexicalSource::Bundle,
                    exposures: 0.0,
                    review_count: 1,
                    lapse_count: 0,
                },
            );
        }
    }

    /// `R` of one lexical card at `now`; 0 without a direct card.
    pub fn lexical_recall(
        &self,
        s: &State,
        lexeme: &LexemeId,
        direction: LexicalDirection,
        now: Timestamp,
    ) -> f64 {
        s.lexical_card(lexeme, direction).map_or(0.0, |card| {
            fsrs::retrievability(
                &self.params.fsrs,
                days(card.last_review, now),
                card.stability,
            )
        })
    }

    /// A correct, unhinted first attempt lends weak recognition evidence to
    /// the lexemes of the item. The arguments are the fields of the answer
    /// that decide eligibility.
    #[allow(clippy::too_many_arguments)]
    fn credit_lexemes(
        s: &mut State,
        bundle: &Bundle,
        item: &ItemRef,
        family: &crate::ids::FamilyId,
        correct: bool,
        rapid: bool,
        hints: u8,
        attempt: u8,
        at: Timestamp,
    ) {
        if !correct || rapid || hints > 0 || attempt > 1 {
            return;
        }
        let Some(lexemes) = bundle.content.item_lexemes.get(&item.id) else {
            return;
        };
        for lexeme in lexemes {
            let born = {
                let exposure = s.lexical_exposures.entry(lexeme.clone()).or_default();
                exposure.count = exposure.count.saturating_add(1);
                exposure.items.insert(item.id.clone());
                exposure.families.insert(family.clone());
                exposure.items.len() >= 3 && exposure.families.len() >= 2
            };
            if let Some(card) = s
                .lexical_cards
                .get_mut(lexeme)
                .and_then(|cards| cards.get_mut(&LexicalDirection::Recognize))
            {
                // Passive meetings never move the FSRS interval; they only
                // add a weak 0.15 of recognition evidence.
                card.exposures += 0.15;
            } else if born {
                s.insert_lexical_card(
                    lexeme,
                    LexicalCard {
                        stability: 1.0,
                        difficulty: 5.0,
                        last_review: at,
                        due: Timestamp(at.0 + DAY_MS),
                        direction: LexicalDirection::Recognize,
                        source: LexicalSource::Bundle,
                        exposures: 0.45,
                        review_count: 0,
                        lapse_count: 0,
                    },
                );
            }
        }
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
        let (Some(slot), Some(predicted_noisy)) = (
            item.slots.get(usize::from(a.slot)),
            self.predict_slot(s, bundle, item, a.slot, shown, p.process_noise),
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

        // Fluency bookkeeping of every answer, rapid guesses included: the
        // instructions shown in this session and the items answered lately.
        let family = ctx.families.get(&item.family);
        let instruction = family.map(|f| &f.instruction);
        if s.session != ev.session_id {
            s.session = ev.session_id;
            s.session_instructions.clear();
        }
        // The first answer under an instruction in a session, and any answer
        // outside a known session, does not count.
        let known_instruction =
            s.session.is_some() && instruction.is_some_and(|i| s.session_instructions.contains(i));
        if let Some(i) = instruction {
            s.session_instructions.insert(i.clone());
        }
        let unseen = s
            .seen
            .get(&a.item.id)
            .is_none_or(|&at| days(at, shown) >= p.unseen_days);
        let inconsistent = timing_inconsistent(a);
        s.timing_inconsistent += u32::from(inconsistent);
        s.seen.retain(|_, at| days(*at, shown) < p.unseen_days);
        let seen = s.seen.entry(a.item.id.clone()).or_insert(shown);
        *seen = (*seen).max(shown);

        let expected_ms = f64::from(item.expected_time) * 1000.0;
        let latency = f64::from(a.latency_ms);
        let hidden = a.hidden_ms.is_some_and(|ms| ms > 0);
        let rapid = !hidden
            && expected_ms > 0.0
            && latency < p.rapid_share * expected_ms
            && (!item.typed_input() || a.latency_ms < p.rapid_typed_ms);
        Self::credit_lexemes(
            s,
            bundle,
            &a.item,
            &item.family,
            correct,
            rapid,
            a.hints,
            a.attempt,
            t,
        );
        // Fluency: the residual r = βᶠ − ln t_ret of a counted answer, and
        // whether the slot's primary KCs were known at display, before any
        // update of this answer (bⱼ).
        let residual = match family {
            Some(f)
                if correct
                    && a.attempt <= 1
                    && a.hints == 0
                    && !rapid
                    && a.hidden_ms == Some(0)
                    && !inconsistent
                    && known_instruction
                    && unseen =>
            {
                self.retrieval_seconds(s, a, item.typed_input()).map(|t| {
                    let cap = p.time_cap * libm::exp(f.beta);
                    f.beta - libm::log(t.clamp(0.001, cap))
                })
            }
            _ => None,
        };
        let known = residual.is_some()
            && slot
                .kcs
                .iter()
                .filter(|k| k.role == KcRole::Primary)
                .all(|k| {
                    self.belief(s, bundle, &k.kc, shown)
                        .is_some_and(|b| self.mastery_probability(b) > p.baseline_mastery)
                });
        // Process noise: the skill changes while the learner learns. σ²
        // grows by η·Δt + q at the start of processing, so the prediction
        // and the update below use the same σ². A rapid guess updates
        // nothing and only reports the prediction as `predict` made it.
        let predicted = if rapid {
            predicted_noisy
        } else {
            for k in &slot.kcs {
                if let Some(state) = self.touch(s, bundle, &k.kc, shown) {
                    state.var += p.process_noise;
                }
            }
            self.predict_slot(s, bundle, item, a.slot, shown, 0.0)
                .expect("the slot resolved above")
        };
        let outcome = AnswerOutcome {
            predicted,
            correct,
            rapid,
            timing_inconsistent: inconsistent,
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
        // The slot's KCs were widened at `shown` above; the update uses the
        // same moment, so its σ² is the one the prediction used.
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
            shown,
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

        // Memory, fluency and production: the slot's primary KCs. The tempo
        // of an answer not counted for fluency is unknown: Good.
        let grade = if correct {
            let tempo = residual.unwrap_or(0.0);
            if a.confidence == Some(Confidence::Unsure) || a.hints > 0 || tempo < -p.tempo_band {
                Grade::Hard
            } else if tempo > p.tempo_band {
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
            if let Some(r) = residual {
                k.tau += p.lambda * (r - k.tau);
                k.counted += 1;
                k.residuals.push(r);
                if k.residuals.len() > p.spread_window {
                    k.residuals.remove(0);
                }
            }
            if correct
                && item.is_production()
                && k.production.len() < p.production_needed
                && !k.production.contains(&a.item.id)
            {
                k.production.push(a.item.id.clone());
            }
        }
        // A counted answer also feeds vⱼ (typed, with a measurable typing
        // time) and, on known KCs, bⱼ.
        if let Some(r) = residual {
            if item.typed_input()
                && let (Some(device), Some(ms), Some(chars)) =
                    (a.input_device, a.typing_ms, a.chars_typed)
                && ms > 0
                && chars > 0
            {
                let samples = s.typing.entry(device).or_default();
                samples.push(f64::from(chars) * 1000.0 / f64::from(ms));
                if samples.len() > p.typing_window {
                    samples.remove(0);
                }
            }
            if known {
                let b = s.baseline.entry(item.subject.clone()).or_default();
                b.sum += r;
                b.n += 1;
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
            let newly_mastered = s.kcs.get(kc).is_some_and(|k| !k.ever_mastered)
                && self.is_mastered(s, bundle, kc, t);
            if newly_mastered {
                s.kcs.get_mut(kc).expect("present").ever_mastered = true;
                s.mastered_at.entry(kc.clone()).or_insert(t);
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
        format!("knowledge-v1.4+{short}")
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
        let noise = self.params.process_noise;
        self.predict_slot(s, bundle, bundle.projection(item)?, slot, now, noise)
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
            EventPayload::MentorOverride(m) => {
                if let OverrideKind::MarkKnown { kc, .. } = &m.kind {
                    self.mark_known(s, ctx.bundle, kc, e.client_ts);
                }
                // Plan changes and disputed verdicts are not the model's.
                Applied::Skipped
            }
            EventPayload::PlacementCompleted(p) => {
                // Merge per subject; a later placement for one subject keeps
                // another. Clamp to the subject's scale when it exists.
                for (subject_id, level) in &p.start_level {
                    let Some(subject) = subject(ctx.bundle, subject_id) else {
                        continue;
                    };
                    let max = f64::from(
                        u16::try_from(subject.scale.levels.len())
                            .unwrap_or(u16::MAX)
                            .saturating_sub(1),
                    );
                    if let Some(level) = Finite::new(level.get().clamp(0.0, max)) {
                        s.profile.start_level.insert(subject_id.clone(), level);
                    }
                }
                Applied::Skipped
            }
            EventPayload::CustomLexemeCreated(c) | EventPayload::CustomLexemeUpdated(c) => {
                Self::save_custom_lexeme(s, e, c);
                Applied::Skipped
            }
            EventPayload::CustomLexemeArchived(c) => {
                Self::archive_custom_lexeme(s, e, c);
                Applied::Skipped
            }
            EventPayload::BookMarkCreated(m) => {
                Self::save_book_mark(s, e, m);
                Applied::Skipped
            }
            EventPayload::BookMarkDeleted(m) => {
                Self::delete_book_mark(s, e, m);
                Applied::Skipped
            }
            EventPayload::NoteSaved(n) => {
                Self::save_note(s, e, n);
                Applied::Skipped
            }
            EventPayload::NoteDeleted(n) => {
                Self::delete_note(s, e, n);
                Applied::Skipped
            }
            // Applied through the corrections, in the place of the answer.
            EventPayload::Reclassified(_)
            // A planner decision (docs/11, «Лог решений»).
            | EventPayload::PlanIssued(_)
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
