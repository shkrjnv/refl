//! Quantities read from the state at a moment `now`: KC statuses,
//! misconception status, prerequisite suspicion, the continuous level per
//! domain and the return-after-a-break trigger. None of them changes the
//! state; time only enters as an argument.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use super::math;
use super::state::State;
use super::v1::{Belief, KnowledgeModel, days, kc_entry, subject};
use crate::bundle::{Bundle, KcStatus};
use crate::ids::{DomainId, KcId, MisconceptionId, SubjectId};
use crate::rng::SplitMix64;
use crate::value::Timestamp;

/// Status of a KC for the knowledge map (`docs/03-layer2-learner-model.md`,
/// «Статусы KC»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KcProgress {
    /// No direct observation: only the starting belief.
    NotStarted,
    Learning,
    Mastered,
    /// Mastered with FSRS stability of at least 21 days.
    LongTerm,
    /// Was mastered, R fell below 0.8.
    Forgetting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MisconceptionStatus {
    /// Needs its remedy: m > 0.35 and at least two real manifestations.
    Active,
    /// m < 0.15 with α + β ≥ 8.
    Cured,
    /// Neither yet.
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MisconceptionView {
    /// m = α / (α + β) after decay.
    pub strength: f64,
    /// α + β after decay.
    pub evidence: f64,
    pub manifested: u32,
    pub status: MisconceptionStatus,
}

/// Continuous level of a domain with its 80% interval
/// (`docs/04-placement-planner.md`, «Остановка и результат»).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Level {
    /// `L_d = Σ_c m_{d,c} − 1` from the probabilities.
    pub point: f64,
    /// 10th and 90th percentiles of the Monte Carlo samples.
    pub low: f64,
    pub high: f64,
}

/// The break threshold (`docs/03-layer2-learner-model.md`, «Перерыв и
/// возвращение»).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReturnCheck {
    /// The first session starts with the return check.
    pub due: bool,
    pub days_since: f64,
    /// Importance-weighted mean R of mastered KCs; `None` without any.
    pub mean_recall: Option<f64>,
    /// Share of mastered KCs that are forgetting; `None` without any.
    pub forgetting_share: Option<f64>,
}

/// Index of the q-quantile in `n` sorted samples (nearest rank).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // n is the sample count, small
fn rank(q: f64, n: usize) -> usize {
    ((libm::ceil(q * n as f64) as usize).max(1) - 1).min(n - 1)
}

impl KnowledgeModel {
    pub fn progress(&self, s: &State, bundle: &Bundle, kc: &KcId, now: Timestamp) -> KcProgress {
        let Some(k) = s.kcs.get(kc) else {
            return KcProgress::NotStarted;
        };
        let r = self.recall(k.card.as_ref(), now);
        if k.ever_mastered && r < self.params.forgetting_r {
            KcProgress::Forgetting
        } else if self.is_mastered(s, bundle, kc, now) {
            let stable = k
                .card
                .is_some_and(|c| c.stability >= self.params.long_term_stability);
            if stable {
                KcProgress::LongTerm
            } else {
                KcProgress::Mastered
            }
        } else if k.observations > 0 {
            KcProgress::Learning
        } else {
            KcProgress::NotStarted
        }
    }

    /// `None` for a misconception the learner has had no chance to show.
    pub fn misconception(
        &self,
        s: &State,
        id: &MisconceptionId,
        now: Timestamp,
    ) -> Option<MisconceptionView> {
        let m = s.misconceptions.get(id)?;
        let p = &self.params;
        let (alpha, beta) = self.decayed(m, now);
        let strength = alpha / (alpha + beta);
        let status = if strength > p.misconception_active_m
            && m.manifested >= p.misconception_active_count
        {
            MisconceptionStatus::Active
        } else if strength < p.misconception_cured_m && alpha + beta >= p.misconception_cured_n {
            MisconceptionStatus::Cured
        } else {
            MisconceptionStatus::Open
        };
        Some(MisconceptionView {
            strength,
            evidence: alpha + beta,
            manifested: m.manifested,
            status,
        })
    }

    /// Suspicion that a prerequisite is the root of an error:
    /// `D = Φ((θ* − μ) / σ)` (`docs/03-layer2-learner-model.md`,
    /// «Распространение по графу»).
    pub fn suspicion(&self, s: &State, bundle: &Bundle, kc: &KcId, now: Timestamp) -> Option<f64> {
        let b = self.belief(s, bundle, kc, now)?;
        Some(math::phi(
            (self.params.mastery_theta - b.mu) / libm::sqrt(b.var),
        ))
    }

    /// Level of one domain over its live KCs; `None` if it has none. An
    /// empty scale level below the highest populated one counts as covered,
    /// above it as zero.
    pub fn level(
        &self,
        s: &State,
        bundle: &Bundle,
        subject_id: &SubjectId,
        domain: &DomainId,
        now: Timestamp,
    ) -> Option<Level> {
        self.level_inner(s, bundle, subject_id, domain, now, true)
    }

    /// The same level over live KCs that have at least one direct
    /// observation. Placement uses this so an untouched KC's prior does not
    /// drag a short check down; the full level above remains the statistics
    /// and planner view.
    pub fn level_assessed(
        &self,
        s: &State,
        bundle: &Bundle,
        subject_id: &SubjectId,
        domain: &DomainId,
        now: Timestamp,
    ) -> Option<Level> {
        self.level_inner(s, bundle, subject_id, domain, now, false)
    }

    #[allow(clippy::cast_precision_loss)] // sample count
    fn level_inner(
        &self,
        s: &State,
        bundle: &Bundle,
        subject_id: &SubjectId,
        domain: &DomainId,
        now: Timestamp,
        include_unobserved: bool,
    ) -> Option<Level> {
        let p = &self.params;
        let scale_len = subject(bundle, subject_id)?.scale.levels.len();
        let mut kcs: Vec<(usize, f64, Belief)> = Vec::new();
        for e in &bundle.content.kcs {
            if e.status == KcStatus::Live && &e.kc.subject == subject_id && &e.kc.domain == domain {
                let observed = s.kcs.get(&e.kc.id).is_some_and(|k| k.observations > 0);
                if !include_unobserved && !observed {
                    continue;
                }
                let belief = self.belief(s, bundle, &e.kc.id, now)?;
                kcs.push((usize::from(e.level_rank), e.kc.importance.get(), belief));
            }
        }
        let top = kcs.iter().map(|k| k.0).max()?;
        let mut weight = alloc::vec![0.0; scale_len.max(top + 1)];
        for &(c, imp, _) in &kcs {
            weight[c] += imp;
        }
        // Covered empty levels, then `− 1`.
        let base = weight[..=top].iter().filter(|w| **w <= 0.0).count() as f64 - 1.0;
        let point = base
            + kcs
                .iter()
                .filter(|k| weight[k.0] > 0.0)
                .map(|&(c, imp, b)| imp * self.mastery_probability(b) / weight[c])
                .sum::<f64>();

        let mut rng = SplitMix64::new(p.level_seed);
        let n = usize::try_from(p.level_samples)
            .unwrap_or(usize::MAX)
            .max(1);
        let mut samples: Vec<f64> = (0..n)
            .map(|_| {
                let mut l = base;
                for &(c, imp, b) in &kcs {
                    let theta = b.mu + libm::sqrt(b.var) * rng.next_normal();
                    let known = theta - p.delta * (1.0 - b.r) > p.mastery_theta;
                    if known && weight[c] > 0.0 {
                        l += imp / weight[c];
                    }
                }
                l
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        Some(Level {
            point,
            low: samples[rank(0.1, n)],
            high: samples[rank(0.9, n)],
        })
    }

    /// Whether the first session in `subject` starts with a return check;
    /// `None` before the first answer in it.
    pub fn return_check(
        &self,
        s: &State,
        bundle: &Bundle,
        subject_id: &SubjectId,
        now: Timestamp,
    ) -> Option<ReturnCheck> {
        let p = &self.params;
        let days_since = days(*s.last_answer.get(subject_id)?, now);
        let (mut weighted_r, mut weight, mut mastered, mut forgetting) = (0.0, 0.0, 0u32, 0u32);
        for (id, k) in &s.kcs {
            let Some(e) = kc_entry(bundle, id) else {
                continue;
            };
            if !k.ever_mastered || &e.kc.subject != subject_id {
                continue;
            }
            let r = self.recall(k.card.as_ref(), now);
            weighted_r += e.kc.importance.get() * r;
            weight += e.kc.importance.get();
            mastered += 1;
            if r < p.forgetting_r {
                forgetting += 1;
            }
        }
        let mean_recall = (weight > 0.0).then(|| weighted_r / weight);
        let forgetting_share = (mastered > 0).then(|| f64::from(forgetting) / f64::from(mastered));
        let slipping = mean_recall.is_some_and(|r| r < p.return_mean_r)
            || forgetting_share.is_some_and(|f| f > p.return_forgetting_share);
        let due =
            days_since >= p.return_always_days || (days_since >= p.return_min_days && slipping);
        Some(ReturnCheck {
            due,
            days_since,
            mean_recall,
            forgetting_share,
        })
    }
}
