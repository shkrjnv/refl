//! `plan_issued` of session decisions (`docs/04-placement-planner.md`,
//! «Лог решений прототипа»).

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::build::Start;
use super::values::{LearnValue, ReviewValue};
use super::{Plan, Planner, Rule};
use crate::event::{PlanAction, PlanCandidate, PlanFactor, PlanIssuedEvent, PlanScope};
use crate::model::{LearnerModel, State};
use crate::value::{Finite, Timestamp, UnitInterval};

/// A named multiplier; every planner value is finite by construction.
pub(super) fn factor(name: &str, value: f64) -> PlanFactor {
    PlanFactor {
        name: name.into(),
        value: finite(value),
    }
}

pub(super) fn finite(value: f64) -> Finite {
    Finite::new(value).expect("planner values are finite")
}

fn learn_candidate(v: &LearnValue) -> PlanCandidate {
    PlanCandidate {
        action: PlanAction::Learn { kc: v.kc.clone() },
        value_per_minute: finite(v.value),
        factors: vec![
            factor("importance", v.importance),
            factor("goal", v.goal),
            factor("gap", v.gap),
            factor("ready", v.ready),
            factor("misconception", v.misconception),
            factor("unlock", v.unlock),
        ],
    }
}

fn review_candidate(v: &ReviewValue) -> PlanCandidate {
    PlanCandidate {
        action: PlanAction::Review { kc: v.kc.clone() },
        value_per_minute: finite(v.value),
        factors: vec![
            factor("importance", v.importance),
            factor("goal", v.goal),
            factor("below_target", v.below_target),
            factor("decay", v.decay),
        ],
    }
}

/// What a session event chose.
enum Chosen<'v> {
    Break,
    Learn(usize, &'v LearnValue),
    Review,
}

impl Planner<'_> {
    #[allow(clippy::too_many_arguments)] // the parts of one event
    fn session_event(
        &self,
        s: &State,
        plan: &Plan,
        now: Timestamp,
        learn: &[LearnValue],
        reviews: &[ReviewValue],
        chosen: Option<Chosen<'_>>,
        propensity: f64,
        trigger: Option<&Rule>,
    ) -> Option<PlanIssuedEvent> {
        let chosen = chosen?;
        let with_break = matches!(chosen, Chosen::Break);
        let mut candidates: Vec<PlanCandidate> = Vec::new();
        if with_break {
            candidates.push(PlanCandidate {
                action: PlanAction::Break,
                value_per_minute: finite(0.0),
                factors: vec![factor("rapid_streak", f64::from(s.rapid.streak))],
            });
        }
        let offset = candidates.len();
        candidates.extend(learn.iter().map(learn_candidate));
        let reviews_at = candidates.len();
        candidates.extend(reviews.iter().map(review_candidate));
        let (index, dominant) = match chosen {
            Chosen::Break => (0, "rapid_guesses"),
            Chosen::Learn(i, v) => (offset + i, v.dominant()),
            Chosen::Review => (reviews_at, "memory"),
        };
        if index >= candidates.len() {
            return None;
        }
        Some(PlanIssuedEvent {
            planner_version: self.version(),
            model_version: self.model.version(),
            scope: PlanScope::Session,
            candidates,
            chosen: u16::try_from(index).unwrap_or(u16::MAX),
            propensity: UnitInterval::new(propensity).expect("a probability"),
            seed: plan.seed,
            violations: self.violation_codes(plan, s, now),
            dominant: String::from(dominant),
            trigger: trigger.map(|r| String::from(r.code())),
        })
    }

    /// The event of a freshly built session; `None` without candidates.
    pub(super) fn issue_start(
        &self,
        s: &State,
        plan: &Plan,
        start: &Start,
        now: Timestamp,
    ) -> Option<PlanIssuedEvent> {
        let (chosen, propensity) = match start.pick {
            Some((i, propensity)) => (Some(Chosen::Learn(i, &start.learn[i])), propensity),
            None => ((!start.due.is_empty()).then_some(Chosen::Review), 1.0),
        };
        self.session_event(
            s,
            plan,
            now,
            &start.learn,
            &start.due,
            chosen,
            propensity,
            None,
        )
    }

    /// The event of a plan changed by `rule`: the learning KC and the
    /// pending reviews with their values now. Under the deferral rule the
    /// chosen candidate is the deferred KC: the next sessions read it from
    /// the log ([`super::History`]).
    pub(super) fn issue_replan(
        &self,
        s: &State,
        plan: &Plan,
        rule: &Rule,
        now: Timestamp,
    ) -> Option<PlanIssuedEvent> {
        let learn_kc = if *rule == Rule::KcDeferred {
            plan.learn.as_ref()
        } else {
            self.active_learn(s, plan, now)
        };
        let learn: Vec<LearnValue> = learn_kc
            .map(|l| self.learn_value(s, &l.kc, now, &self.unlock(s, now)))
            .into_iter()
            .collect();
        let reviews: Vec<ReviewValue> = plan
            .pending_reviews()
            .iter()
            .filter_map(|r| self.review_value(s, &r.kc, now))
            .collect();
        let chosen = if *rule == Rule::RapidGuesses {
            Some(Chosen::Break)
        } else if let Some(v) = learn.first() {
            Some(Chosen::Learn(0, v))
        } else {
            (!reviews.is_empty()).then_some(Chosen::Review)
        };
        self.session_event(s, plan, now, &learn, &reviews, chosen, 1.0, Some(rule))
    }
}
