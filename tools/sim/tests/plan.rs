//! The prototype session builder (`docs/04-placement-planner.md`, «Сборка
//! сессии в прототипе»): no plan or decision breaks a rule on random
//! learner states and content, the same seeds give the same sessions, and
//! the validator catches every rule it checks.

use std::collections::BTreeMap;

use proptest::prelude::*;
use refl_core::bundle::{Bundle, KcStatus};
use refl_core::ids::{FamilyId, KcId, SubjectId};
use refl_core::model::{KnowledgeModel, Profile, State, replay};
use refl_core::plan::{
    Budget, End, History, Next, Params, Phase, Plan, Planner, ReadingSpeed, Step, Violation,
};
use refl_core::value::{Finite, Lang, Seed, Timestamp};
use refl_sim::cohort::{Learner, SessionReport, Track, session_ours};
use refl_sim::{
    BundleSpec, DAY_MS, LogSpec, SPELLING_KC, SUBJECT, TextReview, event_time, kc_id, reindex,
    synthetic_bundle, synthetic_log,
};

/// A synthetic bundle with some KCs back to draft.
fn bundle(kcs: usize, items_per_kc: usize, drafts: &[usize]) -> Bundle {
    let mut b = synthetic_bundle(BundleSpec { kcs, items_per_kc });
    for i in drafts {
        let id = kc_id(i % kcs);
        if let Some(e) = b.content.kcs.iter_mut().find(|e| e.kc.id == id) {
            e.status = KcStatus::Draft;
        }
    }
    reindex(&mut b, TextReview::Ignored);
    b
}

fn profile(level: u8) -> Profile {
    Profile {
        start_level: BTreeMap::from([(
            SubjectId::new(SUBJECT).unwrap(),
            Finite::new(f64::from(level)).unwrap(),
        )]),
        l1: None,
    }
}

#[derive(Debug, Clone)]
struct Case {
    kcs: usize,
    items_per_kc: usize,
    drafts: Vec<usize>,
    events: usize,
    log_seed: u64,
    level: u8,
    days: i64,
    budget: Budget,
    seed: u64,
    impatient: bool,
}

fn case() -> impl Strategy<Value = Case> {
    (
        (
            2usize..10,
            2usize..16,
            proptest::collection::vec(0usize..10, 0..3),
        ),
        (0usize..2500, any::<u64>(), 0u8..4),
        (
            1i64..4,
            prop_oneof![
                Just(Budget::Ten),
                Just(Budget::Fifteen),
                Just(Budget::TwentyFive)
            ],
            any::<u64>(),
            proptest::bool::weighted(0.2),
        ),
    )
        .prop_map(
            |(
                (kcs, items_per_kc, drafts),
                (events, log_seed, level),
                (days, budget, seed, impatient),
            )| {
                Case {
                    kcs,
                    items_per_kc,
                    drafts,
                    events,
                    log_seed,
                    level,
                    days,
                    budget,
                    seed,
                    impatient,
                }
            },
        )
}

/// `case.days` daily sessions after the case's log.
fn run(c: &Case) -> Vec<SessionReport> {
    let bundle = bundle(c.kcs, c.items_per_kc, &c.drafts);
    let log = synthetic_log(
        &bundle,
        LogSpec {
            events: c.events,
            seed: c.log_seed,
        },
    );
    let model = KnowledgeModel::default();
    let params = Params::default();
    let start = event_time(&log) + DAY_MS;
    let mut track = Track::new(&model, &bundle, Timestamp(start), c.seed);
    track.state = replay(&model, &profile(c.level), &bundle, &log);
    track.history = History::from_log(&bundle, &log);
    let mut learner = Learner::new(&bundle, c.seed);
    if c.impatient {
        learner = learner.impatient();
    }
    (0..c.days)
        .map(|day| {
            track.clock = track.clock.max(start + day * DAY_MS);
            session_ours(&mut track, &params, &mut learner, c.budget)
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 40, ..ProptestConfig::default() })]

    #[test]
    fn no_plan_or_decision_breaks_a_rule(c in case()) {
        for (day, r) in run(&c).iter().enumerate() {
            prop_assert!(r.violations.is_empty(), "day {day}: {:?}", r.violations);
            prop_assert!(r.checks > 0);
            // Every session ends by a plan, with its reason.
            prop_assert!(r.end.is_some(), "day {day}");
            let plan = r.plan.as_ref().unwrap();
            prop_assert_eq!(plan.end.map(End::code), r.end);
        }
    }

    #[test]
    fn same_seeds_same_sessions(c in case()) {
        let (a, b) = (run(&c), run(&c));
        for (x, y) in a.iter().zip(&b) {
            prop_assert_eq!(&x.issued, &y.issued);
            prop_assert_eq!(&x.plan, &y.plan);
        }
    }
}

/// A learner after `events` synthetic events, `days_later` days on.
fn state_after(bundle: &Bundle, events: usize, days_later: i64) -> (State, Timestamp) {
    let log = synthetic_log(bundle, LogSpec { events, seed: 5 });
    let model = KnowledgeModel::default();
    let state = replay(&model, &Profile::default(), bundle, &log);
    (state, Timestamp(event_time(&log) + days_later * DAY_MS))
}

fn step(kc: &KcId, phase: Phase, production: bool, seconds: u32) -> Step {
    Step {
        kc: kc.clone(),
        phase,
        production,
        family: None,
        seconds,
    }
}

fn empty_plan(now: Timestamp, budget: Budget) -> Plan {
    Plan {
        budget,
        started: now,
        seed: Seed::new(0).unwrap(),
        learn: None,
        reviews: Vec::new(),
        done: Vec::new(),
        steps: Vec::new(),
        break_suggested: false,
        lang: Lang::Ru,
        end: None,
    }
}

/// A planner without explanation views: lessons take the word prior.
fn planner<'a>(model: &'a KnowledgeModel, b: &'a Bundle, params: &'a Params) -> Planner<'a> {
    static NO_VIEWS: std::sync::OnceLock<ReadingSpeed> = std::sync::OnceLock::new();
    Planner::new(
        model,
        b,
        params,
        NO_VIEWS.get_or_init(ReadingSpeed::default),
    )
}

#[test]
fn the_validator_catches_every_rule() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 8,
        items_per_kc: 12,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let fresh = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let validate = |plan: &Plan, s: &State, t: Timestamp| planner.validate_plan(plan, s, t);
    let (k0, k1, k4) = (kc_id(0), kc_id(1), kc_id(4));

    let mut plan = empty_plan(now, Budget::TwentyFive);
    assert_eq!(validate(&plan, &fresh, now), Vec::<Violation>::new());

    // k1 requires k0, unknown to a fresh learner.
    plan.steps = vec![step(&k1, Phase::Interleaved, true, 10)];
    assert!(validate(&plan, &fresh, now).contains(&Violation::BlockedKc));

    plan.steps = vec![
        step(&k0, Phase::Block, true, 10),
        step(&k4, Phase::Block, true, 10),
    ];
    assert_eq!(validate(&plan, &fresh, now), [Violation::NewKcs]);

    plan.steps = vec![step(&k0, Phase::Block, false, 10)];
    assert_eq!(validate(&plan, &fresh, now), [Violation::NoProduction]);

    plan.steps = (0..4)
        .map(|_| step(&k0, Phase::Review, false, 10))
        .collect();
    assert_eq!(validate(&plan, &fresh, now), [Violation::RunTooLong]);

    plan.steps = vec![
        step(&k0, Phase::Review, false, 300),
        step(&k4, Phase::Review, false, 300),
    ];
    assert_eq!(validate(&plan, &fresh, now), [Violation::ReviewShare]);

    let mut fin = step(&k0, Phase::Final, false, 200);
    fin.family = Some(FamilyId::new("en.fam.k00.0").unwrap());
    plan.steps = vec![fin];
    assert_eq!(validate(&plan, &fresh, now), [Violation::FinalShare]);

    plan.steps = vec![step(
        &KcId::new(SPELLING_KC).unwrap(),
        Phase::Review,
        false,
        10,
    )];
    assert_eq!(validate(&plan, &fresh, now), [Violation::Ineligible]);

    // A month after a long log on 40 KCs every KC with a card is due: a
    // 10-minute session cannot fit them, so a block breaks the debt rule.
    let big = synthetic_bundle(BundleSpec {
        kcs: 40,
        items_per_kc: 12,
    });
    let planner = self::planner(&model, &big, &params);
    let (s, later) = state_after(&big, 4000, 30);
    let mut plan = empty_plan(later, Budget::Ten);
    plan.steps = vec![step(&k0, Phase::Block, true, 10)];
    assert!(
        planner
            .validate_plan(&plan, &s, later)
            .contains(&Violation::ReviewDebt)
    );
}

#[test]
fn review_debt_leaves_no_learning_kc() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 12,
        items_per_kc: 12,
    });
    let (s, later) = state_after(&b, 4000, 30);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let history = History::default();
    let session = planner.build_session(
        &s,
        &history,
        later,
        Budget::Ten,
        Lang::Ru,
        Seed::new(3).unwrap(),
    );
    assert_eq!(session.plan.learn, None);
    assert_ne!(session.plan.reviews.len(), 0);
    assert!(
        session
            .plan
            .steps
            .iter()
            .all(|st| matches!(st.phase, Phase::Review))
    );
    let issued = session.issued.expect("reviews are candidates");
    assert_eq!(issued.dominant, "memory");
    assert!((issued.propensity.get() - 1.0).abs() < f64::EPSILON);
    assert_eq!(issued.violations, Vec::<String>::new());
}

#[test]
fn a_fresh_learner_starts_with_the_lesson_and_a_block() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 8,
        items_per_kc: 12,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let history = History::default();
    let session = planner.build_session(
        &s,
        &history,
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(9).unwrap(),
    );
    let learn = session.plan.learn.clone().expect("a KC to learn");
    // The 120-word explanation takes 72 s of the budget at 100 words a minute.
    assert_eq!(session.plan.steps[0].seconds, 72);
    let phases: Vec<Phase> = session.plan.steps.iter().map(|st| st.phase).collect();
    assert_eq!(
        phases[..5],
        [
            Phase::Lesson,
            Phase::Block,
            Phase::Block,
            Phase::Block,
            Phase::Block
        ]
    );
    // The 4th block item is production.
    assert!(session.plan.steps[4].production);
    // Only KCs without prerequisites are ready for a fresh learner.
    assert_eq!(b.content.requires_closure[&learn.kc], []);
    let issued = session.issued.expect("an event");
    assert_eq!(issued.chosen as usize, issued
        .candidates
        .iter()
        .position(|c| matches!(&c.action, refl_core::event::PlanAction::Learn { kc } if *kc == learn.kc))
        .unwrap());
    assert!(matches!(
        planner.next_item(&s, &session.plan, &history, now, Seed::new(1).unwrap()),
        Next::Lesson { kc, .. } if kc == learn.kc
    ));
}

/// Views shrink the word prior towards their median: n₀ = 5 prior views
/// of 72 s and 5 real ones of 132 s make 102 s.
#[test]
fn explanation_time_follows_its_views() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 8,
        items_per_kc: 12,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let history = History::default();
    let lesson_seconds = |speed: &ReadingSpeed| {
        let planner = Planner::new(&model, &b, &params, speed);
        let session = planner.build_session(
            &s,
            &history,
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(9).unwrap(),
        );
        assert_eq!(session.plan.steps[0].phase, Phase::Lesson);
        session.plan.steps[0].seconds
    };
    assert_eq!(lesson_seconds(&ReadingSpeed::default()), 72);
    // Five views of 108 s of a 72 s explanation: the ratio is 1.5, pulled
    // towards 1 with n₀ = 5 gives 1.25, and every lesson takes 90 s: the
    // factor is the learner's, not the explanation's.
    let ids: Vec<_> = b.content.explanations.iter().map(|e| &e.id).collect();
    let views = [ids[0]; 5].map(|id| (id, 108));
    let speed = ReadingSpeed::from_views(&b, &params, views);
    assert_eq!(speed.views, 5);
    assert!((speed.factor - 1.25).abs() < 1e-12);
    assert_eq!(lesson_seconds(&speed), 90);
}

/// A KC deferred by the 12-items rule waits for the next UTC day; the
/// deferral is read from the rule's `plan_issued` in the log.
#[test]
fn a_deferred_kc_returns_the_next_day() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 8,
        items_per_kc: 12,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let s = State::new(Profile::default());
    // 18:00 UTC of a day.
    let now = Timestamp(refl_sim::T0.0 + 18 * 3_600_000);
    let build = |history: &History, at: Timestamp| {
        planner
            .build_session(
                &s,
                history,
                at,
                Budget::TwentyFive,
                Lang::Ru,
                Seed::new(9).unwrap(),
            )
            .plan
            .learn
            .map(|l| l.kc)
    };
    let first = build(&History::default(), now).expect("a KC to learn");
    let deferred = refl_sim::event(
        now,
        1,
        refl_core::event::EventPayload::PlanIssued(refl_core::event::PlanIssuedEvent {
            planner_version: planner.version(),
            model_version: "m".into(),
            scope: refl_core::event::PlanScope::Session,
            candidates: vec![refl_core::event::PlanCandidate {
                action: refl_core::event::PlanAction::Learn { kc: first.clone() },
                value_per_minute: Finite::new(1.0).unwrap(),
                factors: Vec::new(),
            }],
            chosen: 0,
            propensity: refl_core::value::UnitInterval::new(1.0).unwrap(),
            seed: Seed::new(1).unwrap(),
            violations: Vec::new(),
            dominant: "gap".into(),
            trigger: Some("kc_deferred".into()),
        }),
    );
    let history = History::from_log(&b, [&deferred]);
    assert_eq!(history.last_deferred(&first), Some(now));
    // Later the same day: another KC.
    let later = Timestamp(now.0 + 3_600_000);
    assert_ne!(build(&history, later), Some(first.clone()));
    // The next day it is a candidate again.
    assert_eq!(build(&history, Timestamp(now.0 + DAY_MS)), Some(first));
}

/// A plan with nothing to give ends at once and says why.
#[test]
fn an_empty_session_records_nothing_due() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 4,
    });
    let mut b = b;
    for e in &mut b.content.kcs {
        e.status = KcStatus::Draft;
    }
    reindex(&mut b, TextReview::Ignored);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let history = History::default();
    let session = planner.build_session(
        &s,
        &history,
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(1).unwrap(),
    );
    assert_eq!(session.plan.end, Some(End::NothingDue));
    assert_eq!(session.issued, None);
    assert_eq!(
        planner.next_item(&s, &session.plan, &history, now, Seed::new(2).unwrap()),
        Next::Finished {
            end: End::NothingDue
        }
    );
}
