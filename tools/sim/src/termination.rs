//! Deterministic terminal `RunLimit` scenarios shared by the native/WASM
//! `plan-hash` check. The fixture is synthetic and uses the real planner
//! APIs; no session engine or production rule is duplicated here.

use refl_core::model::{KnowledgeModel, Profile, State};
use refl_core::plan::{
    Adapted, Budget, Done, Learn, Outcome, Params, Phase, Plan, PlannedReview, Planner,
    ReadingSpeed, Rule, Step,
};
use refl_core::value::{Lang, Seed, Timestamp};

use crate::{BundleSpec, T0, kc_id, synthetic_bundle};

/// The observable result of one terminal `RunLimit` scenario.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub end: Option<String>,
    pub steps: usize,
    pub review_steps: usize,
    pub final_steps: usize,
}

fn learning_plan(now: Timestamp, run: usize, reviews: Vec<PlannedReview>) -> Plan {
    let k0 = kc_id(0);
    Plan {
        budget: Budget::TwentyFive,
        started: now,
        seed: Seed::new(71).expect("seed"),
        learn: vec![Learn {
            kc: k0.clone(),
            contrasts: Vec::new(),
            lesson: false,
        }],
        candidates: vec![k0.clone()],
        deferred_today: Vec::new(),
        reviews,
        done: (0..run)
            .map(|_| Done {
                step: Step {
                    kc: k0.clone(),
                    phase: Phase::Interleaved,
                    production: false,
                    family: None,
                    seconds: 10,
                },
                shown: None,
                rule: None,
            })
            .chain([Done {
                step: Step {
                    kc: k0.clone(),
                    phase: Phase::Block,
                    production: false,
                    family: None,
                    seconds: 10,
                },
                shown: None,
                rule: Some(Rule::BlockClosed),
            }])
            .collect(),
        // A lesson consumes no item, so a pending review on k0 stays
        // pending while the counted run above is preserved.
        steps: vec![Step {
            kc: k0,
            phase: Phase::Lesson,
            production: false,
            family: None,
            seconds: 0,
        }],
        break_suggested: false,
        lang: Lang::Ru,
        end: None,
    }
}

fn outcome(adapted: &Adapted) -> Scenario {
    Scenario {
        end: adapted.plan.end.map(|e| e.code().to_string()),
        steps: adapted.plan.steps.len(),
        review_steps: adapted
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Review)
            .count(),
        final_steps: adapted
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Final)
            .count(),
    }
}

fn run(plan: &Plan) -> Scenario {
    let bundle = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 40,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading = ReadingSpeed::default();
    let planner = Planner::new(&model, &bundle, &params, &reading);
    let state = State::new(Profile::default());
    outcome(&planner.adapt(&state, plan, &Outcome::Lesson, T0))
}

/// A run-limited plan with no alternative: the terminal reason is
/// `run_limit`, not `items_exhausted`.
#[must_use]
pub fn scenario_terminal() -> Scenario {
    let k0 = kc_id(0);
    run(&learning_plan(
        T0,
        3,
        vec![PlannedReview {
            kc: k0,
            seconds: 10,
        }],
    ))
}

/// A run-limited learning KC with an eligible review: the review keeps the
/// plan alive and no terminal reason is recorded.
#[must_use]
pub fn scenario_review_alternative() -> Scenario {
    run(&learning_plan(
        T0,
        3,
        vec![PlannedReview {
            kc: kc_id(1),
            seconds: 10,
        }],
    ))
}
