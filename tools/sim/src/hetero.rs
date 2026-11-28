//! W24 heterogeneous learner cohort: six archetypes, two L1 backgrounds,
//! planted L1 misconceptions, and three item-selection strategies.
//!
//! The simulated learner is deliberately a different model from the core:
//! a hidden true skill per KC, true prerequisites, true active
//! misconceptions, slips, rapid guesses and its own forgetting. The core
//! model only sees the answer events. Strategy 1 ignores the graph and the
//! diagnoses (a Leitner queue); strategy 2 uses the model's KC uncertainty
//! but not the graph or misconceptions; strategy 3 adds prerequisite
//! routing, misconception targeting, expected variance reduction per second
//! and FSRS due reviews.

// Simulation and report code: integer counters become report floats, and the
// ordered cohort/report functions are intentionally long.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::cohort::difficulty_shift;
use crate::{T0, TextReview, event_in, reindex, session};
use refl_core::bundle::{Bundle, KcStatus};
use refl_core::content::MisconceptionOrigin;
use refl_core::event::{AnswerContext, AnswerEvent, EventPayload, InputDevice, Response, Verdict};
use refl_core::ids::{DomainId, FamilyId, ItemRef, KcId, MisconceptionId, SubjectId};
use refl_core::item::{ItemStatus, KcRole};
use refl_core::model::{
    Families, FoldCtx, KnowledgeModel, LearnerModel, NativeLanguage, Profile, State, math::sigmoid,
};
use refl_core::placement::{self, Placement};
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use refl_core::rng::SplitMix64;
use refl_core::value::Timestamp;

const DAY_MS: i64 = 86_400_000;
const CHECKPOINTS: [usize; 4] = [30, 60, 100, 200];
const MASTERY: f64 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Archetype {
    Zero,
    Spiky,
    Fossilized,
    FastSlip,
    Guesser,
    Steady,
}

impl Archetype {
    pub const ALL: [Self; 6] = [
        Self::Zero,
        Self::Spiky,
        Self::Fossilized,
        Self::FastSlip,
        Self::Guesser,
        Self::Steady,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Zero => "True zero",
            Self::Spiky => "Spiky / uneven",
            Self::Fossilized => "L1-fossilized",
            Self::FastSlip => "Fast + high slip",
            Self::Guesser => "Impulsive guesser",
            Self::Steady => "Steady average",
        }
    }

    fn params(self) -> (f64, f64, f64, f64, f64) {
        // learn, forget, slip, rapid, relapse
        match self {
            Self::Zero => (0.20, 1.0, 0.05, 0.03, 0.02),
            Self::Spiky => (0.34, 0.8, 0.06, 0.02, 0.02),
            Self::Fossilized => (0.24, 1.1, 0.07, 0.04, 0.10),
            Self::FastSlip => (0.55, 0.6, 0.18, 0.04, 0.01),
            Self::Guesser => (0.30, 1.8, 0.08, 0.35, 0.03),
            Self::Steady => (0.34, 0.9, 0.06, 0.03, 0.02),
        }
    }

    fn plant_probability(self) -> f64 {
        match self {
            Self::Zero => 0.85,
            Self::Spiky => 0.50,
            Self::Fossilized => 0.92,
            Self::FastSlip => 0.12,
            Self::Guesser => 0.35,
            Self::Steady => 0.45,
        }
    }

    fn base(self, family: Family, rng: &mut SplitMix64) -> f64 {
        let value = match self {
            Self::Zero => -2.0,
            Self::Steady => 0.0,
            Self::Fossilized => -0.2,
            Self::FastSlip => 3.0,
            Self::Guesser => 0.3,
            Self::Spiky => match family {
                Family::Be | Family::Nouns => 2.5,
                Family::Articles | Family::Present => -0.5,
                Family::Other => 0.5,
            },
        };
        value + 0.25 * rng.next_normal()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum L1 {
    Uz,
    Ru,
}

impl L1 {
    fn name(self) -> &'static str {
        match self {
            Self::Uz => "uz",
            Self::Ru => "ru",
        }
    }

    fn native(self) -> NativeLanguage {
        match self {
            Self::Uz => NativeLanguage::Uz,
            Self::Ru => NativeLanguage::Ru,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Articles,
    Be,
    Present,
    Nouns,
    Other,
}

fn family(kc: &KcId) -> Family {
    let id = kc.as_str();
    if id.contains(".article.") || id.contains("noun.countability") {
        Family::Articles
    } else if id.contains(".be.") || id.contains("there-is-are") {
        Family::Be
    } else if id.contains("present-simple") || id.contains("present-continuous") {
        Family::Present
    } else if id.contains("noun.")
        || id.contains("pronoun.")
        || id.contains("possessive.")
        || id.contains("determiner.")
    {
        Family::Nouns
    } else {
        Family::Other
    }
}

#[derive(Debug, Clone)]
pub struct SlotSpec {
    pub item: ItemRef,
    pub slot: u16,
    pub primary: Vec<KcId>,
    pub secondary: Vec<KcId>,
    pub misconceptions: Vec<MisconceptionId>,
    pub answer: String,
    pub typed: bool,
    pub options: Vec<(String, Option<MisconceptionId>)>,
    pub wrong: Vec<(String, MisconceptionId)>,
    pub expected_time: u32,
    pub family: FamilyId,
    /// The learner's true prerequisites, used only by strategy 3.
    pub blocked_by: Vec<KcId>,
}

/// Promote every compiled KC and item to live for the test fixture, so the
/// real 32-KC graph can be simulated before a human review.
pub fn promote(bundle: &mut Bundle) {
    for entry in &mut bundle.content.kcs {
        entry.status = KcStatus::Live;
    }
    for entry in &mut bundle.content.items {
        entry.item.status = ItemStatus::Live;
    }
    // Draft items have no published projection in the compiled bundle; the
    // fixture builds them from the item bodies so the core can resolve them.
    bundle.content.item_versions = bundle
        .content
        .items
        .iter()
        .map(|entry| ItemVersionEntry {
            item: entry.item.id.clone(),
            version: entry.item.version,
            projection: ItemProjection::of(&entry.item),
        })
        .collect();
    bundle
        .content
        .item_versions
        .sort_by(|a, b| (&a.item, a.version).cmp(&(&b.item, b.version)));
    reindex(bundle, TextReview::Ignored);
}

pub fn learning_kcs(bundle: &Bundle) -> Vec<&refl_core::bundle::KcEntry> {
    bundle
        .content
        .kcs
        .iter()
        .filter(|e| e.status == KcStatus::Live && !e.kc.observed_only)
        .filter(|e| {
            bundle
                .content
                .items_by_kc
                .get(&e.kc.id)
                .is_some_and(|items| !items.primary.is_empty())
        })
        .collect()
}

pub fn slots(bundle: &Bundle) -> Vec<SlotSpec> {
    let mut out = Vec::new();
    for entry in &bundle.content.items {
        if !entry.session_eligible {
            continue;
        }
        let item_ref = ItemRef {
            id: entry.item.id.clone(),
            version: entry.item.version,
            seed: None,
        };
        let Some(projection) = bundle.projection(&item_ref) else {
            continue;
        };
        for (index, body_slot) in entry.item.body.slots().iter().enumerate() {
            let Ok(slot) = u16::try_from(index) else {
                continue;
            };
            let Some(_row) = projection.slots.get(index) else {
                continue;
            };
            let primary: Vec<KcId> = body_slot
                .kcs
                .iter()
                .filter(|k| k.role == KcRole::Primary)
                .map(|k| k.kc.clone())
                .collect();
            if primary.is_empty() {
                continue;
            }
            let secondary: Vec<KcId> = body_slot
                .kcs
                .iter()
                .filter(|k| k.role == KcRole::Secondary)
                .map(|k| k.kc.clone())
                .collect();
            let mut misconceptions = BTreeSet::new();
            for option in &body_slot.options {
                if let Some(m) = &option.misconception {
                    misconceptions.insert(m.clone());
                }
            }
            for wrong in &body_slot.expected_wrong {
                misconceptions.insert(wrong.misconception.clone());
            }
            out.push(SlotSpec {
                item: item_ref.clone(),
                slot,
                primary,
                secondary,
                misconceptions: misconceptions.into_iter().collect(),
                answer: body_slot.answer.clone(),
                typed: projection.typed_input(),
                options: body_slot
                    .options
                    .iter()
                    .map(|o| (o.text.clone(), o.misconception.clone()))
                    .collect(),
                wrong: body_slot
                    .expected_wrong
                    .iter()
                    .map(|w| (w.pattern.0.clone(), w.misconception.clone()))
                    .collect(),
                expected_time: projection.expected_time.max(1),
                family: projection.family.clone(),
                blocked_by: Vec::new(),
            });
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct SimLearner {
    pub archetype: Archetype,
    pub l1: L1,
    rng: SplitMix64,
    time_rng: SplitMix64,
    clock: i64,
    learn_rate: f64,
    forget_rate: f64,
    slip: f64,
    rapid: f64,
    relapse: f64,
    base: BTreeMap<KcId, f64>,
    gain: BTreeMap<KcId, f64>,
    stability: BTreeMap<KcId, f64>,
    last: BTreeMap<KcId, Timestamp>,
    active: BTreeMap<MisconceptionId, f64>,
    affects: BTreeMap<KcId, Vec<MisconceptionId>>,
    requires: BTreeMap<KcId, Vec<KcId>>,
    slots_by_kc: BTreeMap<KcId, Vec<usize>>,
}

impl SimLearner {
    pub fn new(bundle: &Bundle, index: usize, seed: u64, specs: &[SlotSpec]) -> Self {
        let archetype = Archetype::ALL[index % Archetype::ALL.len()];
        let l1 = if (index / Archetype::ALL.len()).is_multiple_of(2) {
            L1::Uz
        } else {
            L1::Ru
        };
        let mut rng = SplitMix64::new(seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (learn_rate, forget_rate, slip, rapid, relapse) = archetype.params();
        let mut base = BTreeMap::new();
        let mut slots_by_kc: BTreeMap<KcId, Vec<usize>> = BTreeMap::new();
        for (i, slot) in specs.iter().enumerate() {
            for kc in &slot.primary {
                slots_by_kc.entry(kc.clone()).or_default().push(i);
            }
        }
        for entry in learning_kcs(bundle) {
            base.insert(
                entry.kc.id.clone(),
                archetype.base(family(&entry.kc.id), &mut rng),
            );
        }
        let mut active = BTreeMap::new();
        let mut affects: BTreeMap<KcId, Vec<MisconceptionId>> = BTreeMap::new();
        for mis in &bundle.content.misconceptions {
            for kc in &mis.affects {
                affects.entry(kc.clone()).or_default().push(mis.id.clone());
            }
            let applies = match l1 {
                L1::Uz => mis.l1.uz,
                L1::Ru => mis.l1.ru,
            };
            if mis.origin == MisconceptionOrigin::L1Transfer
                && applies
                && rng.next_f64() < archetype.plant_probability()
            {
                let strength = match archetype {
                    Archetype::Fossilized => 0.85 + 0.10 * rng.next_f64(),
                    Archetype::Zero => 0.70 + 0.20 * rng.next_f64(),
                    Archetype::Spiky => 0.50 + 0.20 * rng.next_f64(),
                    Archetype::Guesser | Archetype::Steady => 0.45 + 0.20 * rng.next_f64(),
                    Archetype::FastSlip => 0.15 + 0.15 * rng.next_f64(),
                };
                active.insert(mis.id.clone(), strength);
            }
        }
        let mut requires = BTreeMap::new();
        for (kc, reqs) in &bundle.content.requires_closure {
            let direct: Vec<KcId> = reqs
                .iter()
                .filter(|r| r.distance == 1)
                .map(|r| r.kc.clone())
                .collect();
            requires.insert(kc.clone(), direct);
        }
        Self {
            archetype,
            l1,
            rng: rng.clone(),
            time_rng: SplitMix64::new(seed ^ 0x71E5_0000_0000_0001),
            clock: T0.0,
            learn_rate,
            forget_rate,
            slip,
            rapid,
            relapse,
            base,
            gain: BTreeMap::new(),
            stability: BTreeMap::new(),
            last: BTreeMap::new(),
            active,
            affects,
            requires,
            slots_by_kc,
        }
    }

    fn days(from: Timestamp, to: Timestamp) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        {
            (to.0 - from.0).max(0) as f64 / DAY_MS as f64
        }
    }

    fn skill(&self, kc: &KcId, at: Timestamp) -> f64 {
        let base = self.base.get(kc).copied().unwrap_or(-3.0);
        let gain = self.gain.get(kc).copied().unwrap_or(0.0);
        let stability = self.stability.get(kc).copied().unwrap_or(1.0);
        let kept = self.last.get(kc).map_or(1.0, |last| {
            libm::exp(-self.forget_rate * Self::days(*last, at) / stability)
        });
        let penalty = self
            .affects
            .get(kc)
            .and_then(|ids| {
                ids.iter()
                    .filter_map(|id| self.active.get(id))
                    .copied()
                    .fold(None, |acc: Option<f64>, s| {
                        Some(acc.map_or(s, |a| a.max(s)))
                    })
            })
            .unwrap_or(0.0);
        base + gain * kept - 0.9 * penalty
    }

    fn readiness(&self, kc: &KcId, at: Timestamp) -> f64 {
        self.requires.get(kc).map_or(1.0, |reqs| {
            reqs.iter()
                .map(|r| if self.skill(r, at) < 0.0 { 0.4 } else { 1.0 })
                .product()
        })
    }

    fn practise(&mut self, kc: &KcId, at: Timestamp, weight: f64) {
        let now = self.skill(kc, at);
        let step = weight * self.learn_rate * self.readiness(kc, at) * (1.0 - sigmoid(now - 2.5));
        let base = self.base.get(kc).copied().unwrap_or(-3.0);
        let spaced = self
            .last
            .get(kc)
            .is_none_or(|last| Self::days(*last, at) >= 0.5);
        if let Some(stability) = self.stability.get_mut(kc) {
            if spaced && self.last.contains_key(kc) {
                *stability *= 1.6;
            } else if !spaced {
                *stability += 0.1;
            }
        } else {
            self.stability.insert(kc.clone(), 1.0);
        }
        self.gain.insert(kc.clone(), now - base + step);
        self.last.insert(kc.clone(), at);
    }

    fn strongest_active(&self, kc: &KcId) -> Option<(MisconceptionId, f64)> {
        self.affects.get(kc).and_then(|ids| {
            ids.iter()
                .filter_map(|id| self.active.get(id).map(|s| (id.clone(), *s)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
        })
    }

    pub fn active_count(&self) -> usize {
        self.active.values().filter(|s| **s > 0.5).count()
    }

    fn targeted(&self, spec: &SlotSpec) -> Option<(MisconceptionId, f64)> {
        spec.misconceptions
            .iter()
            .filter_map(|id| self.active.get(id).map(|s| (id.clone(), *s)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    pub fn mastery(&self, bundle: &Bundle, specs: &[SlotSpec], kc: &KcId, at: Timestamp) -> f64 {
        let Some(indices) = self.slots_by_kc.get(kc) else {
            return 0.0;
        };
        let mut sum = 0.0;
        let mut n = 0.0;
        for &i in indices {
            let spec = &specs[i];
            let Some(projection) = bundle.projection(&spec.item) else {
                continue;
            };
            let difficulty =
                f64::from(projection.difficulty.get()) - 3.0 + difficulty_shift(&spec.item);
            // Latent mastery: the hidden skill, without the careless-slip and
            // guessing terms used when the simulated learner actually answers.
            // A high-slip learner can therefore be genuinely mastered while
            // still making observed mistakes, which is the point of that
            // archetype.
            let skill: f64 = spec
                .primary
                .iter()
                .chain(&spec.secondary)
                .map(|kc| sigmoid(1.3 * (self.skill(kc, at) - difficulty)))
                .product();
            let readiness: f64 = spec
                .primary
                .iter()
                .map(|kc| self.readiness(kc, at))
                .product();
            let penalty = spec
                .primary
                .iter()
                .filter_map(|kc| self.strongest_active(kc))
                .map(|(_, s)| s)
                .fold(0.0, f64::max);
            sum += (skill * readiness * (1.0 - 0.25 * penalty)).clamp(0.0, 1.0);
            n += 1.0;
        }
        if n == 0.0 { 0.0 } else { sum / n }
    }

    fn answer(
        &mut self,
        bundle: &Bundle,
        spec: &SlotSpec,
        at: Timestamp,
        placement: bool,
        remedy: bool,
    ) -> (AnswerEvent, bool, Option<MisconceptionId>) {
        let Some(projection) = bundle.projection(&spec.item) else {
            panic!("the simulated item is in the bundle");
        };
        let difficulty =
            f64::from(projection.difficulty.get()) - 3.0 + difficulty_shift(&spec.item);
        let g = projection.guessing(spec.slot);
        let readiness: f64 = spec
            .primary
            .iter()
            .map(|kc| self.readiness(kc, at))
            .product();
        let skill: f64 = spec
            .primary
            .iter()
            .chain(&spec.secondary)
            .map(|kc| sigmoid(1.3 * (self.skill(kc, at) - difficulty)))
            .product();
        let targeted = self.targeted(spec);
        let mis_penalty = targeted.as_ref().map_or(0.0, |(_, s)| *s);
        let mut p = g + (1.0 - g - self.slip) * skill * readiness;
        p *= 1.0 - 0.20 * mis_penalty;
        let rapid = self.rng.next_f64() < self.rapid;
        let from_mis = !rapid
            && targeted
                .as_ref()
                .is_some_and(|(_, strength)| self.rng.next_f64() < 0.75 * strength);
        let correct = !rapid && !from_mis && self.rng.next_f64() < p;
        let (response, verdict, misconception) = if rapid {
            if spec.typed {
                (Response::Text("?".into()), Verdict::Unrecognized, None)
            } else if spec.options.is_empty() {
                (Response::Option(0), Verdict::Correct, None)
            } else {
                let i = pick(&mut self.rng, spec.options.len());
                (
                    Response::Option(u16::try_from(i + 1).unwrap_or(1)),
                    Verdict::Wrong,
                    spec.options[i].1.clone(),
                )
            }
        } else if correct {
            if spec.typed {
                (Response::Text(spec.answer.clone()), Verdict::Correct, None)
            } else {
                (Response::Option(0), Verdict::Correct, None)
            }
        } else if from_mis || !spec.wrong.is_empty() {
            if spec.typed {
                let (_, mis) = spec
                    .wrong
                    .get(pick(&mut self.rng, spec.wrong.len()))
                    .cloned()
                    .unwrap_or_else(|| ("x".into(), spec.misconceptions[0].clone()));
                (Response::Text("xx".into()), Verdict::Wrong, Some(mis))
            } else if !spec.options.is_empty() {
                let i = pick(&mut self.rng, spec.options.len());
                (
                    Response::Option(u16::try_from(i + 1).unwrap_or(1)),
                    Verdict::Wrong,
                    spec.options[i].1.clone(),
                )
            } else {
                (Response::Text("xx".into()), Verdict::Wrong, None)
            }
        } else {
            (Response::Text("xx".into()), Verdict::Wrong, None)
        };
        if !placement {
            for kc in &spec.primary {
                self.practise(kc, at, if correct { 1.0 } else { 0.5 });
                if correct {
                    if let Some((mis, _)) = targeted.as_ref() {
                        if spec.misconceptions.contains(mis) {
                            if let Some(strength) = self.active.get_mut(mis) {
                                *strength = (*strength - 0.18).max(0.0);
                            }
                        } else if let Some((mis, _)) = self.strongest_active(kc)
                            && let Some(strength) = self.active.get_mut(&mis)
                        {
                            *strength = (*strength - 0.03).max(0.0);
                        }
                    }
                } else if let Some(mis) = &misconception
                    && let Some(strength) = self.active.get_mut(mis)
                {
                    *strength = (*strength + self.relapse).min(0.98);
                }
            }
            // S3 diagnosis includes the content's targeted remediation: after
            // a wrong answer on a diagnosed L1 pattern, the micro-explanation
            // is shown and the idea loses strength. S1/S2 only see correctness
            // and do not get this remediation.
            if remedy
                && !correct
                && let Some((mis, _)) = targeted.as_ref()
                && let Some(strength) = self.active.get_mut(mis)
            {
                *strength = (*strength - 0.20).max(0.0);
            }
        }
        let latency = if rapid {
            // Below the model's rapid-share threshold: the core discards it.
            (spec.expected_time * 1000 / 10).max(200)
        } else {
            // Above both rapid thresholds (25% of expected time and the
            // 2 s typed floor), so a genuine answer updates the model.
            (spec.expected_time * 1000 * 3 / 5).max(2_600)
        };
        let typing_ms = if spec.typed && !rapid {
            Some(latency.saturating_sub(400))
        } else {
            Some(0)
        };
        let chars_typed = if spec.typed {
            u32::try_from(spec.answer.chars().count()).unwrap_or(0)
        } else {
            0
        };
        let event = AnswerEvent {
            item: spec.item.clone(),
            slot: spec.slot,
            scaffold: 0,
            revealed_deps: false,
            context: if placement {
                AnswerContext::Placement
            } else {
                AnswerContext::Practice
            },
            response,
            verdict,
            misconception: misconception.clone(),
            latency_ms: latency,
            first_input_ms: Some(latency),
            typing_ms,
            chars_typed: Some(chars_typed),
            edits: Some(0),
            input_device: Some(InputDevice::Touch),
            hidden_ms: Some(0),
            confidence: None,
            hints: 0,
            attempt: 1,
        };
        (event, correct, misconception)
    }
    fn next_time(&mut self, step: usize) -> Timestamp {
        let mut ms = 60_000 + i64::try_from(self.time_rng.next_u64() % 30_000).unwrap_or(0);
        if step > 0 && step.is_multiple_of(20) {
            ms += DAY_MS;
        }
        if self.archetype == Archetype::Guesser && self.time_rng.next_f64() < 0.10 {
            ms += i64::try_from(1 + self.time_rng.next_u64() % 7).unwrap_or(1) * DAY_MS;
        }
        self.clock += ms;
        Timestamp(self.clock)
    }

    pub fn true_level(&self, bundle: &Bundle, specs: &[SlotSpec], at: Timestamp) -> f64 {
        let entries: Vec<_> = learning_kcs(bundle)
            .into_iter()
            .filter(|e| e.kc.domain.as_str() == "grammar")
            .collect();
        let Some(top) = entries.iter().map(|e| usize::from(e.level_rank)).max() else {
            return 0.0;
        };
        let scale_len = bundle
            .content
            .subjects
            .first()
            .map_or(top + 1, |s| s.scale.levels.len().max(top + 1));
        let mut weight = vec![0.0; scale_len];
        let mut mastered = vec![0.0; scale_len];
        for e in &entries {
            let rank = usize::from(e.level_rank);
            let importance = e.kc.importance.get();
            weight[rank] += importance;
            if self.mastery(bundle, specs, &e.kc.id, at) >= MASTERY {
                mastered[rank] += importance;
            }
        }
        let base = weight[..=top].iter().filter(|w| **w <= 0.0).count() as f64 - 1.0;
        base + (0..=top)
            .map(|i| {
                if weight[i] > 0.0 {
                    mastered[i] / weight[i]
                } else {
                    0.0
                }
            })
            .sum::<f64>()
    }
}

fn pick(rng: &mut SplitMix64, n: usize) -> usize {
    if n == 0 {
        0
    } else {
        usize::try_from(rng.next_u64() % u64::try_from(n).unwrap_or(1)).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strategy {
    Leitner,
    KcAdaptive,
    Full,
}

impl Strategy {
    pub const ALL: [Self; 3] = [Self::Leitner, Self::KcAdaptive, Self::Full];

    pub fn name(self) -> &'static str {
        match self {
            Self::Leitner => "S1 Leitner/linear",
            Self::KcAdaptive => "S2 KC-adaptive",
            Self::Full => "S3 full refl",
        }
    }
}

struct Selector {
    strategy: Strategy,
    l1: L1,
    boxes: BTreeMap<(ItemRef, u16), u8>,
    due: BTreeMap<(ItemRef, u16), u32>,
    order: Vec<usize>,
    cursor: BTreeMap<KcId, usize>,
    asks: BTreeMap<KcId, u32>,
    seen: BTreeMap<usize, u32>,
    prerequisites: BTreeMap<KcId, Vec<KcId>>,
    dependents: BTreeMap<KcId, Vec<KcId>>,
    importance: BTreeMap<KcId, f64>,
    slots_by_kc: BTreeMap<KcId, Vec<usize>>,
}

impl Selector {
    fn new(strategy: Strategy, bundle: &Bundle, specs: &[SlotSpec], l1: L1, seed: u64) -> Self {
        let mut order: Vec<usize> = (0..specs.len()).collect();
        order.sort_by(|&a, &b| {
            specs[a]
                .primary
                .first()
                .cmp(&specs[b].primary.first())
                .then_with(|| specs[a].item.cmp(&specs[b].item))
                .then_with(|| specs[a].slot.cmp(&specs[b].slot))
        });
        let mut boxes = BTreeMap::new();
        let mut due = BTreeMap::new();
        for spec in specs {
            boxes.insert((spec.item.clone(), spec.slot), 0);
            due.insert((spec.item.clone(), spec.slot), 0);
        }
        let mut prerequisites = BTreeMap::new();
        let mut dependents: BTreeMap<KcId, Vec<KcId>> = BTreeMap::new();
        for (kc, reqs) in &bundle.content.requires_closure {
            let direct: Vec<KcId> = reqs
                .iter()
                .filter(|r| r.distance == 1)
                .map(|r| r.kc.clone())
                .collect();
            for r in &direct {
                dependents.entry(r.clone()).or_default().push(kc.clone());
            }
            prerequisites.insert(kc.clone(), direct);
        }
        let importance = learning_kcs(bundle)
            .into_iter()
            .map(|e| (e.kc.id.clone(), e.kc.importance.get()))
            .collect();
        let mut slots_by_kc: BTreeMap<KcId, Vec<usize>> = BTreeMap::new();
        for (i, spec) in specs.iter().enumerate() {
            for kc in &spec.primary {
                slots_by_kc.entry(kc.clone()).or_default().push(i);
            }
        }
        let _ = seed;
        Self {
            strategy,
            l1,
            boxes,
            due,
            order,
            cursor: BTreeMap::new(),
            asks: BTreeMap::new(),
            seen: BTreeMap::new(),
            prerequisites,
            dependents,
            importance,
            slots_by_kc,
        }
    }

    fn choose(
        &mut self,
        bundle: &Bundle,
        state: &State,
        model: &KnowledgeModel,
        specs: &[SlotSpec],
        step: usize,
        now: Timestamp,
    ) -> Option<usize> {
        if specs.is_empty() {
            return None;
        }
        match self.strategy {
            Strategy::Leitner => {
                let mut best: Option<usize> = None;
                let mut best_key: Option<(u32, usize)> = None;
                for &i in &self.order {
                    let spec = &specs[i];
                    let key = (
                        self.due
                            .get(&(spec.item.clone(), spec.slot))
                            .copied()
                            .unwrap_or(0),
                        i,
                    );
                    if best_key.is_none_or(|current| key < current) {
                        best_key = Some(key);
                        best = Some(i);
                    }
                }
                let _ = (bundle, state, model, step, now);
                best
            }
            Strategy::KcAdaptive => {
                let mut best: Option<(&KcId, f64)> = None;
                for (kc, indices) in &self.slots_by_kc {
                    if indices.is_empty() {
                        continue;
                    }
                    let Some(belief) = model.belief(state, bundle, kc, now) else {
                        continue;
                    };
                    let asks = f64::from(self.asks.get(kc).copied().unwrap_or(0));
                    let importance = self.importance.get(kc).copied().unwrap_or(0.5);
                    let score = importance * belief.var / (1.0 + asks);
                    if best.as_ref().is_none_or(|(_, s)| score > *s) {
                        best = Some((kc, score));
                    }
                }
                let (kc, _) = best?;
                let indices = self.slots_by_kc.get(kc)?;
                let cursor = self.cursor.entry(kc.clone()).or_insert(0);
                let index = indices[*cursor % indices.len()];
                *cursor += 1;
                Some(index)
            }
            Strategy::Full => {
                let mut best: Option<(usize, f64)> = None;
                for (i, spec) in specs.iter().enumerate() {
                    let score = self.full_parts(bundle, state, model, spec, i, now).0;
                    if best.as_ref().is_none_or(|(_, s)| score > *s) {
                        best = Some((i, score));
                    }
                }
                best.map(|(i, _)| i)
            }
        }
    }

    fn mis_prior(&self, bundle: &Bundle, id: &MisconceptionId) -> f64 {
        bundle
            .content
            .misconceptions
            .iter()
            .find(|m| &m.id == id)
            .map_or(0.0, |m| match self.l1 {
                L1::Uz => m.prior.uz.get(),
                L1::Ru => m.prior.ru.get(),
            })
    }

    /// `(score, variance reduction per second, misconception bonus, due count)`
    fn full_parts(
        &self,
        bundle: &Bundle,
        state: &State,
        model: &KnowledgeModel,
        spec: &SlotSpec,
        index: usize,
        now: Timestamp,
    ) -> (f64, f64, f64, f64) {
        let mut score = 0.0;
        for kc in &spec.primary {
            let Some(information) =
                model.information(state, bundle, &spec.item, spec.slot, kc, now)
            else {
                continue;
            };
            let Some(belief) = model.belief(state, bundle, kc, now) else {
                continue;
            };
            let v = belief.var + model.params.process_noise;
            let reduced = v - 1.0 / (1.0 / v + information);
            score += self.importance.get(kc).copied().unwrap_or(0.5) * reduced;
        }
        score /= f64::from(spec.expected_time.max(1));
        let info_score = score;

        // Misconception diagnosis: target the strongest believed active idea,
        // using the content prior until the model has its own evidence. This
        // is a multiplier: the variance-reduction score is in squared-belief
        // units and can be much smaller than a probability.
        let mis_bonus = spec
            .misconceptions
            .iter()
            .map(|id| {
                model
                    .misconception(state, id, now)
                    .map_or(self.mis_prior(bundle, id), |view| view.strength)
            })
            .fold(0.0, f64::max);
        score *= 1.0 + 1.5 * mis_bonus;

        // FSRS due reviews keep their memory alive, but do not crowd out
        // first learning.
        let due_bonus = spec
            .primary
            .iter()
            .filter_map(|kc| state.kcs.get(kc)?.card.as_ref())
            .map(|card| model.recall(Some(card), now))
            .filter(|r| *r < 0.75)
            .count() as f64;
        score *= 1.0 + 0.5 * due_bonus;

        // Graph routing: do not spend a step on a blocked KC; prefer the
        // prerequisite that unblocks others.
        for kc in &spec.primary {
            let blocked = self
                .prerequisites
                .get(kc)
                .into_iter()
                .flatten()
                .filter_map(|req| model.belief(state, bundle, req, now))
                .any(|b| model.mastery_probability(b) < 0.5);
            if blocked {
                score *= 0.5;
            }
            let unlocks = self
                .dependents
                .get(kc)
                .into_iter()
                .flatten()
                .filter_map(|dep| model.belief(state, bundle, dep, now))
                .any(|b| model.mastery_probability(b) < 0.5);
            if unlocks {
                score *= 1.3;
            }
        }
        let seen = f64::from(self.seen.get(&index).copied().unwrap_or(0));
        score /= 1.0 + seen / 4.0;
        (score, info_score, mis_bonus, due_bonus)
    }

    fn record(&mut self, spec: &SlotSpec, index: usize, correct: bool, step: usize) {
        let key = (spec.item.clone(), spec.slot);
        let box_number = self.boxes.get(&key).copied().unwrap_or(0);
        let next = if correct { (box_number + 1).min(4) } else { 0 };
        self.boxes.insert(key.clone(), next);
        let interval = [1u32, 2, 4, 8, 16][usize::from(next)];
        self.due.insert(
            key,
            u32::try_from(step)
                .unwrap_or(u32::MAX)
                .saturating_add(interval),
        );
        for kc in &spec.primary {
            *self.asks.entry(kc.clone()).or_insert(0) += 1;
        }
        *self.seen.entry(index).or_insert(0) += 1;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunSpec {
    pub learners: usize,
    pub steps: usize,
    pub seed: u64,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub archetype: Archetype,
    pub l1: L1,
    pub mastery_step: BTreeMap<KcId, u32>,
    pub to_learn: Vec<KcId>,
    pub active_initial: usize,
    pub active_at: [usize; 4],
    pub true_level: f64,
    pub model_level: f64,
}

#[derive(Debug, Clone)]
pub struct PlacementOutcome {
    pub archetype: Archetype,
    pub l1: L1,
    pub errors: [Option<f64>; 3],
}

#[derive(Debug, Clone)]
pub struct StrategyRun {
    pub strategy: Strategy,
    pub outcomes: Vec<Outcome>,
    pub placement: Vec<PlacementOutcome>,
}

#[derive(Debug, Clone)]
pub struct BundleRun {
    pub label: String,
    pub learners: usize,
    pub steps: usize,
    pub target_kcs: Vec<KcId>,
    pub strategies: Vec<StrategyRun>,
    /// Per KC: (slots, families) in the item bank, for the failure audit.
    pub bank: BTreeMap<KcId, (usize, usize)>,
}

pub fn run(bundle: &Bundle, label: &str, spec: RunSpec) -> BundleRun {
    let specs = slots(bundle);
    let targets: Vec<KcId> = learning_kcs(bundle)
        .into_iter()
        .filter(|e| e.kc.domain.as_str() == "grammar")
        .map(|e| e.kc.id.clone())
        .collect();
    let mut bank = BTreeMap::new();
    for kc in &targets {
        let mut families = BTreeSet::new();
        let mut count = 0;
        for slot in &specs {
            if slot.primary.contains(kc) {
                count += 1;
                families.insert(slot.family.clone());
            }
        }
        bank.insert(kc.clone(), (count, families.len()));
    }
    let model = KnowledgeModel::default();
    let families = Families::of(bundle);
    let ctx = FoldCtx::new(bundle, &families);
    let subject = SubjectId::new("en").expect("valid subject");
    let domain = DomainId::new("grammar").expect("valid domain");
    let slot_index: BTreeMap<(ItemRef, u16), usize> = specs
        .iter()
        .enumerate()
        .map(|(i, s)| ((s.item.clone(), s.slot), i))
        .collect();

    let mut strategies = Vec::new();
    for strategy in Strategy::ALL {
        let mut outcomes = Vec::new();
        let mut placement_outcomes = Vec::new();
        for learner_index in 0..spec.learners {
            let base = SimLearner::new(bundle, learner_index, spec.seed, &specs);
            let profile = Profile {
                start_level: BTreeMap::new(),
                l1: Some(base.l1.native()),
            };
            let to_learn: Vec<KcId> = targets
                .iter()
                .filter(|kc| base.mastery(bundle, &specs, kc, T0) < MASTERY)
                .cloned()
                .collect();
            let mut learner = base.clone();
            let mut selector = Selector::new(strategy, bundle, &specs, base.l1, spec.seed);
            let mut state = model.init(&profile);
            let mut mastery_step = BTreeMap::new();
            let active_initial = learner.active_count();
            let mut active_at = [active_initial; 4];

            let mut at = T0;
            for step in 0..spec.steps {
                at = learner.next_time(step);
                let Some(index) = selector.choose(bundle, &state, &model, &specs, step, at) else {
                    break;
                };
                let slot = &specs[index];
                let (answer, correct, _) =
                    learner.answer(bundle, slot, at, false, strategy == Strategy::Full);
                let event = event_in(
                    Some(session(1)),
                    at,
                    u64::try_from(step).unwrap_or(0),
                    EventPayload::Answer(answer),
                );
                model
                    .apply(&mut state, &event, &ctx)
                    .expect("events are folded in order");
                selector.record(slot, index, correct, step);
                for kc in &to_learn {
                    if !mastery_step.contains_key(kc)
                        && learner.mastery(bundle, &specs, kc, at) >= MASTERY
                    {
                        mastery_step
                            .insert(kc.clone(), u32::try_from(step + 1).unwrap_or(u32::MAX));
                    }
                }
                if let Some(pos) = CHECKPOINTS.iter().position(|c| *c == step + 1) {
                    active_at[pos] = learner.active_count();
                }
            }
            let true_level = learner.true_level(bundle, &specs, at);
            let model_level = model
                .level(&state, bundle, &subject, &domain, at)
                .map_or(0.0, |l| l.point);
            outcomes.push(Outcome {
                archetype: base.archetype,
                l1: base.l1,
                mastery_step,
                to_learn,
                active_initial,
                active_at,
                true_level,
                model_level,
            });

            // Placement accuracy on a fresh, non-learning copy.
            let true_placement = base.true_level(bundle, &specs, T0);
            let mut placement_learner = base.clone();
            let mut placement_state = model.init(&profile);
            let mut placement = Placement {
                subject: subject.clone(),
                asked: Vec::new(),
                correct: 0,
            };
            let mut errors = [None; 3];
            for step in 0..10 {
                let next = placement::next(&model, bundle, &placement_state, &placement, T0);
                let placement::PlacementNext::Item { item, slot } = next else {
                    break;
                };
                let Some(&index) = slot_index.get(&(item.clone(), slot)) else {
                    break;
                };
                let (answer, correct, _) =
                    placement_learner.answer(bundle, &specs[index], T0, true, false);
                let event = event_in(
                    Some(session(90)),
                    T0,
                    u64::try_from(step).unwrap_or(0),
                    EventPayload::Answer(answer),
                );
                model
                    .apply(&mut placement_state, &event, &ctx)
                    .expect("placement events are folded in order");
                let _ = placement::record(bundle, &mut placement, item, slot, correct);
                if let Some(pos) = [3usize, 6, 10].iter().position(|c| *c == step + 1) {
                    let estimate = model
                        .level(&placement_state, bundle, &subject, &domain, T0)
                        .map_or(0.0, |l| l.point);
                    errors[pos] = Some((estimate - true_placement).abs());
                }
            }
            placement_outcomes.push(PlacementOutcome {
                archetype: base.archetype,
                l1: base.l1,
                errors,
            });
        }
        strategies.push(StrategyRun {
            strategy,
            outcomes,
            placement: placement_outcomes,
        });
    }
    BundleRun {
        label: label.to_owned(),
        learners: spec.learners,
        steps: spec.steps,
        target_kcs: targets,
        strategies,
        bank,
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        values[mid - 1] * 0.5 + values[mid] * 0.5
    } else {
        values[mid]
    })
}

fn quantile(mut values: Vec<f64>, q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = (q * values.len() as f64).ceil() as usize;
    Some(values[rank.saturating_sub(1).min(values.len() - 1)])
}

fn steps_of(outcomes: &[Outcome]) -> Vec<f64> {
    let mut steps = Vec::new();
    for outcome in outcomes {
        for kc in &outcome.to_learn {
            if let Some(step) = outcome.mastery_step.get(kc) {
                steps.push(f64::from(*step));
            }
        }
    }
    steps
}

fn reach(outcomes: &[Outcome]) -> (usize, usize) {
    let reached = outcomes
        .iter()
        .map(|o| {
            o.to_learn
                .iter()
                .filter(|kc| o.mastery_step.contains_key(*kc))
                .count()
        })
        .sum();
    (reached, outcomes.iter().map(|o| o.to_learn.len()).sum())
}

fn mis_reduction(outcomes: &[Outcome], checkpoint: usize) -> (usize, usize) {
    let initial: usize = outcomes.iter().map(|o| o.active_initial).sum();
    let left: usize = outcomes.iter().map(|o| o.active_at[checkpoint]).sum();
    (initial, left)
}

fn level_errors(outcomes: &[Outcome]) -> (f64, f64) {
    if outcomes.is_empty() {
        return (0.0, 0.0);
    }
    let n = outcomes.len() as f64;
    let mae = outcomes
        .iter()
        .map(|o| (o.model_level - o.true_level).abs())
        .sum::<f64>()
        / n;
    let rmse = libm::sqrt(
        outcomes
            .iter()
            .map(|o| {
                let e = o.model_level - o.true_level;
                e * e
            })
            .sum::<f64>()
            / n,
    );
    (mae, rmse)
}

fn placement_errors(placement: &[PlacementOutcome], checkpoint: usize) -> (f64, f64) {
    let errors: Vec<f64> = placement
        .iter()
        .filter_map(|p| p.errors[checkpoint])
        .collect();
    if errors.is_empty() {
        return (0.0, 0.0);
    }
    let n = errors.len() as f64;
    let mae = errors.iter().sum::<f64>() / n;
    let rmse = libm::sqrt(errors.iter().map(|e| e * e).sum::<f64>() / n);
    (mae, rmse)
}

fn summary_line(outcomes: &[Outcome]) -> String {
    let (reached, total) = reach(outcomes);
    let (mae, rmse) = level_errors(outcomes);
    if total == 0 {
        return format!(
            "no learning targets (all KCs already >= {MASTERY:.2} at T0) | level MAE {mae:.3} RMSE {rmse:.3}"
        );
    }
    let steps = steps_of(outcomes);
    let median = median(steps.clone()).unwrap_or(f64::NAN);
    let q25 = quantile(steps.clone(), 0.25).unwrap_or(f64::NAN);
    let q75 = quantile(steps, 0.75).unwrap_or(f64::NAN);
    format!(
        "{reached}/{total} ({:.1}%) | median {:.0} IQR {:.0}-{:.0} | level MAE {mae:.3} RMSE {rmse:.3}",
        100.0 * reached as f64 / total as f64,
        median,
        q25,
        q75
    )
}

impl BundleRun {
    pub fn report(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "\n== {}: {} learners x {} steps, target KCs {} ==",
            self.label,
            self.learners,
            self.steps,
            self.target_kcs.len()
        );
        for run in &self.strategies {
            let (reached, total) = reach(&run.outcomes);
            let steps = steps_of(&run.outcomes);
            let (median, q25, q75) = (
                median(steps.clone()).unwrap_or(f64::NAN),
                quantile(steps.clone(), 0.25).unwrap_or(f64::NAN),
                quantile(steps, 0.75).unwrap_or(f64::NAN),
            );
            let (mae, rmse) = level_errors(&run.outcomes);
            let _ = writeln!(out, "\n-- {} --", run.strategy.name());
            let _ = writeln!(
                out,
                "mastery reached {reached}/{total} ({:.1}%); steps median {:.0} IQR {:.0}-{:.0}; level MAE {:.3} RMSE {:.3}",
                if total == 0 {
                    0.0
                } else {
                    100.0 * reached as f64 / total as f64
                },
                median,
                q25,
                q75,
                mae,
                rmse
            );
            for (i, checkpoint) in CHECKPOINTS.iter().enumerate() {
                let (initial, left) = mis_reduction(&run.outcomes, i);
                let reduction = if initial == 0 {
                    0.0
                } else {
                    100.0 * (initial.saturating_sub(left)) as f64 / initial as f64
                };
                let _ = write!(
                    out,
                    "mis @{checkpoint}: {reduction:.1}% ({initial}->{left})  "
                );
            }
            let _ = writeln!(out);
            for (i, checkpoint) in [3usize, 6, 10].iter().enumerate() {
                let (mae, rmse) = placement_errors(&run.placement, i);
                let _ = write!(
                    out,
                    "placement @{checkpoint}: MAE {mae:.3} RMSE {rmse:.3}  "
                );
            }
            let _ = writeln!(out);
            let _ = writeln!(out, "by archetype:");
            for archetype in Archetype::ALL {
                let subset: Vec<Outcome> = run
                    .outcomes
                    .iter()
                    .filter(|o| o.archetype == archetype)
                    .cloned()
                    .collect();
                let _ = writeln!(out, "  {:<18} {}", archetype.name(), summary_line(&subset));
            }
        }
        let _ = writeln!(out, "\nper KC (median [IQR] steps / reach%):");
        let _ = writeln!(
            out,
            "{:<42} {:>8} {:>22} {:>22} {:>22}",
            "KC", "bank", "S1", "S2", "S3"
        );
        let mut failures = Vec::new();
        let mut low_reach: Vec<String> = Vec::new();
        for kc in &self.target_kcs {
            let (bank_slots, bank_families) = self.bank.get(kc).copied().unwrap_or((0, 0));
            let mut cells = Vec::new();
            let mut rates = Vec::new();
            let mut medians = Vec::new();
            for run in &self.strategies {
                let steps: Vec<f64> = run
                    .outcomes
                    .iter()
                    .filter_map(|o| o.mastery_step.get(kc).map(|s| f64::from(*s)))
                    .collect();
                let eligible = run
                    .outcomes
                    .iter()
                    .filter(|o| o.to_learn.contains(kc))
                    .count();
                let rate = if eligible == 0 {
                    0.0
                } else {
                    100.0 * steps.len() as f64 / eligible as f64
                };
                let median = median(steps.clone()).unwrap_or(f64::NAN);
                let q25 = quantile(steps.clone(), 0.25).unwrap_or(f64::NAN);
                let q75 = quantile(steps, 0.75).unwrap_or(f64::NAN);
                rates.push(rate);
                medians.push(median);
                cells.push(format!("{median:.0} [{q25:.0}-{q75:.0}] ({rate:.0}%)"));
            }
            let _ = writeln!(
                out,
                "{:<42} {:>3}/{:<3} {:>22} {:>22} {:>22}",
                kc.as_str(),
                bank_slots,
                bank_families,
                cells[0],
                cells[1],
                cells[2]
            );
            let best_other = rates[0].max(rates[1]);
            if rates[2] < 10.0 {
                low_reach.push(format!(
                    "{}: S3 reach {:.0}% (baselines {:.0}/{:.0}%), bank {} slots / {} families",
                    kc.as_str(),
                    rates[2],
                    rates[0],
                    rates[1],
                    bank_slots,
                    bank_families
                ));
            }
            if rates[2] + 0.01 < best_other {
                failures.push(format!(
                    "{}: S3 rate {:.0}% vs best of S1/S2 {:.0}%, median {:.0} vs {:.0}/{:.0}; bank {} slots / {} families",
                    kc.as_str(),
                    rates[2],
                    best_other,
                    medians[2],
                    medians[0],
                    medians[1],
                    bank_slots,
                    bank_families
                ));
            }
        }
        let _ = writeln!(out, "\nfailure audit (S3 rate below both baselines):");
        if failures.is_empty() {
            let _ = writeln!(out, "none on this seed");
        } else {
            for failure in &failures {
                let _ = writeln!(out, "- {failure}");
            }
        }
        let _ = writeln!(out, "\nS3 low absolute reach (<10%):");
        if low_reach.is_empty() {
            let _ = writeln!(out, "none on this seed");
        } else {
            for line in &low_reach {
                let _ = writeln!(out, "- {line}");
            }
        }

        let mut archetype_failures = Vec::new();
        let mut low_arch: Vec<String> = Vec::new();
        for archetype in Archetype::ALL {
            let mut rates = Vec::new();
            let mut medians = Vec::new();
            for run in &self.strategies {
                let subset: Vec<Outcome> = run
                    .outcomes
                    .iter()
                    .filter(|o| o.archetype == archetype)
                    .cloned()
                    .collect();
                let (reached, total) = reach(&subset);
                rates.push(if total == 0 {
                    0.0
                } else {
                    100.0 * reached as f64 / total as f64
                });
                medians.push(median(steps_of(&subset)).unwrap_or(f64::NAN));
            }
            let best_other = rates[0].max(rates[1]);
            if rates[2] < 10.0 {
                low_arch.push(format!(
                    "{}: S3 reach {:.0}% (baselines {:.0}/{:.0}%), median {:.0}",
                    archetype.name(),
                    rates[2],
                    rates[0],
                    rates[1],
                    medians[2]
                ));
            }
            if rates[2] + 0.01 < best_other {
                archetype_failures.push(format!(
                    "{}: S3 rate {:.0}% vs best of S1/S2 {:.0}%, median {:.0} vs {:.0}/{:.0}",
                    archetype.name(),
                    rates[2],
                    best_other,
                    medians[2],
                    medians[0],
                    medians[1]
                ));
            }
        }
        let _ = writeln!(out, "\narchetype audit (S3 rate below both baselines):");
        if archetype_failures.is_empty() {
            let _ = writeln!(out, "none on this seed");
        } else {
            for failure in &archetype_failures {
                let _ = writeln!(out, "- {failure}");
            }
        }
        let _ = writeln!(out, "\nS3 low absolute reach by archetype (<10%):");
        if low_arch.is_empty() {
            let _ = writeln!(out, "none on this seed");
        } else {
            for line in &low_arch {
                let _ = writeln!(out, "- {line}");
            }
        }
        let _ = writeln!(out, "\nL1 split:");
        for l1 in [L1::Uz, L1::Ru] {
            let subset: Vec<Outcome> = self.strategies.last().map_or_else(Vec::new, |run| {
                run.outcomes
                    .iter()
                    .filter(|o| o.l1 == l1)
                    .cloned()
                    .collect()
            });
            let _ = writeln!(out, "  {}: {}", l1.name(), summary_line(&subset));
        }
        out
    }
}
