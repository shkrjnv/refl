//! The plan validator of the prototype (`docs/04-placement-planner.md`,
//! «Валидатор прототипа»; `docs/11-quality.md`, «К3»). It checks the whole
//! plan, this session's answers and the steps ahead, against the state it
//! was planned on; any violation is a planner bug.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use super::build::answered_production;
use super::{Phase, Plan, Planner, Step};
use crate::ids::{ItemRef, KcId};
use crate::item::KcRole;
use crate::model::State;
use crate::model::derived::KcProgress;
use crate::value::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Violation {
    /// A lesson, block or interleaved step ahead on a KC with Ready below
    /// the threshold.
    BlockedKc,
    /// More than one KC with lesson or block steps, or a not-started KC
    /// other than the learning one in the interleaved part.
    NewKcs,
    /// Learning while reviews do not fit their share.
    ReviewDebt,
    /// A KC still practised without a production item.
    NoProduction,
    /// More than the allowed steps in a row on one KC after the block.
    RunTooLong,
    ReviewShare,
    FinalShare,
    /// A shown item that is not session-eligible, or a step ahead that
    /// cannot get one.
    Ineligible,
}

impl Violation {
    /// The code in `plan_issued.violations`.
    pub fn code(self) -> &'static str {
        match self {
            Self::BlockedKc => "blocked-kc",
            Self::NewKcs => "new-kcs",
            Self::ReviewDebt => "review-debt",
            Self::NoProduction => "no-production",
            Self::RunTooLong => "run-too-long",
            Self::ReviewShare => "review-share",
            Self::FinalShare => "final-share",
            Self::Ineligible => "ineligible",
        }
    }
}

fn codes(v: &[Violation]) -> Vec<String> {
    v.iter().map(|x| String::from(x.code())).collect()
}

impl Plan {
    /// Expected seconds of the items answered in steps of `phase`.
    fn shown_seconds(&self, phase: Phase) -> u32 {
        self.shown()
            .filter(|(st, _)| st.phase == phase)
            .map(|(_, sh)| sh.seconds)
            .sum()
    }
}

impl Planner<'_> {
    /// Every rule `plan` breaks, sorted, each once.
    pub fn validate_plan(&self, plan: &Plan, s: &State, now: Timestamp) -> Vec<Violation> {
        let p = self.params;
        let t = f64::from(plan.budget.seconds());
        let mut out = Vec::new();
        let all_steps = || plan.done.iter().map(|d| &d.step).chain(&plan.steps);

        if plan
            .steps
            .iter()
            .any(|st| st.phase.is_learning() && self.ready(s, &st.kc, now) < p.ready_min)
        {
            out.push(Violation::BlockedKc);
        }

        let block_kcs: BTreeSet<&KcId> = all_steps()
            .filter(|st| matches!(st.phase, Phase::Lesson | Phase::Block))
            .map(|st| &st.kc)
            .collect();
        let learn_kc = plan.learn.as_ref().map(|l| &l.kc);
        let new_interleaved = plan.steps.iter().any(|st| {
            st.phase == Phase::Interleaved
                && Some(&st.kc) != learn_kc
                && self.progress(s, &st.kc, now) == KcProgress::NotStarted
        });
        if block_kcs.len() > 1 || new_interleaved {
            out.push(Violation::NewKcs);
        }

        if !block_kcs.is_empty() {
            let reviewed: BTreeSet<&KcId> = plan
                .done
                .iter()
                .filter(|d| d.step.phase == Phase::Review)
                .map(|d| &d.step.kc)
                .collect();
            let done: u32 = plan.shown_seconds(Phase::Review);
            let due: u32 = self
                .due(s, plan.started)
                .iter()
                .filter(|r| !reviewed.contains(&r.kc))
                .map(|r| r.seconds)
                .sum();
            if f64::from(done + due) > p.review_share * t {
                out.push(Violation::ReviewDebt);
            }
        }

        let practised: BTreeSet<&KcId> = plan
            .steps
            .iter()
            .filter(|st| matches!(st.phase, Phase::Block | Phase::Interleaved))
            .map(|st| &st.kc)
            .collect();
        if practised.iter().any(|kc| {
            !answered_production(plan, kc)
                && !plan.steps.iter().any(|st| st.production && &st.kc == *kc)
        }) {
            out.push(Violation::NoProduction);
        }

        let mut run: Option<(&KcId, usize)> = None;
        for st in all_steps().filter(|st| st.phase.counts_in_runs()) {
            let n = match run {
                Some((kc, n)) if kc == &st.kc => n + 1,
                _ => 1,
            };
            if n > p.max_run {
                out.push(Violation::RunTooLong);
                break;
            }
            run = Some((&st.kc, n));
        }

        for (phase, share, violation) in [
            (Phase::Review, p.review_share, Violation::ReviewShare),
            (Phase::Final, p.final_share, Violation::FinalShare),
        ] {
            let ahead: u32 = plan
                .steps
                .iter()
                .filter(|st| st.phase == phase)
                .map(|st| st.seconds)
                .sum();
            if f64::from(plan.shown_seconds(phase) + ahead) > share * t {
                out.push(violation);
            }
        }

        let shown_ok = plan
            .shown()
            .all(|(st, sh)| self.eligible_for(st, &sh.item, sh.slot));
        if !shown_ok || plan.steps.iter().any(|st| !self.servable(plan, st)) {
            out.push(Violation::Ineligible);
        }

        out.sort();
        out.dedup();
        out
    }

    /// The item is the current, session-eligible version and the step's KC
    /// is primary in the slot (and, for a final step, it is of its family).
    fn eligible_for(&self, step: &Step, item: &ItemRef, slot: u16) -> bool {
        let items = &self.bundle.content.items;
        let Ok(i) = items.binary_search_by(|e| e.item.id.cmp(&item.id)) else {
            return false;
        };
        let entry = &items[i];
        let primary = entry
            .item
            .body
            .slots()
            .get(usize::from(slot))
            .is_some_and(|sl| {
                sl.kcs
                    .iter()
                    .any(|k| k.kc == step.kc && k.role == KcRole::Primary)
            });
        let family = step.family.as_ref().is_none_or(|f| *f == entry.item.family);
        entry.session_eligible && entry.item.version == item.version && primary && family
    }

    /// A step ahead has an unseen eligible item that fits it.
    fn servable(&self, plan: &Plan, step: &Step) -> bool {
        if step.phase == Phase::Lesson {
            return self.is_live(&step.kc);
        }
        let shown: BTreeSet<_> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        self.slots(&step.kc).iter().any(|c| {
            !shown.contains(&c.item.id)
                && step
                    .family
                    .as_ref()
                    .is_none_or(|f| *f == c.projection.family)
                && (!step.production || c.projection.is_production())
        })
    }

    pub(super) fn violation_codes(&self, plan: &Plan, s: &State, now: Timestamp) -> Vec<String> {
        codes(&self.validate_plan(plan, s, now))
    }

    /// The plan's violations and those of the chosen item for its step.
    pub(super) fn choice_violation_codes(
        &self,
        plan: &Plan,
        step: &Step,
        item: &ItemRef,
        slot: u16,
        s: &State,
        now: Timestamp,
    ) -> Vec<String> {
        let mut v = self.validate_plan(plan, s, now);
        if !self.eligible_for(step, item, slot) {
            v.push(Violation::Ineligible);
        }
        let production = self
            .bundle
            .projection(item)
            .is_some_and(crate::projection::ItemProjection::is_production);
        if step.production && !production {
            v.push(Violation::NoProduction);
        }
        v.sort();
        v.dedup();
        codes(&v)
    }
}
