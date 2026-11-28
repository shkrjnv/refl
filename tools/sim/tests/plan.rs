//! The prototype session builder (`docs/04-placement-planner.md`, «Сборка
//! сессии в прототипе»): no plan or decision breaks a rule on random
//! learner states and content, the same seeds give the same sessions, and
//! the validator catches every rule it checks.

use std::collections::BTreeMap;

use proptest::prelude::*;
use refl_core::bundle::{Bundle, KcStatus, Required};
use refl_core::event::{PlanAction, PlanScope};
use refl_core::ids::{FamilyId, ItemId, ItemRef, KcId, SubjectId};
use refl_core::item::KcRole;
use refl_core::model::fsrs::Card;
use refl_core::model::{KcState, KnowledgeModel, Profile, State, replay};
use refl_core::plan::{
    Adapted, Answered, Budget, Done, End, History, Learn, Next, Outcome, Params, Phase, Plan,
    PlannedReview, Planner, ReadingSpeed, Rule, SECOND_KC, Shown, Step, Violation,
};
use refl_core::value::{Finite, Lang, Seed, Timestamp, UnitInterval};
use refl_sim::cohort::{Learner, SessionReport, Track, session_ours};
use refl_sim::{
    BundleSpec, DAY_MS, LogSpec, SPELLING_KC, SUBJECT, TextReview, event_time, item_ref, kc_id,
    reindex, synthetic_bundle, synthetic_log,
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
            prop_assert!(plan.learn.len() <= 2, "day {day}");
            // The end is on the last session event, and only there.
            let session: Vec<_> =
                r.issued.iter().filter(|e| e.scope == PlanScope::Session).collect();
            if let Some((last, rest)) = session.split_last() {
                prop_assert_eq!(last.end.as_deref(), r.end, "day {}", day);
                prop_assert!(rest.iter().all(|e| e.end.is_none()), "day {day}");
            }
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

/// Rule 6: fresh learners on independent KCs have no partner once the
/// first block closes, so a second learning KC opens, logged with its draw,
/// and is interleaved with the first after its own block.
#[test]
fn a_replayed_state_and_history_build_the_same_plan() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 6,
        items_per_kc: 12,
    });
    let log = synthetic_log(
        &b,
        LogSpec {
            events: 600,
            seed: 31,
        },
    );
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let profile = Profile::default();
    let now = Timestamp(event_time(&log) + DAY_MS);
    let state = replay(&model, &profile, &b, &log);
    let history = History::from_log(&b, &log);

    let mut reversed_log = log.clone();
    reversed_log.reverse();
    let reversed_state = replay(&model, &profile, &b, &reversed_log);
    let reversed_history = History::from_log(&b, &reversed_log);
    assert_eq!(state.hash(), reversed_state.hash());

    let first = planner.build_session(
        &state,
        &history,
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(17).unwrap(),
    );
    let second = planner.build_session(
        &reversed_state,
        &reversed_history,
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(17).unwrap(),
    );
    assert_eq!(first.plan, second.plan);
    assert_eq!(first.issued, second.issued);
}

#[test]
fn a_closed_block_without_partners_opens_a_second_kc() {
    let b = bundle(6, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let mut opened = 0;
    for seed in 0..20 {
        let mut track = Track::new(&model, &b, refl_sim::T0, seed);
        let mut learner = Learner::new(&b, seed);
        let r = session_ours(&mut track, &params, &mut learner, Budget::TwentyFive);
        assert!(r.violations.is_empty(), "seed {seed}: {:?}", r.violations);
        let plan = r.plan.expect("a plan");
        let second = r
            .issued
            .iter()
            .filter(|e| e.trigger.as_deref() == Some(refl_core::plan::SECOND_KC))
            .collect::<Vec<_>>();
        assert!(second.len() <= 1);
        let Some(ev) = second.first() else {
            continue;
        };
        opened += 1;
        assert_eq!(plan.learn.len(), 2);
        let PlanAction::Learn { kc } = &ev.candidates[usize::from(ev.chosen)].action else {
            panic!("rule 6 chooses a KC");
        };
        assert_eq!(kc, &plan.learn[1].kc);
        assert_ne!(ev.seed, plan.seed);
        // After the second block both learning KCs alternate while both are
        // given; once one is deferred, the other goes on alone.
        let interleaved: Vec<&KcId> = plan
            .done
            .iter()
            .take_while(|d| d.rule != Some(Rule::KcDeferred))
            .filter(|d| d.step.phase == Phase::Interleaved)
            .map(|d| &d.step.kc)
            .collect();
        assert!(interleaved.windows(4).all(|w| w.iter().any(|k| *k != w[0])));
    }
    assert!(opened > 0, "rule 6 never fired");
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
        history: History::default(),
        return_check: false,
        learn: Vec::new(),
        candidates: Vec::new(),
        deferred_today: Vec::new(),
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

    // Two learning KCs may have blocks; a not-started KC that is not one of
    // them may not be interleaved.
    plan.steps = vec![
        step(&k0, Phase::Block, true, 10),
        step(&k4, Phase::Block, true, 10),
    ];
    assert_eq!(validate(&plan, &fresh, now), Vec::<Violation>::new());
    plan.steps = vec![step(&k4, Phase::Interleaved, true, 10)];
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
    // Two months later: since model v1.3 most synthetic answers have no
    // known tempo (their items recur within 14 days) and are graded Good, so
    // a month no longer makes enough reviews due.
    let (s, later) = state_after(&b, 4000, 60);
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
    assert_eq!(session.plan.learn, []);
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
    let learn = session.plan.learn.first().cloned().expect("a KC to learn");
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
            .first()
            .map(|l| l.kc.clone())
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
            end: None,
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

/// The longest run of steps on `kc` counted by the run rule.
fn longest_run(steps: &[&Step], kc: &KcId) -> usize {
    let mut run = (0, 0);
    for st in steps
        .iter()
        .filter(|st| !matches!(st.phase, Phase::Lesson | Phase::Block | Phase::Final))
    {
        run.0 = if &st.kc == kc { run.0 + 1 } else { 0 };
        run.1 = usize::max(run.1, run.0);
    }
    run.1
}

/// A fresh learner's first session on `b`, with the state and time after it.
fn first_session(b: &Bundle, params: &Params, seed: u64) -> (SessionReport, State, Timestamp) {
    let model = KnowledgeModel::default();
    let mut track = Track::new(&model, b, refl_sim::T0, seed);
    let mut learner = Learner::new(b, seed);
    let r = session_ours(&mut track, params, &mut learner, Budget::TwentyFive);
    let now = track.now();
    (r, track.state, now)
}

/// P1b on a graph with one root: a fresh learner has nothing ready but the
/// root, so after its block the root goes on alone, more than three in a
/// row, and the session ends by mastery, the cap, items or time, never for
/// lack of a partner.
#[test]
fn a_kc_without_partners_goes_on_after_its_block() {
    // k1 requires k0, k2 requires k1, k3 requires k2.
    let b = bundle(4, 40, &[]);
    let params = Params::default();
    let k0 = kc_id(0);
    let mut alone = 0;
    for seed in 0..20 {
        let (r, ..) = first_session(&b, &params, seed);
        assert!(r.violations.is_empty(), "seed {seed}: {:?}", r.violations);
        let plan = r.plan.expect("a plan");
        let end = plan.end.expect("an end");
        assert!(
            matches!(
                end,
                End::KcMastered | End::KcCap | End::ItemsExhausted | End::Budget | End::KcDeferred
            ),
            "seed {seed}: {end:?}"
        );
        let done: Vec<&Step> = plan.done.iter().map(|d| &d.step).collect();
        if plan.learn.len() == 1 && longest_run(&done, &k0) > params.max_run {
            alone += 1;
        }
        if end == End::KcCap {
            let on_k0 = done
                .iter()
                .filter(|st| st.phase == Phase::Interleaved && st.kc == k0)
                .count();
            // Steps interleaved earlier with a second learning KC count too.
            if plan.learn.len() == 1 {
                assert_eq!(on_k0, params.alone_cap as usize, "seed {seed}");
            } else {
                assert!(on_k0 >= params.alone_cap as usize, "seed {seed}");
            }
        }
    }
    assert!(alone > 0, "the root never went on alone");
}

/// The plan after its lesson, ready for the first block item.
fn after_lesson(planner: &Planner<'_>, s: &State, now: Timestamp, seed: u64) -> Plan {
    let history = History::default();
    let mut plan = planner
        .build_session(
            s,
            &history,
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(seed).unwrap(),
        )
        .plan;
    if plan
        .steps
        .first()
        .is_some_and(|st| st.phase == Phase::Lesson)
    {
        plan = planner.adapt(s, &plan, &Outcome::Lesson, now).plan;
    }
    plan
}

#[test]
fn a_confident_error_replans_and_puts_the_family_in_the_final() {
    let b = bundle(8, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let history = History::default();
    let plan = after_lesson(&planner, &s, now, 21);
    let Next::Item(choice) = planner.next_item(&s, &plan, &history, now, Seed::new(1).unwrap())
    else {
        panic!("the block has an item");
    };
    let answered = Answered {
        item: choice.item,
        slot: choice.slot,
        correct: false,
        rapid: false,
        sure: true,
    };
    let adapted = planner.adapt(&s, &plan, &Outcome::Answer(answered), now);
    assert_eq!(adapted.rule, Some(Rule::ConfidentError));
    assert!(
        adapted
            .issued
            .iter()
            .any(|e| { e.trigger.as_deref() == Some("confident_error") })
    );
    assert!(
        adapted.plan.steps.iter().any(|st| st.phase == Phase::Final),
        "the confident error is reviewed in the final"
    );
}

#[test]
fn a_rapid_streak_replans_to_a_break() {
    let b = bundle(8, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let mut s = State::new(Profile::default());
    s.rapid.streak = params.rapid_streak;
    let now = Timestamp(refl_sim::T0.0);
    let history = History::default();
    let plan = after_lesson(&planner, &s, now, 22);
    let Next::Item(choice) = planner.next_item(&s, &plan, &history, now, Seed::new(2).unwrap())
    else {
        panic!("the block has an item");
    };
    let answered = Answered {
        item: choice.item,
        slot: choice.slot,
        correct: true,
        rapid: true,
        sure: false,
    };
    let adapted = planner.adapt(&s, &plan, &Outcome::Answer(answered), now);
    assert_eq!(adapted.rule, Some(Rule::RapidGuesses));
    assert!(adapted.plan.break_suggested);
    let event = adapted
        .issued
        .iter()
        .find(|e| e.trigger.as_deref() == Some("rapid_guesses"))
        .expect("the break event");
    assert!(matches!(
        event.candidates[usize::from(event.chosen)].action,
        refl_core::event::PlanAction::Break
    ));
}

/// P1b, P1c: rule 6 is checked on every replan. A KC whose Ready is below
/// the threshold while the root goes on alone does not open; once it
/// reaches the threshold, it opens as the second learning KC, and
/// three-in-a-row applies again, counting the run the root has built.
#[test]
fn a_candidate_ready_mid_run_opens_the_second_kc() {
    let b = bundle(4, 40, &[]);
    let params = Params::default();
    let model = KnowledgeModel::default();
    let planner = planner(&model, &b, &params);
    let (k0, k1) = (kc_id(0), kc_id(1));
    let (r, state, now) = (0..20)
        .map(|seed| first_session(&b, &params, seed))
        .find(|(r, ..)| r.end == Some("kc_cap"))
        .expect("a session reaches the cap");
    // The plan after five interleaved steps of the root alone, one more
    // ahead.
    let mut plan = r.plan.expect("a plan");
    let cut = plan
        .done
        .iter()
        .enumerate()
        .filter(|(_, d)| d.step.phase == Phase::Interleaved)
        .nth(4)
        .map(|(i, _)| i + 1)
        .expect("five interleaved steps");
    plan.done.truncate(cut);
    plan.steps = vec![step(&k0, Phase::Interleaved, false, 10)];
    plan.end = None;
    let shown: Vec<&ItemId> = plan
        .done
        .iter()
        .filter_map(|d| d.shown.as_ref().map(|s| &s.item.id))
        .collect();
    let item = b.content.items_by_kc[&k0]
        .primary
        .iter()
        .find(|i| !shown.contains(&&i.id))
        .expect("an unseen item")
        .clone();
    let answer = Outcome::Answer(Answered {
        item,
        slot: 0,
        correct: true,
        rapid: false,
        sure: false,
    });
    // The root's μ puts k1's Ready below or above 0.5; without production
    // credit the root is not mastered either way.
    let with_mu = |z: f64| {
        let mut s = state.clone();
        let k = s.kcs.get_mut(&k0).expect("the root was answered");
        k.mu = 1.5 + z * libm::sqrt(k.var);
        k.production.clear();
        s
    };
    let second = |a: &Adapted| {
        a.issued
            .iter()
            .any(|e| e.trigger.as_deref() == Some(SECOND_KC))
    };

    let low = with_mu(-2.0);
    assert!(planner.ready(&low, &k1, now) < 0.5);
    let a = planner.adapt(&low, &plan, &answer, now);
    assert!(!second(&a));
    assert_eq!(a.plan.learn.len(), 1);
    let ahead: Vec<&Step> = a.plan.steps.iter().collect();
    assert!(longest_run(&ahead, &k0) > params.max_run);
    assert_eq!(planner.validate_plan(&a.plan, &low, now), []);

    let high = with_mu(2.0);
    assert!(planner.ready(&high, &k1, now) >= 0.5);
    let a = planner.adapt(&high, &plan, &answer, now);
    assert!(second(&a));
    assert_eq!(a.plan.learn.len(), 2);
    assert_eq!(a.plan.learn[1].kc, k1);
    // The root has six in a row: after k1's lesson and block, k1 comes
    // first, and no run ahead goes past three.
    let first = a
        .plan
        .steps
        .iter()
        .find(|st| !matches!(st.phase, Phase::Lesson | Phase::Block))
        .expect("interleaved steps ahead");
    assert_eq!(first.kc, k1);
    let ahead: Vec<&Step> = a.plan.steps.iter().collect();
    assert!(longest_run(&ahead, &k0) <= params.max_run);
    assert_eq!(planner.validate_plan(&a.plan, &high, now), []);
}

/// P1c on a graph with one root: learners placed a little higher start with
/// only the root ready; practising it in the session lifts its child's
/// Ready across the threshold, and the child, not a start candidate, opens
/// as the second learning KC. Whether a run may pass three is checked on
/// every decision by the validator's own reading (no violation): the child
/// leaves the runs alone only while its Ready is above the threshold, and
/// errors on the root can drop it below again.
#[test]
fn a_child_made_ready_in_the_session_opens() {
    let b = bundle(4, 40, &[]);
    let params = Params::default();
    let model = KnowledgeModel::default();
    let (k0, k1) = (kc_id(0), kc_id(1));
    let mut opened = 0;
    for (level, seed) in (0..4).flat_map(|l| (0..20).map(move |s| (l, s))) {
        let mut track = Track::new(&model, &b, refl_sim::T0, seed);
        track.state = State::new(profile(level));
        let mut learner = Learner::new(&b, seed);
        let r = session_ours(&mut track, &params, &mut learner, Budget::TwentyFive);
        assert!(
            r.violations.is_empty(),
            "level {level} seed {seed}: {:?}",
            r.violations
        );
        let plan = r.plan.expect("a plan");
        if plan.learn.first().map(|l| &l.kc) != Some(&k0)
            || plan.candidates.contains(&k1)
            || plan.learn.get(1).map(|l| &l.kc) != Some(&k1)
        {
            continue;
        }
        opened += 1;
        let ev = r
            .issued
            .iter()
            .find(|e| e.trigger.as_deref() == Some(SECOND_KC))
            .expect("rule 6 logged");
        let PlanAction::Learn { kc } = &ev.candidates[usize::from(ev.chosen)].action else {
            panic!("rule 6 chooses a KC");
        };
        assert_eq!(kc, &k1);
    }
    assert!(opened > 0, "no child opened in the session");
}

/// P1b: a single KC goes on alone to exactly `alone_cap` interleaved steps
/// and the session ends `kc_cap`; the validator accepts a run alone up to
/// the cap and flags the step past it.
#[test]
fn a_kc_alone_stops_at_the_cap() {
    let b = bundle(1, 40, &[]);
    let params = Params::default();
    let model = KnowledgeModel::default();
    let planner = planner(&model, &b, &params);
    let k0 = kc_id(0);
    let mut capped = 0;
    for seed in 0..10 {
        let model = KnowledgeModel::default();
        let mut track = Track::new(&model, &b, refl_sim::T0, seed);
        let mut learner = Learner::new(&b, seed);
        let r = session_ours(&mut track, &params, &mut learner, Budget::TwentyFive);
        assert!(r.violations.is_empty(), "seed {seed}: {:?}", r.violations);
        let plan = r.plan.expect("a plan");
        if plan.end != Some(End::KcCap) {
            continue;
        }
        capped += 1;
        let interleaved = |p: &Plan| {
            p.done
                .iter()
                .filter(|d| d.step.phase == Phase::Interleaved)
                .count()
        };
        assert_eq!(interleaved(&plan), params.alone_cap as usize, "seed {seed}");
        assert_eq!(r.end, Some("kc_cap"));

        let (s, now) = (&track.state, track.now());
        let first = plan
            .done
            .iter()
            .position(|d| d.step.phase == Phase::Interleaved)
            .expect("interleaved steps");
        let mut ahead = plan.clone();
        ahead.done.truncate(first);
        ahead.end = None;
        let alone = |n: usize| -> Vec<Step> {
            (0..n)
                .map(|_| step(&k0, Phase::Interleaved, false, 10))
                .collect()
        };
        ahead.steps = alone(params.alone_cap as usize);
        assert_eq!(planner.validate_plan(&ahead, s, now), []);
        ahead.steps = alone(params.alone_cap as usize + 1);
        assert_eq!(planner.validate_plan(&ahead, s, now), [Violation::KcCap]);
    }
    assert!(capped > 0, "no session reached the cap");
}

/// A plan with nothing to give ends at once and says why.
/// Every item is far longer than a 10-minute session's main budget.
fn slow_bundle() -> Bundle {
    let mut b = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 8,
    });
    for e in &mut b.content.items {
        e.item.expected_time = 1_000;
    }
    for v in &mut b.content.item_versions {
        v.projection.expected_time = 1_000;
    }
    b
}

#[test]
fn a_block_that_does_not_fit_the_budget_ends_it() {
    let b = slow_bundle();
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let session = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::Ten,
        Lang::Ru,
        Seed::new(3).unwrap(),
    );
    let plan = session.plan;
    assert_eq!(plan.steps.len(), 1, "only the lesson fits");
    assert_eq!(plan.steps[0].phase, Phase::Lesson);
    let adapted = planner.adapt(&s, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::Budget));
    assert_eq!(adapted.plan.steps.len(), 0);
}

#[test]
fn a_due_review_that_cannot_fit_ends_budget_not_nothing_due() {
    let b = slow_bundle();
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut s = State::new(Profile::default());
    s.kcs.insert(
        kc_id(0),
        KcState {
            mu: 0.0,
            var: 2.25,
            var_at: now,
            observations: 1,
            card: Some(Card {
                stability: 1.0,
                difficulty: 5.0,
                last_review: Timestamp(now.0 - 10 * DAY_MS),
            }),
            tau: 0.0,
            counted: 0,
            residuals: Vec::new(),
            production: Vec::new(),
            ever_mastered: false,
        },
    );
    let session = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::Ten,
        Lang::Ru,
        Seed::new(4).unwrap(),
    );
    assert_eq!(session.plan.end, Some(End::Budget));
    assert_eq!(session.plan.reviews.len(), 0);
    assert_eq!(session.plan.steps.len(), 0);
}

#[test]
fn micro_randomisation_is_seeded_and_records_the_propensity() {
    let mut b = synthetic_bundle(BundleSpec {
        kcs: 9,
        items_per_kc: 6,
    });
    for e in &mut b.content.kcs {
        e.kc.importance = UnitInterval::new(1.0).expect("unit");
    }
    let model = KnowledgeModel::default();
    let params = Params {
        zeta: 0.0,
        ..Params::default()
    };
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut s = State::new(Profile::default());
    for i in [0usize, 4, 8] {
        s.kcs.insert(
            kc_id(i),
            KcState {
                mu: 0.0,
                var: 2.25,
                var_at: now,
                observations: 0,
                card: None,
                tau: 0.0,
                counted: 0,
                residuals: Vec::new(),
                production: Vec::new(),
                ever_mastered: false,
            },
        );
    }
    let seed = Seed::new(11).unwrap();
    let session = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::TwentyFive,
        Lang::Ru,
        seed,
    );
    let plan = session.plan;
    let issued = session.issued.expect("learning candidates");
    assert_eq!(plan.candidates.len(), 3);
    assert_eq!(issued.seed, seed);
    assert_eq!(issued.candidates.len(), 3);
    assert!((issued.propensity.get() - 1.0 / 3.0).abs() < f64::EPSILON);
    let PlanAction::Learn { kc } = &issued.candidates[usize::from(issued.chosen)].action else {
        panic!("a learning KC was chosen");
    };
    assert_eq!(kc, &plan.learn[0].kc);

    let again = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::TwentyFive,
        Lang::Ru,
        seed,
    );
    assert_eq!(again.plan, plan);
}

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

#[test]
fn reviews_done_when_no_learning_kc_is_left() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut s = State::new(Profile::default());
    s.kcs.insert(
        kc_id(0),
        KcState {
            mu: 0.0,
            var: 2.25,
            var_at: now,
            observations: 1,
            card: Some(Card {
                stability: 1.0,
                difficulty: 5.0,
                last_review: Timestamp(now.0 - 10 * DAY_MS),
            }),
            tau: 0.0,
            counted: 0,
            residuals: Vec::new(),
            production: Vec::new(),
            ever_mastered: false,
        },
    );
    let session = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(5).unwrap(),
    );
    let plan = session.plan;
    assert_eq!(plan.learn.len(), 0);
    assert_eq!(plan.reviews.len(), 1);
    let Next::Item(choice) =
        planner.next_item(&s, &plan, &History::default(), now, Seed::new(6).unwrap())
    else {
        panic!("a review item");
    };
    let answered = Answered {
        item: choice.item,
        slot: choice.slot,
        correct: true,
        rapid: false,
        sure: false,
    };
    let adapted = planner.adapt(&s, &plan, &Outcome::Answer(answered), now);
    assert_eq!(adapted.plan.end, Some(End::ReviewsDone));
}

fn mastered_state(bundle: &Bundle, now: Timestamp) -> State {
    let model = KnowledgeModel::default();
    let profile = Profile::default();
    let prior = model.prior(&profile, bundle, &kc_id(0)).expect("prior").0;
    debug_assert!(prior < 3.0);
    let mut s = State::new(profile);
    s.kcs.insert(
        kc_id(0),
        KcState {
            mu: 10.0,
            var: 0.01,
            var_at: now,
            observations: 10,
            card: Some(Card {
                stability: 30.0,
                difficulty: 5.0,
                last_review: now,
            }),
            tau: 1.0,
            counted: 6,
            residuals: vec![0.0; 6],
            production: vec![item_ref(0, 1).id, item_ref(0, 3).id],
            ever_mastered: true,
        },
    );
    s
}

#[test]
fn kc_mastered_when_the_learning_kc_is_mastered_during_the_session() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let plan = planner
        .build_session(
            &fresh,
            &History::default(),
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(7).unwrap(),
        )
        .plan;
    assert_eq!(plan.learn.len(), 1);
    let mastered = mastered_state(&b, now);
    let adapted = planner.adapt(&mastered, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::KcMastered));
}

#[test]
fn kc_deferred_when_the_learning_kc_is_set_aside() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let mut plan = planner
        .build_session(
            &fresh,
            &History::default(),
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(8).unwrap(),
        )
        .plan;
    plan.done.push(Done {
        step: step(&kc_id(0), Phase::Block, false, 10),
        shown: None,
        rule: Some(Rule::KcDeferred),
    });
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::KcDeferred));
}

#[test]
fn items_exhausted_when_all_learning_items_are_shown() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let mut plan = planner
        .build_session(
            &fresh,
            &History::default(),
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(9).unwrap(),
        )
        .plan;
    for j in 0..8usize {
        let item = item_ref(0, j);
        let projection = b.projection(&item).expect("projection");
        plan.done.push(Done {
            step: step(&kc_id(0), Phase::Block, false, projection.expected_time),
            shown: Some(Shown {
                item,
                slot: 0,
                family: projection.family.clone(),
                kind: projection.kind,
                seconds: projection.expected_time,
                correct: true,
                rapid: false,
                sure_error: false,
            }),
            rule: None,
        });
    }
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::ItemsExhausted));
}

#[test]
fn an_unavailable_block_over_budget_is_not_budget() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let k0 = kc_id(0);
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.learn = vec![Learn {
        kc: k0.clone(),
        contrasts: Vec::new(),
        lesson: false,
    }];
    plan.candidates = vec![k0.clone()];
    // Leave the block open, consume almost the main budget, and show all
    // eligible items: the block candidate is unavailable and over budget.
    plan.done.push(Done {
        step: step(&k0, Phase::Review, false, 1345),
        shown: None,
        rule: None,
    });
    show_all_items(&b, &mut plan, &k0);
    plan.steps = vec![step(&k0, Phase::Lesson, false, 0)];
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::ItemsExhausted));
    assert_ne!(adapted.plan.end, Some(End::Budget));
}

#[test]
fn review_prefix_skips_unavailable_review_and_keeps_later_available_one() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 2,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let k0 = kc_id(0);
    let k1 = kc_id(1);
    let mut plan = planner
        .build_session(
            &fresh,
            &History::default(),
            now,
            Budget::TwentyFive,
            Lang::Ru,
            Seed::new(11).unwrap(),
        )
        .plan;
    plan.candidates = vec![k0.clone()];
    // The k0 block consumes both unseen items before reviews. R1 is
    // therefore unavailable even though plan.shown() still reports its item;
    // its duration must not count against R2's time prefix.
    plan.reviews = vec![
        PlannedReview {
            kc: k0.clone(),
            seconds: 6,
        },
        PlannedReview {
            kc: k1.clone(),
            seconds: 5,
        },
    ];
    plan.done.push(Done {
        step: step(&k0, Phase::Review, false, 1320),
        shown: None,
        rule: None,
    });
    plan.steps = vec![step(&k0, Phase::Lesson, false, 0)];
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, None);
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|s| s.phase == Phase::Review && s.kc == k1)
    );
    assert!(
        !adapted
            .plan
            .steps
            .iter()
            .any(|s| s.phase == Phase::Review && s.kc == k0)
    );
}

#[test]
fn review_prefix_accepts_exactly_the_remaining_budget() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let k1 = kc_id(1);
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.reviews.push(PlannedReview { kc: k1, seconds: 5 });
    // main_left is exactly 5s: 1350 - 1345.
    plan.done.push(Done {
        step: step(&kc_id(0), Phase::Review, false, 1345),
        shown: None,
        rule: None,
    });
    plan.steps = vec![step(&kc_id(0), Phase::Lesson, false, 0)];
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, None);
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|s| s.phase == Phase::Review && s.kc == kc_id(1))
    );
}

#[test]
fn review_prefix_rejects_over_budget_review_as_budget() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 2,
        items_per_kc: 8,
    });
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let fresh = State::new(Profile::default());
    let k1 = kc_id(1);
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.reviews.push(PlannedReview { kc: k1, seconds: 5 });
    // main_left is exactly 4s: 1350 - 1346; the Review does not fit.
    plan.done.push(Done {
        step: step(&kc_id(0), Phase::Review, false, 1346),
        shown: None,
        rule: None,
    });
    plan.steps = vec![step(&kc_id(0), Phase::Lesson, false, 0)];
    let adapted = planner.adapt(&fresh, &plan, &Outcome::Lesson, now);
    assert_eq!(adapted.plan.end, Some(End::Budget));
    assert!(
        !adapted
            .plan
            .steps
            .iter()
            .any(|s| s.phase == Phase::Review && s.kc == kc_id(1))
    );
}

fn mastered_kc_state(index: usize, now: Timestamp) -> KcState {
    KcState {
        mu: 10.0,
        var: 0.01,
        var_at: now,
        observations: 10,
        card: Some(Card {
            stability: 30.0,
            difficulty: 5.0,
            last_review: now,
        }),
        tau: 1.0,
        counted: 6,
        residuals: vec![0.0; 6],
        production: vec![item_ref(index, 1).id, item_ref(index, 3).id],
        ever_mastered: true,
    }
}

fn prereq_states(bundle: &Bundle, now: Timestamp) -> (State, State) {
    let k0 = kc_id(0);
    let k4 = kc_id(4);
    let mut high = mastered_state(bundle, now);
    high.kcs.insert(k4.clone(), mastered_kc_state(4, now));
    let mut low = high.clone();
    let k0_state = low.kcs.get_mut(&k0).expect("k0");
    k0_state.mu = -3.0;
    k0_state.var = 1.0;
    k0_state.var_at = now;
    low.kcs.remove(&k4);
    (high, low)
}

fn learning_plan(
    planner: &Planner<'_>,
    s: &State,
    now: Timestamp,
    seed: u64,
    budget: Budget,
) -> Plan {
    let mut plan = planner
        .build_session(
            s,
            &History::default(),
            now,
            budget,
            Lang::Ru,
            Seed::new(seed).unwrap(),
        )
        .plan;
    assert_eq!(plan.learn.len(), 1);
    assert_eq!(plan.learn[0].kc, kc_id(1));
    if plan
        .steps
        .first()
        .is_some_and(|st| st.phase == Phase::Lesson)
    {
        plan = planner.adapt(s, &plan, &Outcome::Lesson, now).plan;
    }
    assert!(
        plan.steps
            .first()
            .is_some_and(|st| st.phase == Phase::Block && st.kc == kc_id(1)),
        "the learning KC starts with a block"
    );
    plan
}

fn add_learn(plan: &mut Plan, kc: &KcId) {
    plan.learn.push(refl_core::plan::Learn {
        kc: kc.clone(),
        contrasts: Vec::new(),
        lesson: false,
    });
}

fn slow_kc(bundle: &mut Bundle, kc: &KcId, seconds: u32) {
    let ids: std::collections::BTreeSet<ItemId> = bundle
        .content
        .items
        .iter()
        .filter(|e| {
            e.item
                .body
                .slots()
                .iter()
                .flat_map(|sl| &sl.kcs)
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

fn show_all_items(bundle: &Bundle, plan: &mut Plan, kc: &KcId) {
    for entry in &bundle.content.items {
        let item = ItemRef {
            id: entry.item.id.clone(),
            version: entry.item.version,
            seed: None,
        };
        let Some(projection) = bundle.projection(&item) else {
            continue;
        };
        if !projection.slots.iter().any(|slot| {
            slot.kcs
                .iter()
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
        }) {
            continue;
        }
        plan.done.push(Done {
            step: step(kc, Phase::Review, false, projection.expected_time),
            shown: Some(Shown {
                item,
                slot: 0,
                family: projection.family.clone(),
                kind: projection.kind,
                seconds: projection.expected_time,
                correct: true,
                rapid: false,
                sure_error: false,
            }),
            rule: None,
        });
    }
}

fn add_interleaved_run(plan: &mut Plan, kc: &KcId, length: usize) {
    for _ in 0..length {
        plan.done.push(Done {
            step: step(kc, Phase::Interleaved, false, 10),
            shown: None,
            rule: None,
        });
    }
}

/// A plan already at a run on `k0`, its block closed, with a first step to
/// consume and the given pending reviews.
fn run_limit_plan(now: Timestamp, run: usize, reviews: Vec<PlannedReview>) -> Plan {
    let k0 = kc_id(0);
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.learn = vec![Learn {
        kc: k0.clone(),
        contrasts: Vec::new(),
        lesson: false,
    }];
    plan.candidates = vec![k0.clone()];
    plan.reviews = reviews;
    add_interleaved_run(&mut plan, &k0, run);
    plan.done.push(Done {
        step: step(&k0, Phase::Block, false, 10),
        shown: None,
        rule: Some(Rule::BlockClosed),
    });
    // A lesson is consumed without showing an item, so the k0 review stays
    // pending even though this fixture has no second KC.
    plan.steps = vec![step(&k0, Phase::Lesson, false, 0)];
    plan
}

fn start_run_limit(planner: &Planner<'_>, state: &State, plan: &Plan, now: Timestamp) -> Adapted {
    planner.adapt(state, plan, &Outcome::Lesson, now)
}

fn add_production_answer(bundle: &Bundle, plan: &mut Plan, kc: &KcId) {
    let item = bundle.content.items_by_kc[kc]
        .primary
        .iter()
        .find(|item| {
            bundle
                .projection(item)
                .is_some_and(refl_core::projection::ItemProjection::is_production)
        })
        .expect("a production item")
        .clone();
    let projection = bundle.projection(&item).expect("projection");
    plan.done.push(Done {
        step: step(kc, Phase::Review, false, projection.expected_time),
        shown: Some(Shown {
            item,
            slot: 0,
            family: projection.family.clone(),
            kind: projection.kind,
            seconds: projection.expected_time,
            correct: true,
            rapid: false,
            sure_error: false,
        }),
        rule: None,
    });
}

fn answer(
    planner: &Planner<'_>,
    s: &State,
    plan: &Plan,
    now: Timestamp,
    seed: u64,
    correct: bool,
    sure: bool,
) -> Adapted {
    let Next::Item(choice) =
        planner.next_item(s, plan, &History::default(), now, Seed::new(seed).unwrap())
    else {
        panic!("the planned step has an item");
    };
    planner.adapt(
        s,
        plan,
        &Outcome::Answer(Answered {
            item: choice.item,
            slot: choice.slot,
            correct,
            rapid: false,
            sure,
        }),
        now,
    )
}

#[test]
fn a_prerequisite_that_fits_is_scheduled_first() {
    let b = bundle(5, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let (high, low) = prereq_states(&b, now);
    let mut plan = learning_plan(&planner, &high, now, 31, Budget::TwentyFive);

    let first = answer(&planner, &high, &plan, now, 1, false, false);
    plan = first.plan;
    add_learn(&mut plan, &kc_id(4));
    let second = answer(&planner, &low, &plan, now, 2, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert_eq!(
        second.plan.steps.first().map(|st| (st.phase, &st.kc)),
        Some((Phase::Prerequisite, &kc_id(0)))
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Block && st.kc == kc_id(4)),
        "the other learning KC still gets its block after the prerequisite"
    );
    assert_eq!(second.plan.end, None);
}

#[test]
fn a_prerequisite_that_exceeds_the_main_budget_is_not_scheduled() {
    let mut b = bundle(2, 12, &[]);
    slow_kc(&mut b, &kc_id(0), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let high = mastered_state(&b, now);
    let mut low = high.clone();
    let k0 = low.kcs.get_mut(&kc_id(0)).expect("k0");
    k0.mu = -3.0;
    k0.var = 1.0;
    k0.var_at = now;
    let plan = learning_plan(&planner, &high, now, 32, Budget::Ten);

    let first = answer(&planner, &high, &plan, now, 3, false, false);
    let second = answer(&planner, &low, &first.plan, now, 4, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second.plan.steps.iter().all(|st| st.phase == Phase::Final),
        "only the independently fitting final remains"
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the over-budget prerequisite is not scheduled"
    );
    assert_eq!(second.plan.end, None);
}

#[test]
fn an_unfittable_prerequisite_is_not_replaced_by_another_step() {
    let mut b = bundle(2, 12, &[]);
    slow_kc(&mut b, &kc_id(0), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let high = mastered_state(&b, now);
    let mut low = high.clone();
    let k0 = low.kcs.get_mut(&kc_id(0)).expect("k0");
    k0.mu = -3.0;
    k0.var = 1.0;
    k0.var_at = now;
    let plan = learning_plan(&planner, &high, now, 33, Budget::Ten);

    let first = answer(&planner, &high, &plan, now, 5, false, false);
    let second = answer(&planner, &low, &first.plan, now, 6, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second.plan.steps.iter().all(|st| !matches!(
            st.phase,
            Phase::Lesson | Phase::Block | Phase::Review | Phase::Interleaved
        )),
        "no lower-priority main step replaces the unfittable prerequisite"
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the unfittable prerequisite is not scheduled"
    );
    assert_eq!(second.plan.end, None);
}

#[test]
fn a_final_can_still_be_scheduled_when_a_prerequisite_does_not_fit() {
    let mut b = bundle(2, 12, &[]);
    slow_kc(&mut b, &kc_id(0), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let high = mastered_state(&b, now);
    let mut low = high.clone();
    let k0 = low.kcs.get_mut(&kc_id(0)).expect("k0");
    k0.mu = -3.0;
    k0.var = 1.0;
    k0.var_at = now;
    let plan = learning_plan(&planner, &high, now, 34, Budget::Ten);

    let first = answer(&planner, &high, &plan, now, 7, false, false);
    let second = answer(&planner, &low, &first.plan, now, 8, false, true);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second.plan.steps.iter().any(|st| st.phase == Phase::Final),
        "the independently fitting final is kept"
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the unfittable prerequisite is not added"
    );
    assert_eq!(second.plan.end, None);
}

#[test]
fn a_prerequisite_without_an_unseen_item_is_not_selected() {
    let b = bundle(5, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let (high, low) = prereq_states(&b, now);
    let mut plan = learning_plan(&planner, &high, now, 35, Budget::TwentyFive);
    show_all_items(&b, &mut plan, &kc_id(0));

    let first = answer(&planner, &high, &plan, now, 9, false, false);
    let mut next = first.plan;
    add_learn(&mut next, &kc_id(4));
    let second = answer(&planner, &low, &next, now, 10, false, false);

    assert_eq!(second.rule, None);
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "a selected prerequisite must have an unseen eligible item"
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Block && st.kc == kc_id(4)),
        "normal planning continues after the unavailable prerequisite is skipped"
    );
}

#[test]
fn a_run_limited_prerequisite_is_not_a_budget_failure() {
    let mut b = bundle(5, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let fast_planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let (high, low) = prereq_states(&b, now);
    let mut plan = learning_plan(&fast_planner, &high, now, 36, Budget::TwentyFive);
    add_interleaved_run(&mut plan, &kc_id(0), params.max_run);

    let first = answer(&fast_planner, &high, &plan, now, 11, false, false);

    // Make the final impossible and leave no alternative KC, so the end
    // reason is observable. The prerequisite run rejection must not be
    // converted into a time failure.
    slow_kc(&mut b, &kc_id(1), 1_000);
    let slow_planner = planner(&model, &b, &params);
    let second = answer(&slow_planner, &low, &first.plan, now, 12, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the run rule rejects the prerequisite"
    );
    assert!(
        second.plan.steps.iter().all(|st| st.phase != Phase::Final),
        "the slowed final does not fit"
    );
    assert_ne!(
        second.plan.end,
        Some(End::Budget),
        "a run-limit rejection is not a time failure"
    );
}

#[test]
fn another_kc_can_follow_a_run_limited_prerequisite() {
    let b = bundle(5, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let (high, low) = prereq_states(&b, now);
    let mut plan = learning_plan(&planner, &high, now, 37, Budget::TwentyFive);
    add_interleaved_run(&mut plan, &kc_id(0), params.max_run);

    let first = answer(&planner, &high, &plan, now, 13, false, false);
    let mut next = first.plan;
    add_learn(&mut next, &kc_id(4));
    next.reviews.push(PlannedReview {
        kc: kc_id(0),
        seconds: 10,
    });
    let second = answer(&planner, &low, &next, now, 14, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the run rule rejects the prerequisite"
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Block && st.kc == kc_id(4)),
        "another eligible KC is scheduled"
    );
    assert_eq!(second.plan.end, None);
}

#[test]
fn run_limit_is_exactly_the_max_run_boundary() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let review = PlannedReview {
        kc: kc_id(0),
        seconds: 10,
    };

    // One more counted step reaches max_run and is accepted.
    let accepted_plan = run_limit_plan(now, params.max_run - 1, vec![review.clone()]);
    let accepted = start_run_limit(
        &planner,
        &State::new(Profile::default()),
        &accepted_plan,
        now,
    );
    assert_ne!(accepted.plan.end, Some(End::RunLimit));
    assert_ne!(accepted.plan.steps, Vec::<Step>::new());

    // One more step at the boundary would be the fourth and is rejected.
    let rejected_plan = run_limit_plan(now, params.max_run, vec![review]);
    let rejected = start_run_limit(
        &planner,
        &State::new(Profile::default()),
        &rejected_plan,
        now,
    );
    assert_eq!(rejected.plan.end, Some(End::RunLimit));
    assert_eq!(rejected.plan.steps, Vec::<Step>::new());
}

#[test]
fn run_limit_is_a_terminal_reason_without_an_alternative() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let plan = run_limit_plan(
        now,
        params.max_run,
        vec![PlannedReview {
            kc: kc_id(0),
            seconds: 10,
        }],
    );
    let adapted = start_run_limit(&planner, &State::new(Profile::default()), &plan, now);

    assert_eq!(adapted.plan.end, Some(End::RunLimit));
    assert_eq!(adapted.plan.steps, Vec::<Step>::new());
    let last = adapted.issued.last().expect("an ending session event");
    assert_eq!(last.end.as_deref(), Some("run_limit"));
    assert_eq!(End::RunLimit.code(), "run_limit");
}

#[test]
fn a_review_with_an_unseen_item_keeps_a_run_limited_plan_alive() {
    let b = bundle(2, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let plan = run_limit_plan(
        now,
        params.max_run,
        vec![PlannedReview {
            kc: kc_id(1),
            seconds: 10,
        }],
    );
    let adapted = start_run_limit(&planner, &State::new(Profile::default()), &plan, now);

    assert_eq!(adapted.plan.end, None);
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Review && st.kc == kc_id(1))
    );

    let mut ahead = plan.clone();
    ahead.steps = (0..=params.max_run)
        .map(|_| step(&kc_id(0), Phase::Interleaved, true, 10))
        .collect();
    assert!(
        planner
            .validate_plan(&ahead, &State::new(Profile::default()), now)
            .contains(&Violation::RunTooLong),
        "an eligible pending review keeps the run rule active"
    );
}

#[test]
fn another_learning_kc_is_a_valid_alternative_to_a_run_limit() {
    let b = bundle(2, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(now, params.max_run, Vec::new());
    add_learn(&mut plan, &kc_id(1));
    add_production_answer(&b, &mut plan, &kc_id(1));

    let mut s = State::new(Profile::default());
    s.kcs.insert(
        kc_id(0),
        KcState {
            mu: 5.0,
            var: 0.1,
            var_at: now,
            observations: 5,
            card: None,
            tau: 0.0,
            counted: 0,
            residuals: Vec::new(),
            production: Vec::new(),
            ever_mastered: false,
        },
    );
    let adapted = start_run_limit(&planner, &s, &plan, now);

    assert_eq!(adapted.plan.end, None);
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Block && st.kc == kc_id(1))
    );
}

#[test]
fn run_limit_is_cleared_by_a_later_accepted_step_removed_by_starvation() {
    let b = bundle(2, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(now, params.max_run, Vec::new());
    add_learn(&mut plan, &kc_id(1));
    plan.done.push(Done {
        step: step(&kc_id(1), Phase::Block, false, 10),
        shown: None,
        rule: Some(Rule::BlockClosed),
    });

    let mut s = State::new(Profile::default());
    s.kcs.insert(
        kc_id(0),
        KcState {
            mu: 5.0,
            var: 0.1,
            var_at: now,
            observations: 5,
            card: None,
            tau: 0.0,
            counted: 0,
            residuals: Vec::new(),
            production: Vec::new(),
            ever_mastered: false,
        },
    );
    let adapted = start_run_limit(&planner, &s, &plan, now);

    // k0 is run-limited, then k1 is accepted without production and is
    // removed by finish. The accepted k1 step must clear the stale RunLimit.
    assert_eq!(adapted.plan.steps, Vec::<Step>::new());
    assert_eq!(adapted.plan.end, Some(End::ItemsExhausted));
    assert_ne!(adapted.plan.end, Some(End::RunLimit));
    assert_ne!(adapted.plan.end, Some(End::Budget));
}

#[test]
fn a_pending_review_without_an_unseen_item_is_not_an_alternative() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let review = PlannedReview {
        kc: KcId::new(SPELLING_KC).unwrap(),
        seconds: 2_000,
    };
    let mut plan = run_limit_plan(now, params.max_run, vec![review.clone()]);
    add_production_answer(&b, &mut plan, &kc_id(0));
    let state = State::new(Profile::default());
    let adapted = start_run_limit(&planner, &state, &plan, now);

    assert_ne!(adapted.plan.end, Some(End::RunLimit));
    assert_ne!(adapted.plan.end, Some(End::Budget));
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Interleaved && st.kc == kc_id(0))
    );

    let mut ahead = plan.clone();
    ahead.steps = (0..=params.max_run)
        .map(|_| step(&kc_id(0), Phase::Interleaved, true, 10))
        .collect();
    ahead.reviews = vec![review];
    assert!(
        !planner
            .validate_plan(&ahead, &State::new(Profile::default()), now)
            .contains(&Violation::RunTooLong),
        "an unavailable pending review must not switch the run rule on"
    );
}

#[test]
fn a_run_limit_keeps_final_steps_scheduled() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(
        now,
        params.max_run,
        vec![PlannedReview {
            kc: kc_id(0),
            seconds: 10,
        }],
    );
    let item = item_ref(0, 0);
    let projection = b.projection(&item).expect("projection");
    plan.done.push(Done {
        step: step(&kc_id(0), Phase::Block, false, projection.expected_time),
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
    });
    let adapted = start_run_limit(&planner, &State::new(Profile::default()), &plan, now);

    assert_eq!(adapted.plan.end, None);
    assert!(adapted.plan.steps.iter().any(|st| st.phase == Phase::Final));
}

#[test]
fn a_genuine_budget_failure_takes_precedence_over_a_run_limit() {
    let b = bundle(2, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let plan = run_limit_plan(
        now,
        params.max_run,
        vec![PlannedReview {
            kc: kc_id(1),
            seconds: 2_000,
        }],
    );
    let adapted = start_run_limit(&planner, &State::new(Profile::default()), &plan, now);

    assert_eq!(adapted.plan.end, Some(End::Budget));
    assert_eq!(adapted.plan.steps, Vec::<Step>::new());
}

#[test]
fn the_solitary_kc_exception_does_not_create_a_run_limit() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(now, params.max_run, Vec::new());
    add_production_answer(&b, &mut plan, &kc_id(0));
    let adapted = start_run_limit(&planner, &State::new(Profile::default()), &plan, now);

    assert_ne!(adapted.plan.end, Some(End::RunLimit));
    assert_ne!(adapted.plan.end, Some(End::Budget));
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Interleaved && st.kc == kc_id(0))
    );
}

#[test]
fn no_unseen_item_without_a_run_limit_stays_items_exhausted() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(now, 0, Vec::new());
    show_all_items(&b, &mut plan, &kc_id(0));
    let adapted = planner.adapt(
        &State::new(Profile::default()),
        &plan,
        &Outcome::Lesson,
        now,
    );

    assert_eq!(adapted.plan.end, Some(End::ItemsExhausted));
    assert_ne!(adapted.plan.end, Some(End::RunLimit));
}

#[test]
fn a_terminal_run_limit_is_stable_across_repeated_adaptation() {
    let b = bundle(1, 40, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let plan = run_limit_plan(
        now,
        params.max_run,
        vec![PlannedReview {
            kc: kc_id(0),
            seconds: 10,
        }],
    );
    let state = State::new(Profile::default());
    let first = start_run_limit(&planner, &state, &plan, now);
    assert_eq!(first.plan.end, Some(End::RunLimit));

    let second = planner.adapt(&state, &first.plan, &Outcome::Lesson, now);
    assert_eq!(second.plan, first.plan);
    assert_eq!(second.issued, Vec::new());
}

#[test]
fn run_limit_end_serializes_as_its_stable_code() {
    let value = serde_json::to_string(&End::RunLimit).expect("serialize");
    assert_eq!(value, "\"run_limit\"");
    let back: End = serde_json::from_str(&value).expect("deserialize");
    assert_eq!(back, End::RunLimit);

    let now = Timestamp(refl_sim::T0.0);
    let mut plan = run_limit_plan(now, 0, Vec::new());
    plan.end = Some(End::RunLimit);
    let json = serde_json::to_string(&plan).expect("serialize plan");
    let roundtrip: Plan = serde_json::from_str(&json).expect("deserialize plan");
    assert_eq!(roundtrip.end, Some(End::RunLimit));
}

#[test]
fn a_fitting_prerequisite_keeps_an_unfitting_lesson_out() {
    let mut b = bundle(5, 12, &[]);
    slow_kc(&mut b, &kc_id(4), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let (high, low) = prereq_states(&b, now);
    let mut plan = learning_plan(&planner, &high, now, 38, Budget::Ten);

    let first = answer(&planner, &high, &plan, now, 15, false, false);
    plan = first.plan;
    add_learn(&mut plan, &kc_id(4));
    let second = answer(&planner, &low, &plan, now, 16, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert_eq!(
        second.plan.steps.first().map(|st| (st.phase, &st.kc)),
        Some((Phase::Prerequisite, &kc_id(0)))
    );
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Block || st.kc != kc_id(4)),
        "the following slow block is not scheduled"
    );

    let after = answer(&planner, &low, &second.plan, now, 17, true, false);
    assert_eq!(after.rule, None);
    let mut plan = after.plan;
    while plan
        .steps
        .first()
        .is_some_and(|st| st.phase == Phase::Final)
    {
        plan = answer(&planner, &low, &plan, now, 17, true, false).plan;
    }
    assert_eq!(plan.end, Some(End::Budget));
    assert_eq!(plan.steps.len(), 0);
}

#[test]
fn an_ordinary_lesson_that_does_not_fit_ends_budget() {
    let b = bundle(2, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let speed = ReadingSpeed {
        factor: 100.0,
        views: 1,
    };
    let planner = Planner::new(&model, &b, &params, &speed);
    let s = State::new(Profile::default());
    let now = Timestamp(refl_sim::T0.0);
    let session = planner.build_session(
        &s,
        &History::default(),
        now,
        Budget::Ten,
        Lang::Ru,
        Seed::new(41).unwrap(),
    );
    assert_eq!(session.plan.steps.len(), 0);
    assert_eq!(session.plan.end, Some(End::Budget));
}

#[test]
fn review_truncation_keeps_the_planned_order() {
    let mut b = bundle(3, 12, &[]);
    slow_kc(&mut b, &kc_id(0), 100);
    slow_kc(&mut b, &kc_id(1), 250);
    slow_kc(&mut b, &kc_id(2), 250);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = State::new(Profile::default());
    let mut plan = empty_plan(now, Budget::Ten);
    plan.reviews = vec![
        PlannedReview {
            kc: kc_id(0),
            seconds: 100,
        },
        PlannedReview {
            kc: kc_id(1),
            seconds: 250,
        },
        PlannedReview {
            kc: kc_id(2),
            seconds: 250,
        },
    ];
    plan.steps = vec![step(&kc_id(0), Phase::Review, false, 100)];

    let Next::Item(choice) =
        planner.next_item(&s, &plan, &History::default(), now, Seed::new(42).unwrap())
    else {
        panic!("the review has an item");
    };
    let adapted = planner.adapt(
        &s,
        &plan,
        &Outcome::Answer(Answered {
            item: choice.item,
            slot: choice.slot,
            correct: true,
            rapid: false,
            sure: false,
        }),
        now,
    );

    assert_eq!(adapted.rule, None);
    assert_eq!(adapted.plan.steps.len(), 1);
    assert_eq!(adapted.plan.steps[0].phase, Phase::Review);
    assert_eq!(adapted.plan.steps[0].kc, kc_id(1));
    assert_eq!(adapted.plan.end, None);
}

#[test]
fn interleaved_scheduling_keeps_its_planned_step() {
    let b = bundle(2, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = State::new(Profile::default());
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.learn.push(Learn {
        kc: kc_id(0),
        contrasts: Vec::new(),
        lesson: false,
    });
    let production = item_ref(0, 1);
    let production_projection = b.projection(&production).expect("production projection");
    assert!(production_projection.is_production());
    plan.done.push(Done {
        step: step(
            &kc_id(0),
            Phase::Block,
            false,
            production_projection.expected_time,
        ),
        shown: Some(Shown {
            item: production,
            slot: 0,
            family: production_projection.family.clone(),
            kind: production_projection.kind,
            seconds: production_projection.expected_time,
            correct: true,
            rapid: false,
            sure_error: false,
        }),
        rule: Some(Rule::BlockClosed),
    });
    plan.reviews = vec![PlannedReview {
        kc: kc_id(1),
        seconds: 10,
    }];
    plan.steps = vec![step(&kc_id(1), Phase::Review, false, 10)];

    let Next::Item(choice) =
        planner.next_item(&s, &plan, &History::default(), now, Seed::new(43).unwrap())
    else {
        panic!("the review has an item");
    };
    let adapted = planner.adapt(
        &s,
        &plan,
        &Outcome::Answer(Answered {
            item: choice.item,
            slot: choice.slot,
            correct: true,
            rapid: false,
            sure: false,
        }),
        now,
    );

    assert_eq!(adapted.rule, None);
    assert!(
        adapted
            .plan
            .steps
            .iter()
            .any(|st| st.phase == Phase::Interleaved && st.kc == kc_id(0)),
        "the interleaved step follows the review"
    );
}

#[test]
fn a_prerequisite_over_budget_without_a_final_ends_budget() {
    let mut b = bundle(2, 12, &[]);
    slow_kc(&mut b, &kc_id(0), 1_000);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let fast_planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let high = mastered_state(&b, now);
    let mut low = high.clone();
    let k0 = low.kcs.get_mut(&kc_id(0)).expect("k0");
    k0.mu = -3.0;
    k0.var = 1.0;
    k0.var_at = now;
    let plan = learning_plan(&fast_planner, &high, now, 44, Budget::Ten);

    let first = answer(&fast_planner, &high, &plan, now, 18, false, false);

    // Make the final unavailable while keeping the prerequisite candidate
    // over the remaining main budget.
    slow_kc(&mut b, &kc_id(1), 1_000);
    let slow_planner = planner(&model, &b, &params);
    let second = answer(&slow_planner, &low, &first.plan, now, 19, false, false);

    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(0) }));
    assert!(
        second
            .plan
            .steps
            .iter()
            .all(|st| st.phase != Phase::Prerequisite),
        "the over-budget prerequisite is not scheduled"
    );
    assert!(
        second.plan.steps.iter().all(|st| st.phase != Phase::Final),
        "the final is unavailable"
    );
    assert_eq!(second.plan.end, Some(End::Budget));
    assert_eq!(second.plan.steps.len(), 0);
}

fn multi_prerequisite_state(now: Timestamp, values: &[(usize, f64)]) -> State {
    let mut s = State::new(Profile::default());
    for &(index, mu) in values {
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

/// A plan whose learning KC is `k5`, with one previous error so one more
/// non-rapid error fires Rule 3 on its direct prerequisites.
fn multi_prerequisite_plan(b: &Bundle, now: Timestamp) -> Plan {
    let k5 = kc_id(5);
    let item = item_ref(5, 0);
    let projection = b.projection(&item).expect("item projection");
    let mut plan = empty_plan(now, Budget::TwentyFive);
    plan.learn.push(Learn {
        kc: k5.clone(),
        contrasts: Vec::new(),
        lesson: false,
    });
    plan.steps.push(step(&k5, Phase::Block, false, 10));
    plan.done.push(Done {
        step: step(&k5, Phase::Block, false, projection.expected_time),
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
    });
    plan
}

fn selected_prerequisite(plan: &Plan) -> Option<&KcId> {
    plan.steps
        .iter()
        .find(|st| st.phase == Phase::Prerequisite)
        .map(|st| &st.kc)
}

#[test]
fn multiple_prerequisites_rank_by_d_times_importance() {
    // Three direct prerequisites of k5:
    // A = k2: importance 0.9, D = 0.5517168213 -> D * importance = 0.4965451392
    // B = k3: importance 0.6, D = 0.9505285474 -> D * importance = 0.5703171284
    // C = k4: importance 0.75, D = 0.7995458563 -> D * importance = 0.5996593923
    // D-only would choose B; importance-only would choose A; the product
    // chooses C. The synthetic graph has k5 requires k4 and k3; add the
    // direct k5 -> k2 edge to the closure for this fixture.
    let mut b = bundle(6, 12, &[]);
    b.content
        .requires_closure
        .get_mut(&kc_id(5))
        .expect("k5 closure")
        .push(Required {
            kc: kc_id(2),
            distance: 1,
        });
    for entry in &mut b.content.kcs {
        let importance = match entry.kc.id {
            ref id if id == &kc_id(2) => 0.9,
            ref id if id == &kc_id(3) => 0.6,
            ref id if id == &kc_id(4) => 0.75,
            _ => continue,
        };
        entry.kc.importance = UnitInterval::new(importance).expect("unit");
    }
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = multi_prerequisite_state(now, &[(2, 1.37), (3, -0.15), (4, 0.66)]);
    let plan = multi_prerequisite_plan(&b, now);

    let adapted = answer(&planner, &s, &plan, now, 51, false, false);

    assert_eq!(adapted.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(4) }));
    assert_eq!(selected_prerequisite(&adapted.plan), Some(&kc_id(4)));
}

#[test]
fn equal_prerequisite_ranking_breaks_ties_by_lower_id() {
    let mut b = bundle(6, 12, &[]);
    for entry in &mut b.content.kcs {
        if entry.kc.id == kc_id(3) || entry.kc.id == kc_id(4) {
            entry.kc.importance = UnitInterval::new(0.5).expect("unit");
        }
    }
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = multi_prerequisite_state(now, &[(3, -3.0), (4, -3.0)]);
    let plan = multi_prerequisite_plan(&b, now);

    let adapted = answer(&planner, &s, &plan, now, 52, false, false);

    assert_eq!(adapted.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(3) }));
    assert_eq!(selected_prerequisite(&adapted.plan), Some(&kc_id(3)));
}

#[test]
fn an_unseen_top_prerequisite_falls_back_to_the_next_one() {
    let b = bundle(6, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = multi_prerequisite_state(now, &[(3, -3.0), (4, -3.0)]);
    let mut plan = multi_prerequisite_plan(&b, now);
    show_all_items(&b, &mut plan, &kc_id(4));

    let adapted = answer(&planner, &s, &plan, now, 53, false, false);

    assert_eq!(adapted.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(3) }));
    assert_eq!(selected_prerequisite(&adapted.plan), Some(&kc_id(3)));
}

#[test]
fn a_deferred_prerequisite_is_not_selected() {
    let b = bundle(6, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    let s = multi_prerequisite_state(now, &[(3, -3.0), (4, -3.0)]);
    let mut plan = multi_prerequisite_plan(&b, now);
    plan.done.push(Done {
        step: step(&kc_id(4), Phase::Block, false, 10),
        shown: None,
        rule: Some(Rule::KcDeferred),
    });

    let adapted = answer(&planner, &s, &plan, now, 54, false, false);

    assert_eq!(adapted.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(3) }));
    assert_eq!(selected_prerequisite(&adapted.plan), Some(&kc_id(3)));
}

#[test]
fn repeated_wrong_prerequisite_checks_descend_without_duplicates() {
    let b = bundle(6, 12, &[]);
    let model = KnowledgeModel::default();
    let params = Params::default();
    let planner = planner(&model, &b, &params);
    let now = Timestamp(refl_sim::T0.0);
    // k3 is ranked above k4; k2 is the direct prerequisite k3 must descend
    // to after the prerequisite answer is wrong.
    let s = multi_prerequisite_state(now, &[(2, -3.0), (3, -3.0), (4, 1.0)]);
    let plan = multi_prerequisite_plan(&b, now);

    let first = answer(&planner, &s, &plan, now, 55, false, false);
    assert_eq!(first.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(3) }));
    assert_eq!(selected_prerequisite(&first.plan), Some(&kc_id(3)));
    assert_eq!(
        first
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Prerequisite)
            .count(),
        1
    );

    let second = answer(&planner, &s, &first.plan, now, 56, false, false);
    assert_eq!(second.rule, Some(Rule::PrerequisiteCheck { kc: kc_id(2) }));
    assert_eq!(selected_prerequisite(&second.plan), Some(&kc_id(2)));
    assert_eq!(
        second
            .plan
            .steps
            .iter()
            .filter(|st| st.phase == Phase::Prerequisite)
            .count(),
        1,
        "only one pending prerequisite step"
    );
}

#[test]
fn prerequisite_scenarios_expose_rule3_for_native_parity() {
    let a = refl_sim::prereq::scenario_a().expect("scenario A");
    assert_eq!(a.rule, "prerequisite_check");
    assert_eq!(a.selected, Some(refl_sim::prereq::expected_selected()));
    assert_eq!(a.trigger.as_deref(), Some("prerequisite_check"));
    assert_eq!(a.prerequisite_steps, 1);
    assert!(a.subsequent_steps > 0);
    assert_eq!(a.final_steps, 2);
    assert!(a.continued);
    assert!(a.post_prerequisite_steps > 0);
    assert_eq!(a.post_prerequisite_prereq_steps, 0);

    let b = refl_sim::prereq::scenario_b().expect("scenario B");
    assert_eq!(b.rule, "prerequisite_check");
    assert_eq!(b.selected, Some(refl_sim::prereq::expected_selected()));
    assert_eq!(b.trigger.as_deref(), Some("prerequisite_check"));
    assert_eq!(b.prerequisite_steps, 0);
    assert!(b.final_steps > 0);
    assert_eq!(
        b.end,
        refl_sim::prereq::expected_b_end().map(|end| end.code().to_string())
    );
}

#[test]
fn return_check_is_scheduled_first_after_a_break() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 1,
        items_per_kc: 8,
    });
    let now = Timestamp(refl_sim::T0.0 + 20 * DAY_MS);
    let mut s = mastered_state(&b, Timestamp(refl_sim::T0.0 + DAY_MS));
    s.last_answer.insert(
        refl_core::ids::SubjectId::new(SUBJECT).unwrap(),
        Timestamp(now.0 - 15 * DAY_MS),
    );
    s.mastered_at
        .insert(kc_id(0), Timestamp(now.0 - 15 * DAY_MS));
    let model = KnowledgeModel::default();
    let params = Params::default();
    let speed = ReadingSpeed::default();
    let p = planner(&model, &b, &params);
    let session = p.build_session(
        &s,
        &History::default(),
        now,
        Budget::TwentyFive,
        Lang::Ru,
        Seed::new(4).unwrap(),
    );
    assert!(session.plan.return_check);
    assert_eq!(
        session.plan.steps.first().map(|step| step.phase),
        Some(Phase::ReturnCheck)
    );
    assert!(
        session
            .plan
            .steps
            .iter()
            .any(|step| step.phase == Phase::ReturnCheck)
    );
    let _ = speed;
}
