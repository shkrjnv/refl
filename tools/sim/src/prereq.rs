//! Deterministic Rule 3 scenarios shared by the native/WASM `plan-hash`
//! check (`docs/04-placement-planner.md`, «Адаптация», rule 3).

use std::collections::BTreeSet;

use refl_core::bundle::Bundle;
use refl_core::ids::{ItemId, KcId};
use refl_core::item::KcRole;
use refl_core::model::{KcState, KnowledgeModel, Profile, State};
use refl_core::plan::{
    Adapted, Answered, Budget, Done, End, History, Learn, Next, Outcome, Params, Phase, Plan,
    Planner, ReadingSpeed, Rule, Shown, Step,
};
use refl_core::value::{Lang, Seed, Timestamp};

use crate::{BundleSpec, T0, item_ref, kc_id, synthetic_bundle};

/// The externally checkable outcome of one deterministic Rule 3 scenario.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub rule: String,
    pub selected: Option<KcId>,
    /// The `plan_issued.trigger` of the rule event.
    pub trigger: Option<String>,
    pub prerequisite_steps: usize,
    /// Steps after the prerequisite step in the same replan.
    pub subsequent_steps: usize,
    pub final_steps: usize,
    pub end: Option<String>,
    /// A real `next_item` + `adapt` call was made on the prerequisite step.
    pub continued: bool,
    /// Steps in the plan after that prerequisite answer.
    pub post_prerequisite_steps: usize,
    /// Pending prerequisite steps after that answer; expected zero.
    pub post_prerequisite_prereq_steps: usize,
}

fn learning_state(now: Timestamp) -> State {
    let mut s = State::new(Profile::default());
    for (index, mu) in [(3, -3.0), (4, -3.0)] {
        s.kcs.insert(
            kc_id(index),
            KcState {
                mu,
                var: 1.0,
                var_at: now,
                observations: 1,
                card: None,
                tau: 0.0,
                counted: 0,
                residuals: Vec::new(),
                production: Vec::new(),
                ever_mastered: false,
            },
        );
    }
    s
}

/// A plan learning `k5`, with one prior non-rapid error on `k5`.
fn base_plan(bundle: &Bundle, now: Timestamp, budget: Budget) -> Result<Plan, String> {
    let item = item_ref(5, 0);
    let projection = bundle
        .projection(&item)
        .ok_or_else(|| "k5 item projection is missing".to_string())?;
    Ok(Plan {
        budget,
        started: now,
        seed: Seed::new(61).ok_or_else(|| "seed".to_string())?,
        history: History::default(),
        return_check: false,
        learn: vec![Learn {
            kc: kc_id(5),
            contrasts: Vec::new(),
            lesson: false,
        }],
        candidates: Vec::new(),
        deferred_today: Vec::new(),
        reviews: Vec::new(),
        done: vec![Done {
            step: Step {
                kc: kc_id(5),
                phase: Phase::Block,
                production: false,
                family: None,
                seconds: projection.expected_time,
            },
            shown: Some(Shown {
                item,
                slot: 0,
                family: projection.family.clone(),
                kind: projection.kind,
                seconds: projection.expected_time,
                correct: false,
                rapid: false,
                sure_error: false,
            }),
            rule: None,
        }],
        steps: vec![Step {
            kc: kc_id(5),
            phase: Phase::Block,
            production: false,
            family: None,
            seconds: projection.expected_time,
        }],
        break_suggested: false,
        lang: Lang::Ru,
        end: None,
    })
}

fn slow_kc(bundle: &mut Bundle, kc: &KcId, seconds: u32) {
    let ids: BTreeSet<ItemId> = bundle
        .content
        .items
        .iter()
        .filter(|e| {
            e.item
                .body
                .slots()
                .iter()
                .flat_map(|slot| &slot.kcs)
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
        })
        .map(|e| e.item.id.clone())
        .collect();
    for e in &mut bundle.content.items {
        if ids.contains(&e.item.id) {
            e.item.expected_time = seconds;
        }
    }
    for v in &mut bundle.content.item_versions {
        if ids.contains(&v.item) {
            v.projection.expected_time = seconds;
        }
    }
}

fn answer_wrong(
    planner: &Planner<'_>,
    state: &State,
    plan: &Plan,
    now: Timestamp,
) -> Result<Adapted, String> {
    let Next::Item(choice) = planner.next_item(
        state,
        plan,
        &History::default(),
        now,
        Seed::new(62).unwrap(),
    ) else {
        return Err("Rule 3 scenario had no item".to_string());
    };
    Ok(planner.adapt(
        state,
        plan,
        &Outcome::Answer(Answered {
            item: choice.item,
            slot: choice.slot,
            correct: false,
            rapid: false,
            sure: false,
        }),
        now,
    ))
}

fn outcome(adapted: &Adapted) -> Scenario {
    let selected = match &adapted.rule {
        Some(Rule::PrerequisiteCheck { kc }) => Some(kc.clone()),
        _ => None,
    };
    let trigger = adapted
        .issued
        .iter()
        .find_map(|ev| ev.trigger.clone().filter(|t| t == "prerequisite_check"));
    Scenario {
        rule: adapted
            .rule
            .as_ref()
            .map_or_else(|| "none".to_string(), |r| r.code().to_string()),
        selected,
        trigger,
        prerequisite_steps: adapted
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Prerequisite)
            .count(),
        subsequent_steps: adapted
            .plan
            .steps
            .iter()
            .filter(|st| st.phase != Phase::Prerequisite)
            .count(),
        final_steps: adapted
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Final)
            .count(),
        end: adapted.plan.end.map(|e| e.code().to_string()),
        continued: false,
        post_prerequisite_steps: 0,
        post_prerequisite_prereq_steps: 0,
    }
}

fn continue_after_prerequisite(
    planner: &Planner<'_>,
    state: &State,
    plan: &Plan,
    now: Timestamp,
) -> Result<(usize, usize), String> {
    let Next::Item(choice) = planner.next_item(
        state,
        plan,
        &History::default(),
        now,
        Seed::new(63).ok_or_else(|| "seed".to_string())?,
    ) else {
        return Err("continuation had no prerequisite item".to_string());
    };
    let continued = planner.adapt(
        state,
        plan,
        &Outcome::Answer(Answered {
            item: choice.item,
            slot: choice.slot,
            correct: true,
            rapid: false,
            sure: false,
        }),
        now,
    );
    let post_prerequisite_steps = continued
        .plan
        .steps
        .iter()
        .filter(|st| st.phase != Phase::Prerequisite)
        .count();
    let post_prerequisite_prereq_steps = continued
        .plan
        .steps
        .iter()
        .filter(|st| st.phase == Phase::Prerequisite)
        .count();
    Ok((post_prerequisite_steps, post_prerequisite_prereq_steps))
}

/// A Rule 3 check whose prerequisite fits; k0 provides a subsequent
/// learning step so the plan demonstrably continues.
///
/// # Errors
/// Returns an error rather than guessing if the deterministic fixture no
/// longer produces the expected Rule 3 path.
pub fn scenario_a() -> Result<Scenario, String> {
    let bundle = synthetic_bundle(BundleSpec {
        kcs: 6,
        items_per_kc: 12,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading = ReadingSpeed::default();
    let planner = Planner::new(&model, &bundle, &params, &reading);
    let now = T0;
    let state = learning_state(now);
    let mut plan = base_plan(&bundle, now, Budget::TwentyFive)?;
    plan.learn.push(Learn {
        kc: kc_id(0),
        contrasts: Vec::new(),
        lesson: false,
    });
    let adapted = answer_wrong(&planner, &state, &plan, now)?;
    let mut result = outcome(&adapted);
    let (post_steps, post_prereq) =
        continue_after_prerequisite(&planner, &state, &adapted.plan, now)?;
    result.continued = true;
    result.post_prerequisite_steps = post_steps;
    result.post_prerequisite_prereq_steps = post_prereq;
    Ok(result)
}

/// A Rule 3 check whose prerequisite is selected but does not fit the
/// remaining main budget. The prior error still permits an independent
/// Final step.
///
/// # Errors
/// Returns an error rather than guessing if the deterministic fixture no
/// longer produces the expected Rule 3 path.
pub fn scenario_b() -> Result<Scenario, String> {
    let mut bundle = synthetic_bundle(BundleSpec {
        kcs: 6,
        items_per_kc: 12,
    });
    slow_kc(&mut bundle, &kc_id(4), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let reading = ReadingSpeed::default();
    let planner = Planner::new(&model, &bundle, &params, &reading);
    let now = T0;
    let state = learning_state(now);
    let mut plan = base_plan(&bundle, now, Budget::Ten)?;
    // Keep an active learning KC so the rule-3 decision still has a
    // plan_issued candidate while the over-budget prerequisite is skipped.
    plan.learn.push(Learn {
        kc: kc_id(0),
        contrasts: Vec::new(),
        lesson: false,
    });
    let adapted = answer_wrong(&planner, &state, &plan, now)?;
    Ok(outcome(&adapted))
}

/// The expected selected prerequisite in both scenarios.
#[must_use]
pub fn expected_selected() -> KcId {
    kc_id(4)
}

/// The expected terminal end of scenario B is none: Final remains.
#[must_use]
pub fn expected_b_end() -> Option<End> {
    None
}
