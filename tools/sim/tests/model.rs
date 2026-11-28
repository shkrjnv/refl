//! Behaviour of learner model v1 on a small synthetic bundle: three grammar
//! KCs in a chain (k02 requires k01 requires k00), ranks A1, A2, B1; items
//! alternate choice (even) and gap fill (odd); expected time 8 + 2·(j % 4) s.

use refl_core::bundle::Bundle;
use refl_core::event::{
    AnswerContext, AnswerEvent, Confidence, Decoded, EventPayload, ReclassifiedEvent, Response,
    Verdict,
};
use refl_core::ids::{ContentVersion, ItemVersion, KcId};
use refl_core::item::AuthorDifficulty;
use refl_core::model::derived::{KcProgress, MisconceptionStatus};
use refl_core::model::{
    Applied, Families, FoldCtx, KcState, KnowledgeModel, LearnerModel, NeedsReplay, Profile, State,
    replay,
};
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use refl_core::value::Timestamp;
use refl_sim::{
    BundleSpec, DAY_MS, LogSpec, SPELLING_KC, T0, answer, event, event_in, explanation_id,
    item_ref, kc_id, misconception_id, session, synthetic_bundle, synthetic_log, unknown_event,
};

fn bundle() -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 3,
        items_per_kc: 6,
    })
}

fn at(day: i64, ms: i64) -> Timestamp {
    Timestamp(T0.0 + day * DAY_MS + ms)
}

fn spelling() -> KcId {
    KcId::new(SPELLING_KC).unwrap()
}

/// Gap fill `j` (odd) of KC `i`, answered correctly with `text`.
fn typed(i: usize, j: usize, text: &str) -> AnswerEvent {
    answer(
        item_ref(i, j),
        Verdict::Correct,
        Response::Text(text.into()),
        8_000,
    )
}

/// Choice `j` (even) of KC `i`, correct.
fn chosen(i: usize, j: usize) -> AnswerEvent {
    answer(item_ref(i, j), Verdict::Correct, Response::Option(0), 8_000)
}

fn ev(t: Timestamp, tail: u64, a: AnswerEvent) -> Decoded {
    event(t, tail, EventPayload::Answer(a))
}

fn fold(bundle: &Bundle, events: &[Decoded]) -> State {
    replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        bundle,
        events,
    )
}

fn mu(s: &State, kc: &KcId) -> f64 {
    s.kcs[kc].mu
}

/// Starting μ with L = 0: `clamp(1.2 · (0 − rank), ±3)`.
fn prior_mu(rank: f64) -> f64 {
    (-1.2 * rank).max(-3.0)
}

#[test]
fn empty_log_is_the_initial_state() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let profile = Profile::default();
    let state = replay(&model, &profile, &b, &[]);
    assert_eq!(state, State::new(profile));
    assert_eq!(state.position, None);
    assert_eq!(state.kcs.len(), 0);
    assert_eq!(state.misconceptions.len(), 0);
    let back: State = serde_json::from_slice(&state.canonical_bytes()).expect("state round-trips");
    assert_eq!(back, state);
}

#[test]
fn correct_and_incorrect_answers_move_mu_in_opposite_directions() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let profile = Profile::default();
    let prior = model.prior(&profile, &b, &kc_id(0)).expect("prior").0;

    let correct = fold(&b, &[ev(at(0, 0), 1, chosen(0, 0))]);
    let mut wrong_answer = chosen(0, 0);
    wrong_answer.verdict = Verdict::Wrong;
    wrong_answer.response = Response::Option(1);
    let wrong = fold(&b, &[ev(at(0, 0), 1, wrong_answer)]);

    assert!(mu(&correct, &kc_id(0)) > prior);
    assert!(mu(&wrong, &kc_id(0)) < prior);
    assert!(mu(&correct, &kc_id(0)) > mu(&wrong, &kc_id(0)));
    assert!(correct.kcs[&kc_id(0)].var < 2.25);
    assert!(wrong.kcs[&kc_id(0)].var < 2.25);
}

#[test]
fn a_secondary_kc_moves_less_than_the_primary() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let profile = Profile::default();
    let prior0 = model.prior(&profile, &b, &kc_id(0)).expect("prior").0;
    let prior1 = model.prior(&profile, &b, &kc_id(1)).expect("prior").0;
    let projection = b.projection(&item_ref(1, 4)).expect("projection");
    assert_eq!(projection.slots[0].kcs.len(), 2);
    assert_eq!(
        projection.slots[0].kcs[1].role,
        refl_core::item::KcRole::Secondary
    );

    let state = fold(&b, &[ev(at(0, 0), 1, chosen(1, 4))]);
    let primary = mu(&state, &kc_id(1)) - prior1;
    let secondary = mu(&state, &kc_id(0)) - prior0;
    assert!(
        primary > secondary && secondary > 0.0,
        "{primary} {secondary}"
    );
}

#[test]
fn correct_typed_answer_raises_spelling() {
    let b = bundle();
    let s = fold(&b, &[ev(at(0, 0), 1, typed(0, 1, "the"))]);
    let k = &s.kcs[&spelling()];
    assert!(k.mu > prior_mu(1.0), "{}", k.mu);
    assert!(k.var < 2.25);
    assert_eq!(k.observations, 1);
    // Never a slot KC: no memory, no fluency.
    assert!(k.card.is_none());
    assert_eq!(k.counted, 0);
}

#[test]
fn choice_answer_does_not_touch_spelling() {
    let b = bundle();
    let s = fold(&b, &[ev(at(0, 0), 1, chosen(0, 0))]);
    assert!(!s.kcs.contains_key(&spelling()));
    assert!(s.kcs.contains_key(&kc_id(0)));
}

#[test]
fn spelling_success_needs_a_letter_only_answer() {
    let b = bundle();
    for text in ["it's", "well-known", "b2", "  ...  "] {
        let s = fold(&b, &[ev(at(0, 0), 1, typed(0, 1, text))]);
        assert!(!s.kcs.contains_key(&spelling()), "{text}");
    }
}

#[test]
fn spelling_success_is_weaker_than_a_typo_failure() {
    let b = bundle();
    let ok = fold(&b, &[ev(at(0, 0), 1, typed(0, 1, "the"))]);
    let mut a = typed(0, 1, "teh");
    a.verdict = Verdict::Typo;
    let typo = fold(&b, &[ev(at(0, 0), 1, a)]);
    let up = mu(&ok, &spelling()) - prior_mu(1.0);
    let down = prior_mu(1.0) - mu(&typo, &spelling());
    assert!(up > 0.0 && down > 0.0);
    // Weight 0.3 against 1 on the same prior.
    assert!(ok.kcs[&spelling()].var > typo.kcs[&spelling()].var);
    // The typo still counts as correct for the target KC.
    assert!(mu(&typo, &kc_id(0)) > prior_mu(1.0));
}

#[test]
fn spelling_success_takes_hint_and_attempt_multipliers() {
    let b = bundle();
    let plain = fold(&b, &[ev(at(0, 0), 1, typed(0, 1, "the"))]);
    let mut a = typed(0, 1, "the");
    a.hints = 1;
    a.attempt = 2;
    let helped = fold(&b, &[ev(at(0, 0), 1, a)]);
    let gain = |s: &State| mu(s, &spelling()) - prior_mu(1.0);
    assert!(gain(&helped) < gain(&plain) && gain(&helped) > 0.0);
}

#[test]
fn unrecognized_answer_gives_no_spelling_observation() {
    let b = bundle();
    let mut a = typed(0, 1, "the");
    a.verdict = Verdict::Unrecognized;
    let s = fold(&b, &[ev(at(0, 0), 1, a)]);
    assert!(!s.kcs.contains_key(&spelling()));
    assert!(mu(&s, &kc_id(0)) < prior_mu(1.0));
}

#[test]
fn rapid_guess_changes_nothing_but_the_counters() {
    let b = bundle();
    // Choice, expected 8 s: 1 s is under a quarter.
    let mut a = chosen(0, 0);
    a.latency_ms = 1_000;
    let s = fold(&b, &[ev(at(0, 0), 1, a.clone())]);
    assert!(s.kcs.is_empty() && s.misconceptions.is_empty());
    assert_eq!((s.rapid.total, s.rapid.streak), (1, 1));
    // Typed, expected 10 s: 2.4 s is under a quarter but not under 2 s.
    let mut t = typed(0, 1, "the");
    t.latency_ms = 2_400;
    let s = fold(&b, &[ev(at(0, 0), 1, a), ev(at(0, 10_000), 2, t)]);
    assert!(s.kcs.contains_key(&kc_id(0)));
    assert_eq!((s.rapid.total, s.rapid.streak), (1, 0));
}

#[test]
fn unknown_event_marks_the_state_incomplete_and_changes_nothing_else() {
    let b = bundle();
    let base = [ev(at(0, 0), 1, chosen(0, 0))];
    let plain = fold(&b, &base);
    let with_unknown = fold(&b, &[base[0].clone(), unknown_event(at(0, 5_000), 2)]);
    assert!(!plain.incomplete && with_unknown.incomplete);
    assert_eq!(plain.kcs, with_unknown.kcs);
    let model = KnowledgeModel::default();
    let mut s = model.init(&Profile::default());
    let families = Families::of(&b);
    let ctx = FoldCtx::new(&b, &families);
    let applied = model.apply(&mut s, &unknown_event(at(0, 0), 3), &ctx);
    assert_eq!(applied, Ok(Applied::Unknown));
}

#[test]
fn lesson_success_starts_memory_only_after_the_explanation() {
    let b = bundle();
    let mut lesson = chosen(0, 0);
    lesson.context = AnswerContext::Lesson;
    let s = fold(&b, &[ev(at(0, 0), 1, lesson.clone())]);
    assert!(s.kcs[&kc_id(0)].card.is_none());
    let viewed = event(
        at(0, 1_000),
        2,
        EventPayload::ExplanationViewed {
            explanation: explanation_id(0),
            depth: refl_core::content::ExplanationDepth::Short,
            seconds: 30,
        },
    );
    let s = fold(&b, &[viewed, ev(at(0, 60_000), 3, lesson)]);
    assert!(s.kcs[&kc_id(0)].card.is_some());
    // Outside a lesson the first success starts it without an explanation.
    let mut review = chosen(0, 0);
    review.context = AnswerContext::Review;
    let s = fold(&b, &[ev(at(0, 0), 1, review)]);
    assert!(s.kcs[&kc_id(0)].card.is_some());
}

#[test]
fn fsrs_counts_the_first_answer_per_kc_per_day() {
    let b = bundle();
    let events = [
        ev(at(0, 0), 1, chosen(0, 0)),
        ev(at(0, 60_000), 2, chosen(0, 2)),
        ev(at(0, 120_000), 3, chosen(0, 4)),
    ];
    let s = fold(&b, &events);
    let card = s.kcs[&kc_id(0)].card.unwrap();
    assert_eq!(card.last_review, at(0, 0));
    let mut next = events.to_vec();
    next.push(ev(at(1, 0), 4, chosen(0, 0)));
    let s = fold(&b, &next);
    let later = s.kcs[&kc_id(0)].card.unwrap();
    assert_eq!(later.last_review, at(1, 0));
    assert!(later.stability > card.stability);
}

#[test]
fn success_propagates_down_requires_and_errors_do_not() {
    let b = bundle();
    let s = fold(&b, &[ev(at(0, 0), 1, chosen(2, 0))]);
    let d1 = mu(&s, &kc_id(1)) - prior_mu(2.0);
    let d2 = mu(&s, &kc_id(0)) - prior_mu(1.0);
    assert!(d1 > d2 && d2 > 0.0, "{d1} {d2}");
    // Pseudo-observations are not answers.
    assert_eq!(s.kcs[&kc_id(1)].observations, 0);
    let mut wrong = chosen(2, 0);
    wrong.verdict = Verdict::Wrong;
    wrong.response = Response::Option(1);
    let s = fold(&b, &[ev(at(0, 0), 1, wrong)]);
    assert!(!s.kcs.contains_key(&kc_id(1)) && !s.kcs.contains_key(&kc_id(0)));
}

#[test]
fn misconceptions_count_shown_and_resisted() {
    let b = bundle();
    let mut wrong = chosen(0, 0);
    wrong.verdict = Verdict::Wrong;
    wrong.response = Response::Option(1);
    wrong.misconception = Some(misconception_id(0, 0));
    wrong.confidence = Some(Confidence::Sure);
    let s = fold(&b, &[ev(at(0, 0), 1, wrong)]);
    let m = &s.misconceptions[&misconception_id(0, 0)];
    assert!((m.alpha - m.alpha0 - 1.5).abs() < 1e-12);
    assert_eq!(m.manifested, 1);
    // A correct answer where both were possible: β += 1 each.
    let s = fold(&b, &[ev(at(0, 0), 1, chosen(0, 0))]);
    for x in 0..2 {
        let m = &s.misconceptions[&misconception_id(0, x)];
        assert!((m.beta - m.beta0 - 1.0).abs() < 1e-12);
    }
    // Prior with unknown L1: the mean of uz and ru, weight n₀ = 2.
    let m = &s.misconceptions[&misconception_id(0, 0)];
    assert!((m.alpha0 - 2.0 * (0.1 + 0.05) / 2.0).abs() < 1e-12);
}

#[test]
fn misconception_turns_active_and_decays() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let events: Vec<Decoded> = (0..3)
        .map(|n| {
            let mut a = chosen(0, 0);
            a.verdict = Verdict::Wrong;
            a.response = Response::Option(1);
            a.misconception = Some(misconception_id(0, 0));
            ev(at(0, n * 60_000), u64::try_from(n).unwrap(), a)
        })
        .collect();
    let s = fold(&b, &events);
    let id = misconception_id(0, 0);
    let now = model.misconception(&s, &id, at(0, 0)).unwrap();
    assert_eq!(now.status, MisconceptionStatus::Active);
    let later = model.misconception(&s, &id, at(70, 0)).unwrap();
    assert!(later.strength < now.strength);
}

#[test]
fn misconception_cures_after_enough_resisted_answers() {
    let b = bundle();
    let id = misconception_id(0, 0);
    let mut events = Vec::new();
    for n in 0..2u64 {
        let mut wrong = chosen(0, 0);
        wrong.verdict = Verdict::Wrong;
        wrong.response = Response::Option(1);
        wrong.misconception = Some(id.clone());
        events.push(ev(at(0, i64::try_from(n).expect("small")), n, wrong));
    }
    for n in 0..12u64 {
        events.push(ev(
            at(0, i64::try_from(n).expect("small") + 2),
            n + 2,
            chosen(0, 0),
        ));
    }
    let s = fold(&b, &events);
    let view = KnowledgeModel::default()
        .misconception(&s, &id, at(0, 20))
        .expect("touched");
    assert_eq!(view.manifested, 2);
    assert_eq!(view.status, MisconceptionStatus::Cured);
}

#[test]
fn the_fold_reports_the_prediction_made_at_display_time() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let mut s = model.init(&Profile::default());
    let families = Families::of(&b);
    let ctx = FoldCtx::new(&b, &families);
    let first = ev(at(0, 0), 1, chosen(0, 0));
    model.apply(&mut s, &first, &ctx).unwrap();
    let a = typed(0, 1, "the");
    let shown = Timestamp(at(5, 0).0 - i64::from(a.latency_ms));
    let expected = model.predict(&s, &b, &a.item, 0, shown).unwrap();
    let Ok(Applied::Answer(o)) = model.apply(&mut s, &ev(at(5, 0), 2, a), &ctx) else {
        panic!("an answer");
    };
    assert!((o.predicted - expected).abs() < f64::EPSILON);
    assert!(o.correct && !o.rapid);
}

#[test]
fn reclassified_answer_is_refolded_with_the_new_verdict() {
    let b = bundle();
    let mut wrong = typed(0, 1, "a");
    wrong.verdict = Verdict::Wrong;
    let original = ev(at(0, 0), 1, wrong);
    let Decoded::Known(o) = &original else {
        unreachable!()
    };
    let fix = event(
        at(3, 0),
        2,
        EventPayload::Reclassified(ReclassifiedEvent {
            original: o.event_id,
            verdict: Verdict::Correct,
            misconception: None,
            content_version: ContentVersion("synthetic".into()),
        }),
    );
    let corrected = fold(&b, &[original.clone(), fix.clone()]);
    let as_if_correct = fold(&b, &[ev(at(0, 0), 1, typed(0, 1, "a"))]);
    assert_eq!(corrected.kcs, as_if_correct.kcs);

    // Folded onto a state that already holds the wrong verdict: replay.
    let model = KnowledgeModel::default();
    let mut s = model.init(&Profile::default());
    let families = Families::of(&b);
    let ctx = FoldCtx::new(&b, &families);
    model.apply(&mut s, &original, &ctx).unwrap();
    assert_eq!(
        model.apply(&mut s, &fix, &ctx),
        Err(NeedsReplay::Reclassified)
    );
}

#[test]
fn late_and_duplicate_events() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let mut s = model.init(&Profile::default());
    let families = Families::of(&b);
    let ctx = FoldCtx::new(&b, &families);
    let later = ev(at(1, 0), 2, chosen(0, 0));
    model.apply(&mut s, &later, &ctx).unwrap();
    let before = s.clone();
    assert_eq!(model.apply(&mut s, &later, &ctx), Ok(Applied::Duplicate));
    let earlier = ev(at(0, 0), 1, chosen(0, 2));
    assert_eq!(model.apply(&mut s, &earlier, &ctx), Err(NeedsReplay::Late));
    assert_eq!(s, before);
}

#[test]
fn answers_outside_the_bundle_are_counted_not_guessed() {
    let b = bundle();
    let mut a = chosen(0, 0);
    a.item.version = refl_core::ids::ItemVersion(9);
    let s = fold(&b, &[ev(at(0, 0), 1, a)]);
    assert_eq!(s.unresolved, 1);
    assert!(s.kcs.is_empty());
}

/// Fast, sure, correct typed answers on k00, two a day in a session of their
/// own (the first of a session does not count for fluency): mastered, then
/// forgotten after a long break. Fifteen gap fills, so no item comes back
/// within the 3 days fluency asks for.
#[test]
fn mastery_statuses_and_the_return_check() {
    let b = synthetic_bundle(BundleSpec {
        kcs: 3,
        items_per_kc: 30,
    });
    let model = KnowledgeModel::default();
    let subject = refl_core::ids::SubjectId::new("en").unwrap();
    let events: Vec<Decoded> = (0..60_i64)
        .map(|n| {
            let (day, nth) = (n / 2, n % 2);
            let j = 1 + 2 * usize::try_from(n % 15).unwrap();
            let mut a = typed(0, j, "the");
            a.context = AnswerContext::Review;
            a.confidence = Some(Confidence::Sure);
            a.latency_ms = 5_000;
            a.first_input_ms = Some(5_000);
            event_in(
                Some(session(u128::try_from(day).unwrap() + 1)),
                at(day, (nth - 1) * 60_000), // the day's last answer is on the hour
                u64::try_from(n).unwrap(),
                EventPayload::Answer(a),
            )
        })
        .collect();
    let s = fold(&b, &events);
    let k: &KcState = &s.kcs[&kc_id(0)];
    assert!(k.ever_mastered);
    assert!(s.mastered_at.contains_key(&kc_id(0)));
    assert!(k.production.len() == 2 && k.tau > 0.0);
    let last = at(29, 0);
    let progress = model.progress(&s, &b, &kc_id(0), last);
    assert!(
        matches!(progress, KcProgress::Mastered | KcProgress::LongTerm),
        "{progress:?}"
    );
    assert_eq!(
        model.progress(&s, &b, &kc_id(2), last),
        KcProgress::NotStarted
    );
    assert_eq!(
        model.progress(&s, &b, &kc_id(0), at(29 + 3_650, 0)),
        KcProgress::Forgetting
    );

    let check = |days: i64| {
        model
            .return_check(&s, &b, &subject, at(29 + days, 0))
            .unwrap()
    };
    assert!(!check(1).due);
    assert!(!check(13).due || check(13).mean_recall.unwrap() < 0.85);
    assert!(check(14).due);
    assert!(check(3_650).forgetting_share.unwrap() > 0.2);
}

#[test]
fn return_check_needs_a_week_when_nothing_is_mastered() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let subject = refl_core::ids::SubjectId::new("en").unwrap();
    let s = fold(&b, &[ev(at(0, 0), 1, chosen(0, 0))]);
    let check = |days| model.return_check(&s, &b, &subject, at(days, 0)).unwrap();
    assert!(check(8).mean_recall.is_none());
    assert!(!check(13).due);
    assert!(check(14).due);
}

#[test]
fn level_counts_empty_lower_levels_as_covered() {
    let b = bundle();
    let model = KnowledgeModel::default();
    let subject = refl_core::ids::SubjectId::new("en").unwrap();
    let grammar = refl_core::ids::DomainId::new("grammar").unwrap();
    // Grammar KCs sit at A1, A2, B1; pre-A1 is empty and below them.
    let mut s = model.init(&Profile::default());
    for i in 0..3 {
        s.kcs.insert(
            kc_id(i),
            KcState {
                mu: 10.0,
                var: 0.01,
                var_at: at(0, 0),
                observations: 1,
                card: None,
                tau: 0.0,
                counted: 1,
                residuals: vec![0.0],
                production: Vec::new(),
                ever_mastered: true,
            },
        );
    }
    let l = model.level(&s, &b, &subject, &grammar, at(0, 0)).unwrap();
    assert!((l.point - 3.0).abs() < 1e-9, "{l:?}");
    assert!((l.low - 3.0).abs() < 1e-9 && (l.high - 3.0).abs() < 1e-9);

    let fresh = model.init(&Profile::default());
    let a = model
        .level(&fresh, &b, &subject, &grammar, at(0, 0))
        .unwrap();
    let again = model
        .level(&fresh, &b, &subject, &grammar, at(0, 0))
        .unwrap();
    assert_eq!(a, again, "fixed seed: deterministic");
    // The 80% interval need not hold the mean: with nothing known most
    // samples are exactly 0.
    assert!(a.low <= a.high, "{a:?}");
    assert!(a.point > 0.0 && a.point < 0.5);
}

#[test]
fn profile_is_part_of_the_state_hash() {
    let b = bundle();
    let events = [ev(at(0, 0), 1, chosen(0, 0))];
    let model = KnowledgeModel::default();
    let zero = replay(&model, &Profile::default(), &b, &events);
    let profile = Profile {
        l1: Some(refl_core::model::NativeLanguage::Uz),
        ..Profile::default()
    };
    let uz = replay(&model, &profile, &b, &events);
    assert_ne!(zero.hash(), uz.hash());
}

/// What content-build does on a release that bumps every item: version 2 is
/// the current body with another difficulty, and the lock keeps the
/// projections of version 1.
fn bumped(b: &Bundle) -> Bundle {
    let mut out = b.clone();
    for e in &mut out.content.items {
        e.item.version = ItemVersion(2);
        e.item.difficulty = AuthorDifficulty::new(e.item.difficulty.get() % 5 + 1).unwrap();
    }
    let current: Vec<_> = out
        .content
        .items
        .iter()
        .map(|e| ItemVersionEntry {
            item: e.item.id.clone(),
            version: e.item.version,
            projection: ItemProjection::of(&e.item),
        })
        .collect();
    out.content.item_versions.extend(current);
    out.content
        .item_versions
        .sort_by(|a, b| (&a.item, a.version).cmp(&(&b.item, b.version)));
    out.content_version = ContentVersion("bumped".into());
    out
}

#[test]
fn old_log_folds_to_the_same_state_after_a_version_bump() {
    let old = bundle();
    let log = synthetic_log(
        &old,
        LogSpec {
            events: 400,
            seed: 11,
        },
    );
    let model = KnowledgeModel::default();
    let before = replay(&model, &Profile::default(), &old, &log);
    assert_eq!(before.unresolved, 0);
    assert!(before.kcs.len() > 1, "the log answers items");

    let new = bumped(&old);
    let after = replay(&model, &Profile::default(), &new, &log);
    assert_eq!(after.unresolved, 0);
    assert_eq!(before.hash(), after.hash());

    // Without the old projections the answers would be lost, not guessed.
    let mut lost = new;
    lost.content.item_versions.retain(|e| e.version.0 == 2);
    let broken = replay(&model, &Profile::default(), &lost, &log);
    assert!(broken.unresolved > 0);
    assert_ne!(before.hash(), broken.hash());
}

fn mark(t: Timestamp, tail: u64, kc: &KcId) -> Decoded {
    use refl_core::event::{MentorOverrideEvent, OverrideKind, OverrideReason};
    event(
        t,
        tail,
        EventPayload::MentorOverride(MentorOverrideEvent {
            mentor: refl_core::ids::UserId(uuid::Uuid::nil()),
            kind: OverrideKind::MarkKnown {
                kc: kc.clone(),
                model_mastery: refl_core::value::UnitInterval::new(0.1).unwrap(),
            },
            reason: OverrideReason::KnowsElsewhere,
            note: None,
        }),
    )
}

/// docs/03, «Уточнения версии 1»: one observation, y = 1, b = 0, g = 0,
/// weight 0.3, and nothing but μ and σ.
#[test]
fn a_mentor_mark_is_one_weak_observation_of_mu_and_sigma() {
    let b = bundle();
    let s = fold(&b, &[mark(at(0, 0), 1, &kc_id(0))]);
    let k = &s.kcs[&kc_id(0)];
    // Hand-computed: μ₀ = −1.2, σ² = 2.25, slope 0.7286, P* = 0.2943,
    // p = 0.2796, score = 0.5141, I = 0.1026, w = 0.3.
    assert!((k.mu - -0.8754).abs() < 2e-3, "{}", k.mu);
    assert!((k.var - 2.1043).abs() < 2e-3, "{}", k.var);
    assert!(k.mu > prior_mu(1.0) && k.var < 2.25);
    assert_eq!(k.observations, 1);
    // No memory, no fluency, no production credit.
    assert!(k.card.is_none());
    assert_eq!((k.tau, k.counted), (0.0, 0));
    assert!(k.production.is_empty() && !k.ever_mastered);
    // No misconceptions, no spelling, no counters of answers.
    assert!(s.misconceptions.is_empty());
    assert_eq!(s.kcs.len(), 1);
    assert_eq!((s.rapid.total, s.unresolved), (0, 0));
}

#[test]
fn a_mentor_mark_does_not_reach_prerequisites() {
    let b = bundle();
    let s = fold(&b, &[mark(at(0, 0), 1, &kc_id(2))]);
    assert_eq!(s.kcs.keys().collect::<Vec<_>>(), [&kc_id(2)]);
}

#[test]
fn two_marks_are_two_observations() {
    let b = bundle();
    let one = fold(&b, &[mark(at(0, 0), 1, &kc_id(0))]);
    let two = fold(
        &b,
        &[
            mark(at(0, 0), 1, &kc_id(0)),
            mark(at(0, 1_000), 2, &kc_id(0)),
        ],
    );
    let (a, c) = (&one.kcs[&kc_id(0)], &two.kcs[&kc_id(0)]);
    assert_eq!(c.observations, 2);
    assert!(c.mu > a.mu && c.var < a.var);
}

#[test]
fn marks_alone_never_master_a_kc() {
    let b = bundle();
    let marks: Vec<Decoded> = (0..200)
        .map(|n| mark(at(0, n), u64::try_from(n).unwrap(), &kc_id(0)))
        .collect();
    let s = fold(&b, &marks);
    let model = KnowledgeModel::default();
    let k = &s.kcs[&kc_id(0)];
    // Belief is far above the threshold, yet production and fluency are missing.
    assert!(k.mu > 1.5);
    assert!(!k.ever_mastered && !model.is_mastered(&s, &b, &kc_id(0), at(0, 1_000)));
    assert_eq!(
        model.progress(&s, &b, &kc_id(0), at(0, 1_000)),
        KcProgress::Learning
    );
}

#[test]
fn a_mark_on_an_unknown_kc_changes_nothing() {
    let b = bundle();
    let ghost = KcId::new("en.gram.ghost").unwrap();
    let s = fold(&b, &[mark(at(0, 0), 1, &ghost)]);
    assert!(s.kcs.is_empty());
}

#[test]
fn process_noise_widens_the_belief_before_the_update_and_changes_the_version() {
    let b = bundle();
    let v1 = KnowledgeModel {
        params: refl_core::model::Params {
            process_noise: 0.0,
            ..KnowledgeModel::default().params
        },
    };
    let v11 = KnowledgeModel::default();
    assert!(v11.version().starts_with("knowledge-v1.3+"));
    assert_ne!(v1.version(), v11.version());
    let first = ev(at(0, 0), 1, chosen(0, 0));
    let shown = Timestamp(at(0, 0).0 - 8_000);
    let run = |m: &KnowledgeModel| {
        let mut s = m.init(&Profile::default());
        let before = m
            .predict(&s, &b, &item_ref(0, 0), 0, shown)
            .expect("in the bundle");
        let families = Families::of(&b);
        let applied = m
            .apply(&mut s, &first, &FoldCtx::new(&b, &families))
            .unwrap();
        (s, before, applied)
    };
    let ((s1, _, _), (s11, before, applied)) = (run(&v1), run(&v11));
    // The fold's prediction is `predict`'s, q included, and the update then
    // starts from the same σ² (prior + q), so the new σ² is below it.
    let Applied::Answer(outcome) = applied else {
        panic!("an answer");
    };
    assert_eq!(outcome.predicted.to_bits(), before.to_bits());
    let kc = kc_id(0);
    assert!(s11.kcs[&kc].var > s1.kcs[&kc].var);
    // A wider belief predicts closer to the middle than the same belief.
    let plain = v1.predict(&v1.init(&Profile::default()), &b, &item_ref(0, 0), 0, shown);
    assert_ne!(plain, Some(before));
}
