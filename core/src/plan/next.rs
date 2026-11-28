//! The next item of a step (`docs/04-placement-planner.md`, «Сборка сессии в
//! прототипе», «Следующее задание»).

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use super::issue::{factor, finite};
use super::values::SlotRef;
use super::{Choice, End, History, Next, Phase, Plan, Planner, Step, draw};
use crate::event::{PlanAction, PlanCandidate, PlanIssuedEvent, PlanScope};
use crate::ids::{ItemId, MisconceptionId};
use crate::item::LadderLevel;
use crate::model::derived::MisconceptionStatus;
use crate::model::{LearnerModel, State};
use crate::value::{Seed, Timestamp, UnitInterval};

const DAY_MS: f64 = 86_400_000.0;

/// Keeps the elements that satisfy `f`, unless none does.
fn prefer<T>(v: &mut Vec<T>, f: impl Fn(&T) -> bool) {
    if v.iter().any(&f) {
        v.retain(f);
    }
}

/// A candidate slot with its score `s = |p − p*| + ν·seen − κ_I·Ī`.
struct Scored<'b> {
    slot: SlotRef<'b>,
    score: f64,
    p: f64,
    seen: bool,
    information: f64,
}

impl<'a> Planner<'a> {
    /// p* of a phase; `None` for a review, where any p will do.
    fn target(&self, phase: Phase) -> Option<f64> {
        let p = self.params;
        match phase {
            Phase::Block => Some(p.p_block),
            Phase::Interleaved | Phase::Final => Some(p.p_interleaved),
            Phase::Prerequisite => Some(p.p_prerequisite),
            Phase::Review | Phase::Lesson | Phase::Audit | Phase::ReturnCheck => None,
        }
    }

    fn active_misconceptions<'s>(
        &self,
        s: &'s State,
        now: Timestamp,
    ) -> BTreeSet<&'s MisconceptionId> {
        s.misconceptions
            .keys()
            .filter(|id| {
                self.model
                    .misconception(s, id, now)
                    .is_some_and(|v| v.status == MisconceptionStatus::Active)
            })
            .collect()
    }

    /// The candidates of `step` after the kind and misconception filters,
    /// scored and sorted by score, then (item, slot).
    fn scored(
        &self,
        s: &State,
        plan: &Plan,
        step: &Step,
        history: &History,
        now: Timestamp,
    ) -> Vec<Scored<'a>> {
        let p = self.params;
        let shown: BTreeSet<&ItemId> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        let mut slots: Vec<SlotRef<'a>> = self
            .slots(&step.kc)
            .into_iter()
            .filter(|c| !shown.contains(&c.item.id))
            .filter(|c| {
                step.family
                    .as_ref()
                    .is_none_or(|f| &c.projection.family == f)
            })
            .collect();
        let session_families: BTreeSet<_> = plan.shown().map(|(_, sh)| &sh.family).collect();
        if step.phase.is_probe() {
            let stale = Timestamp(now.0 - 30 * 86_400_000);
            slots.retain(|c| {
                c.projection.is_production()
                    && !session_families.contains(&c.projection.family)
                    && history
                        .last_shown(&c.projection.family)
                        .is_none_or(|t| t <= stale)
            });
        }
        if step.production {
            slots.retain(|c| c.projection.is_production());
        } else if step.phase == Phase::Review {
            prefer(&mut slots, |c| c.projection.typed_input());
        } else {
            let level = if self.mastery(s, &step.kc, now) < p.ladder_mastery {
                LadderLevel::Recognition
            } else {
                LadderLevel::Guided
            };
            prefer(&mut slots, |c| c.projection.ladder() == level);
        }
        let active = self.active_misconceptions(s, now);
        prefer(&mut slots, |c| {
            c.projection.slots[usize::from(c.slot)]
                .misconceptions
                .iter()
                .any(|m| active.contains(m))
        });

        #[allow(clippy::cast_possible_truncation)] // days fit i64 ms
        let window = Timestamp(now.0 - (p.seen_days * DAY_MS) as i64);
        let target = self.target(step.phase);
        let mut scored: Vec<Scored<'a>> = slots
            .into_iter()
            .filter_map(|c| {
                let prob = self.model.predict(s, self.bundle, c.item, c.slot, now)?;
                let family = &c.projection.family;
                let seen = session_families.contains(family)
                    || history.last_shown(family).is_some_and(|t| t >= window);
                let expected = f64::from(c.projection.expected_time);
                let information = if expected > 0.0 {
                    self.model
                        .information(s, self.bundle, c.item, c.slot, &step.kc, now)
                        .unwrap_or(0.0)
                        / expected
                } else {
                    0.0
                };
                let distance = target.map_or(0.0, |t| libm::fabs(prob - t));
                let score = distance + if seen { p.nu } else { 0.0 } - p.kappa_info * information;
                Some(Scored {
                    slot: c,
                    score,
                    p: prob,
                    seen,
                    information,
                })
            })
            .collect();
        scored.sort_by(|a, b| {
            a.score
                .total_cmp(&b.score)
                .then_with(|| a.slot.item.cmp(b.slot.item))
                .then_with(|| a.slot.slot.cmp(&b.slot.slot))
        });
        scored
    }

    /// The first step of the plan: the lesson, or an item picked among the
    /// candidates within 95% of the best with the core PRNG from `seed`.
    /// `recent` holds the families shown before the session.
    ///
    /// # Panics
    /// Never: a propensity `1/n` is a probability and every score is finite.
    pub fn next_item(
        &self,
        s: &State,
        plan: &Plan,
        history: &History,
        now: Timestamp,
        seed: Seed,
    ) -> Next {
        let Some(step) = plan.steps.first() else {
            return Next::Finished {
                end: plan.end.unwrap_or(End::ItemsExhausted),
            };
        };
        if step.phase == Phase::Lesson {
            if let Some(e) = self.lesson_explanation(&step.kc, plan.lang) {
                return Next::Lesson {
                    kc: step.kc.clone(),
                    explanation: e.id.clone(),
                };
            }
            // `remaining` plans a lesson only with an explanation to show.
            return Next::Finished {
                end: End::ItemsExhausted,
            };
        }
        let scored = self.scored(s, plan, step, history, now);
        // A step without a candidate never comes from `remaining`; the
        // validator reports such a plan as `ineligible`.
        let Some(best) = scored.first() else {
            return Next::Finished {
                end: End::ItemsExhausted,
            };
        };
        let bar = best.score + self.params.tie_epsilon;
        let n = scored.iter().take_while(|c| c.score <= bar).count();
        let (index, propensity) = draw(seed, n);
        let chosen = &scored[index];
        let target = self.target(step.phase);
        let candidates: Vec<PlanCandidate> = scored
            .iter()
            .take(n.max(self.params.logged_candidates))
            .map(|c| {
                let mut factors = Vec::with_capacity(4);
                factors.push(factor("p", c.p));
                if let Some(t) = target {
                    factors.push(factor("target", t));
                }
                factors.push(factor("seen", if c.seen { 1.0 } else { 0.0 }));
                factors.push(factor("information", c.information));
                PlanCandidate {
                    action: PlanAction::Item {
                        item: c.slot.item.clone(),
                        slot: c.slot.slot,
                    },
                    value_per_minute: finite(-c.score),
                    factors,
                }
            })
            .collect();
        let item = chosen.slot.item.clone();
        let slot = chosen.slot.slot;
        let context = step.phase.context();
        let issued = PlanIssuedEvent {
            planner_version: self.version(),
            model_version: self.model.version(),
            scope: PlanScope::NextItem,
            candidates,
            chosen: u16::try_from(index).unwrap_or(u16::MAX),
            propensity: UnitInterval::new(propensity).expect("a probability"),
            seed,
            violations: self.choice_violation_codes(plan, step, &item, slot, s, now),
            dominant: String::from(self.item_dominant(s, plan, step, &chosen.slot, now)),
            trigger: None,
            end: None,
        };
        Next::Item(Choice {
            item,
            slot,
            issued,
            context,
        })
    }

    /// The factor «Почему это задание» names for an item choice.
    fn item_dominant(
        &self,
        s: &State,
        plan: &Plan,
        step: &Step,
        chosen: &SlotRef<'_>,
        now: Timestamp,
    ) -> &'static str {
        match step.phase {
            Phase::Review => return "memory",
            Phase::Prerequisite => return "root_cause",
            Phase::Final => return "session_error",
            Phase::Audit => return "audit",
            Phase::ReturnCheck => return "return_check",
            Phase::Lesson | Phase::Block | Phase::Interleaved => {}
        }
        let active = self.active_misconceptions(s, now);
        let diagnoses = &chosen.projection.slots[usize::from(chosen.slot)].misconceptions;
        if diagnoses.iter().any(|m| active.contains(m)) {
            return "misconception";
        }
        if plan.is_learn(&step.kc) {
            self.learn_value(s, &step.kc, now, &self.unlock(s, now))
                .dominant()
        } else if plan.learn.iter().any(|l| l.contrasts.contains(&step.kc)) {
            "contrast"
        } else {
            "interleaving"
        }
    }
}
