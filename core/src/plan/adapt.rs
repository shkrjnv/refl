//! The plan after each answer (`docs/04-placement-planner.md`, «Сборка
//! сессии в прототипе», «Адаптация»): the first rule that fires is recorded
//! with the answer, then the steps ahead are recomputed.

use alloc::vec::Vec;

use super::{Adapted, Done, Outcome, Phase, Plan, Planner, Rule, Shown};
use crate::ids::KcId;
use crate::model::State;
use crate::value::Timestamp;

impl Planner<'_> {
    /// Records `outcome` as the answer to the plan's first step and replans.
    /// `s` already has the answer folded. An outcome that does not fit the
    /// first step (none left, or a lesson against an item) leaves the plan
    /// as it was.
    pub fn adapt(&self, s: &State, plan: &Plan, outcome: &Outcome, now: Timestamp) -> Adapted {
        let unchanged = || Adapted {
            plan: plan.clone(),
            rule: None,
            issued: None,
        };
        let Some(step) = plan.steps.first() else {
            return unchanged();
        };
        let shown = match (outcome, step.phase) {
            (Outcome::Lesson, Phase::Lesson) => None,
            (Outcome::Answer(a), phase) if phase != Phase::Lesson => {
                let Some(p) = self.bundle.projection(&a.item) else {
                    return unchanged();
                };
                Some(Shown {
                    item: a.item.clone(),
                    slot: a.slot,
                    family: p.family.clone(),
                    kind: p.kind,
                    seconds: p.expected_time,
                    correct: a.correct,
                    rapid: a.rapid,
                    sure_error: a.sure && !a.correct,
                })
            }
            _ => return unchanged(),
        };
        let mut next = plan.clone();
        next.steps.remove(0);
        next.done.push(Done {
            step: step.clone(),
            shown,
            rule: None,
        });
        let rule = self.fired(s, &next, now);
        if rule == Some(Rule::RapidGuesses) {
            next.break_suggested = true;
        }
        if let Some(d) = next.done.last_mut() {
            d.rule.clone_from(&rule);
        }
        self.replan(s, &mut next, now);
        let issued = rule
            .as_ref()
            .and_then(|r| self.issue_replan(s, &next, r, now));
        Adapted {
            plan: next,
            rule,
            issued,
        }
    }

    /// The first rule that fires on the last answer of `plan`.
    fn fired(&self, s: &State, plan: &Plan, now: Timestamp) -> Option<Rule> {
        let p = self.params;
        let last = plan.done.last()?;
        let shown = last.shown.as_ref()?;
        let kc = &last.step.kc;
        // Non-rapid answers on the KC in this session, oldest first.
        let answers: Vec<&Shown> = plan
            .shown()
            .filter(|(st, sh)| &st.kc == kc && !sh.rapid)
            .map(|(_, sh)| sh)
            .collect();
        if let Some(l) = &plan.learn
            && &l.kc == kc
            && !plan.block_closed()
            && !plan.deferred()
        {
            let correct = answers.iter().filter(|a| a.correct).count();
            let production = answers.iter().any(|a| a.correct && a.kind.is_production());
            if !shown.rapid && shown.correct && correct >= p.close_correct as usize && production {
                return Some(Rule::BlockClosed);
            }
            let on_kc = plan.shown().filter(|(st, _)| &st.kc == kc).count();
            if on_kc >= p.kc_cap as usize {
                return Some(Rule::KcDeferred);
            }
        }
        if !shown.rapid && !shown.correct {
            let in_row = answers.iter().rev().take_while(|a| !a.correct).count();
            if (last.step.phase == Phase::Prerequisite || in_row >= p.errors_in_row)
                && let Some(j) = self.suspect(s, kc, now)
            {
                return Some(Rule::PrerequisiteCheck { kc: j });
            }
            if shown.sure_error && last.step.phase != Phase::Final {
                return Some(Rule::ConfidentError);
            }
        }
        if shown.rapid
            && s.rapid.streak >= p.rapid_streak
            && s.rapid.streak.is_multiple_of(p.rapid_streak)
        {
            return Some(Rule::RapidGuesses);
        }
        None
    }

    /// The direct prerequisite of `kc` with the largest `D × importance`
    /// among those with `D` above the threshold and an eligible item.
    fn suspect(&self, s: &State, kc: &KcId, now: Timestamp) -> Option<KcId> {
        self.direct_requires(kc)
            .filter(|j| self.is_live(j) && !self.slots(j).is_empty())
            .map(|j| (j, self.suspicion(s, j, now)))
            .filter(|(_, d)| *d > self.params.suspicion_min)
            .map(|(j, d)| (j, d * self.importance(j)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(j, _)| j.clone())
    }
}
