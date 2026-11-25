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
use crate::bundle::KcStatus;
use crate::ids::{ItemId, ItemRef, KcId};
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
    /// More learning KCs, or KCs with lesson or block steps, than a session
    /// may have, or a not-started KC other than a learning one in the
    /// interleaved part.
    NewKcs,
    /// Learning while reviews do not fit their share.
    ReviewDebt,
    /// A KC still practised without a production item.
    NoProduction,
    /// More than the allowed steps in a row on one KC after the block,
    /// while something else could get a step.
    RunTooLong,
    /// A learning KC going on alone gets more interleaved steps in the
    /// session than the cap.
    KcCap,
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
            Self::KcCap => "kc-cap",
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
        let new_interleaved = plan.steps.iter().any(|st| {
            st.phase == Phase::Interleaved
                && !plan.is_learn(&st.kc)
                && self.progress(s, &st.kc, now) == KcProgress::NotStarted
        });
        if plan.learn.len() > p.learn_max || block_kcs.len() > p.learn_max || new_interleaved {
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

        out.extend(self.run_violation(plan, s, now));

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

    /// The run rule, or the cap of a learning KC going on alone, on the
    /// steps ahead: the run carries over from the done steps, which were
    /// checked when they were planned.
    fn run_violation(&self, plan: &Plan, s: &State, now: Timestamp) -> Option<Violation> {
        let p = self.params;
        let counted = |st: &&Step| st.phase.counts_in_runs();
        if let Some(kc) = self.alone_kc(plan, s, now) {
            let ahead = plan
                .steps
                .iter()
                .filter(|st| st.phase == Phase::Interleaved && &st.kc == kc)
                .count();
            let ahead = u32::try_from(ahead).unwrap_or(u32::MAX);
            return (ahead > 0 && plan.interleaved_done(kc).saturating_add(ahead) > p.alone_cap)
                .then_some(Violation::KcCap);
        }
        let mut run: Option<(&KcId, usize)> = None;
        for st in plan.done.iter().map(|d| &d.step).filter(counted) {
            run = Some(match run {
                Some((kc, n)) if kc == &st.kc => (kc, n + 1),
                _ => (&st.kc, 1),
            });
        }
        for st in plan.steps.iter().filter(counted) {
            let n = match run {
                Some((kc, n)) if kc == &st.kc => n + 1,
                _ => 1,
            };
            if n > p.max_run {
                return Some(Violation::RunTooLong);
            }
            run = Some((&st.kc, n));
        }
        None
    }

    /// The validator's own reading of a learning KC going on alone: the only
    /// one still given (not deferred, Ready above the threshold, not
    /// mastered) while no review is left and no contrast of the learning
    /// KCs or started, not mastered start candidate has Ready above the
    /// threshold and an unseen eligible item, and no KC could open as the
    /// second learning KC. Written from the plan, the
    /// state and the content rather than with the planner's predicate, so
    /// that the run rule is checked, not restated.
    fn alone_kc<'p>(&self, plan: &'p Plan, s: &State, now: Timestamp) -> Option<&'p KcId> {
        let p = self.params;
        let mastered = |kc: &KcId| {
            matches!(
                self.progress(s, kc, now),
                KcProgress::Mastered | KcProgress::LongTerm
            )
        };
        let given: Vec<&KcId> = plan
            .learn
            .iter()
            .map(|l| &l.kc)
            .filter(|kc| {
                !plan.deferred(kc) && self.ready(s, kc, now) >= p.ready_min && !mastered(kc)
            })
            .collect();
        let [only] = given[..] else {
            return None;
        };
        let shown: BTreeSet<&ItemId> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        let review_left = plan
            .reviews
            .iter()
            .any(|r| plan.shown().all(|(st, _)| st.kc != r.kc));
        let contrasts = plan.learn.iter().flat_map(|l| &l.contrasts);
        let started = plan
            .candidates
            .iter()
            .filter(|kc| self.progress(s, kc, now) != KcProgress::NotStarted && !mastered(kc));
        let partner = contrasts.chain(started).any(|kc| {
            !plan.is_learn(kc)
                && self.ready(s, kc, now) >= p.ready_min
                && self.slots(kc).iter().any(|c| !shown.contains(&c.item.id))
        });
        // A second learning KC rule 6 could still open: any live KC, not
        // mastered, ready, not deferred today or in the session, not a
        // learning KC or review here, with an unseen production item.
        let may_open = plan.learn.len() < p.learn_max
            && self.bundle.content.kcs.iter().any(|e| {
                let kc = &e.kc.id;
                e.status == KcStatus::Live
                    && !plan.is_learn(kc)
                    && !plan.deferred_today.contains(kc)
                    && plan.reviews.iter().all(|r| &r.kc != kc)
                    && !mastered(kc)
                    && self.ready(s, kc, now) >= p.ready_min
                    && self
                        .slots(kc)
                        .iter()
                        .any(|c| c.projection.is_production() && !shown.contains(&c.item.id))
            });
        (!review_left && !partner && !may_open).then_some(only)
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
