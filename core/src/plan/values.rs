//! What the session rules read from the model and the bundle at `now`:
//! Ready, M, D, V^learn, V^rev (`docs/04-placement-planner.md`, «Ценность
//! темы») and the eligible slots of a KC with their expected times.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::Planner;
use crate::bundle::{KcEntry, KcStatus};
use crate::content::{Explanation, ExplanationDepth, ExplanationTarget, explanation_for};
use crate::ids::{ItemRef, KcId};
use crate::model::derived::{KcProgress, MisconceptionStatus};
use crate::model::{Belief, State, math};
use crate::projection::ItemProjection;
use crate::value::Lang;
use crate::value::Timestamp;

const DAY_MS: f64 = 86_400_000.0;

/// V^learn of a KC with its multipliers.
#[derive(Debug, Clone, PartialEq)]
pub struct LearnValue {
    pub kc: KcId,
    pub importance: f64,
    /// G.
    pub goal: f64,
    /// 1 − M.
    pub gap: f64,
    pub ready: f64,
    /// 1 + m.
    pub misconception: f64,
    /// 1 + ζU.
    pub unlock: f64,
    pub value: f64,
}

impl LearnValue {
    /// The multiplier «Почему это задание» names: the largest of
    /// misconception, unlock and goal above 1, else the gap itself.
    pub fn dominant(&self) -> &'static str {
        [
            ("misconception", self.misconception),
            ("unlock", self.unlock),
            ("goal", self.goal),
        ]
        .into_iter()
        .filter(|(_, v)| *v > 1.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or("gap", |(name, _)| name)
    }
}

/// V^rev of a due KC with its parts and the estimated time of its review.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewValue {
    pub kc: KcId,
    pub importance: f64,
    pub goal: f64,
    /// (r* − R)₊.
    pub below_target: f64,
    /// R(t) − R(t + Δ).
    pub decay: f64,
    pub value: f64,
    pub seconds: u32,
}

/// One eligible slot where a KC is primary.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlotRef<'b> {
    pub item: &'b ItemRef,
    pub slot: u16,
    pub projection: &'b ItemProjection,
}

impl<'a> Planner<'a> {
    pub(super) fn entry(&self, kc: &KcId) -> Option<&'a KcEntry> {
        let kcs = &self.bundle.content.kcs;
        kcs.binary_search_by(|e| e.kc.id.cmp(kc))
            .ok()
            .map(|i| &kcs[i])
    }

    pub(super) fn is_live(&self, kc: &KcId) -> bool {
        self.entry(kc).is_some_and(|e| e.status == KcStatus::Live)
    }

    pub(super) fn importance(&self, kc: &KcId) -> f64 {
        self.entry(kc).map_or(0.0, |e| e.kc.importance.get())
    }

    fn belief(&self, s: &State, kc: &KcId, now: Timestamp) -> Option<Belief> {
        self.model.belief(s, self.bundle, kc, now)
    }

    /// The KCs `kc` requires directly.
    pub(super) fn direct_requires(&self, kc: &KcId) -> impl Iterator<Item = &'a KcId> {
        self.bundle
            .content
            .requires_closure
            .get(kc)
            .into_iter()
            .flatten()
            .filter(|r| r.distance == 1)
            .map(|r| &r.kc)
    }

    /// `P(μ_j > θ*) = Φ((μ_j − θ*)/σ_j)` of one prerequisite.
    fn prerequisite_known(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        self.belief(s, kc, now).map_or(0.0, |b| {
            math::phi((b.mu - self.model.params.mastery_theta) / libm::sqrt(b.var))
        })
    }

    /// `Ready_k`: the product over the direct prerequisites.
    pub fn ready(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        self.direct_requires(kc)
            .map(|j| self.prerequisite_known(s, j, now))
            .product()
    }

    /// `M_k`; 0 for a KC the bundle lacks.
    pub fn mastery(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        self.belief(s, kc, now)
            .map_or(0.0, |b| self.model.mastery_probability(b))
    }

    /// `D_j`; 1 for a KC the bundle lacks.
    pub(super) fn suspicion(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        1.0 - self.prerequisite_known(s, kc, now)
    }

    pub(super) fn progress(&self, s: &State, kc: &KcId, now: Timestamp) -> KcProgress {
        self.model.progress(s, kc, now)
    }

    fn unmastered(&self, s: &State, kc: &KcId, now: Timestamp) -> bool {
        !matches!(
            self.progress(s, kc, now),
            KcProgress::Mastered | KcProgress::LongTerm
        )
    }

    /// `U_k` of every KC: the summed importance of the live, not mastered
    /// KCs that require it directly or transitively.
    pub(super) fn unlock(&self, s: &State, now: Timestamp) -> BTreeMap<&'a KcId, f64> {
        let mut out: BTreeMap<&KcId, f64> = BTreeMap::new();
        for e in &self.bundle.content.kcs {
            if e.status != KcStatus::Live || !self.unmastered(s, &e.kc.id, now) {
                continue;
            }
            for r in self
                .bundle
                .content
                .requires_closure
                .get(&e.kc.id)
                .into_iter()
                .flatten()
            {
                *out.entry(&r.kc).or_insert(0.0) += e.kc.importance.get();
            }
        }
        out
    }

    /// `m_k`: the strength of the strongest active misconception whose
    /// remedy is `kc`, else 0.
    fn remedy_strength(&self, s: &State, kc: &KcId, now: Timestamp) -> f64 {
        self.bundle
            .content
            .misconceptions
            .iter()
            .filter(|m| &m.remedy == kc)
            .filter_map(|m| self.model.misconception(s, &m.id, now))
            .filter(|v| v.status == MisconceptionStatus::Active)
            .map(|v| v.strength)
            .fold(0.0, f64::max)
    }

    /// `V^learn_k = imp · G · (1 − M) · Ready · (1 + m) · (1 + ζU)`.
    pub fn learn_value(
        &self,
        s: &State,
        kc: &KcId,
        now: Timestamp,
        unlocks: &BTreeMap<&KcId, f64>,
    ) -> LearnValue {
        let p = self.params;
        let importance = self.importance(kc);
        let gap = 1.0 - self.mastery(s, kc, now);
        let ready = self.ready(s, kc, now);
        let misconception = 1.0 + self.remedy_strength(s, kc, now);
        let unlock = 1.0 + p.zeta * unlocks.get(kc).copied().unwrap_or(0.0);
        LearnValue {
            kc: kc.clone(),
            importance,
            goal: p.goal_weight,
            gap,
            ready,
            misconception,
            unlock,
            value: importance * p.goal_weight * gap * ready * misconception * unlock,
        }
    }

    /// V^rev of a KC with a card whose R is below r*; `None` otherwise.
    pub fn review_value(&self, s: &State, kc: &KcId, now: Timestamp) -> Option<ReviewValue> {
        let p = self.params;
        let card = s.kcs.get(kc)?.card.as_ref()?;
        let r = self.model.recall(Some(card), now);
        if r >= p.target_recall {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)] // a horizon of days fits i64 ms
        let later = Timestamp(now.0 + (p.review_horizon_days * DAY_MS) as i64);
        let below_target = (p.target_recall - r).max(0.0);
        let decay = r - self.model.recall(Some(card), later);
        let importance = self.importance(kc);
        Some(ReviewValue {
            kc: kc.clone(),
            importance,
            goal: p.goal_weight,
            below_target,
            decay,
            value: importance * p.goal_weight * (below_target + decay),
            seconds: self.max_seconds(kc),
        })
    }

    /// Every eligible slot where `kc` is primary, in (item, slot) order.
    pub(super) fn slots(&self, kc: &KcId) -> Vec<SlotRef<'a>> {
        let mut out = Vec::new();
        let Some(refs) = self.bundle.content.items_by_kc.get(kc) else {
            return out;
        };
        for item in &refs.primary {
            let Some(projection) = self.bundle.projection(item) else {
                continue;
            };
            for (slot, s) in (0u16..).zip(&projection.slots) {
                if s.kcs
                    .iter()
                    .any(|k| &k.kc == kc && k.role == crate::item::KcRole::Primary)
                {
                    out.push(SlotRef {
                        item,
                        slot,
                        projection,
                    });
                }
            }
        }
        out
    }

    /// The mean expected time of the KC's eligible slots, rounded up; 0
    /// without any.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // seconds and counts are small
    pub(super) fn mean_seconds(&self, kc: &KcId) -> u32 {
        let slots = self.slots(kc);
        if slots.is_empty() {
            return 0;
        }
        let sum: u64 = slots
            .iter()
            .map(|s| u64::from(s.projection.expected_time))
            .sum();
        libm::ceil(sum as f64 / slots.len() as f64) as u32
    }

    /// The largest expected time of the KC's eligible slots: a review of it
    /// never takes longer, whichever item it gets.
    pub(super) fn max_seconds(&self, kc: &KcId) -> u32 {
        self.slots(kc)
            .iter()
            .map(|s| s.projection.expected_time)
            .max()
            .unwrap_or(0)
    }

    /// The KC has eligible slots and a production among them.
    pub(super) fn has_production(&self, kc: &KcId) -> bool {
        self.slots(kc).iter().any(|s| s.projection.is_production())
    }

    /// The short explanation of `kc` a learner with interface `lang` sees:
    /// the first reviewed record of the language chain.
    pub(super) fn lesson_explanation(&self, kc: &KcId, lang: Lang) -> Option<&'a Explanation> {
        explanation_for(
            &self.bundle.content.explanations,
            &ExplanationTarget::Kc(kc.clone()),
            ExplanationDepth::Short,
            lang,
        )
    }

    /// Expected seconds of reading `e`: the words of its body, native-language
    /// block and examples at the reading speed, shrunk towards the median of
    /// its `n` views with weight `n / (n + n₀)`; rounded up.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // word counts and seconds are small
    pub(super) fn explanation_seconds(&self, e: &Explanation) -> u32 {
        let p = self.params;
        let words = e.body.split_whitespace().count()
            + e.l1_contrast
                .as_deref()
                .map_or(0, |t| t.split_whitespace().count())
            + e.examples
                .iter()
                .map(|x| x.text.split_whitespace().count())
                .sum::<usize>();
        let prior = words as f64 * 60.0 / p.reading_words_per_minute;
        let seconds = match self.times.get(&e.id) {
            Some((n, median)) => {
                let n = f64::from(n);
                (p.explanation_n0 * prior + n * f64::from(median)) / (p.explanation_n0 + n)
            }
            None => prior,
        };
        libm::ceil(seconds) as u32
    }
}
