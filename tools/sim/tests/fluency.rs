//! Fluency of learner model v1.3 (`docs/03-layer2-learner-model.md`,
//! «Беглость: время извлечения») on constructed answers to the synthetic
//! bundle: three KCs, items alternate choice (even) and gap fill (odd), one
//! instruction for all. The simulator's own times are not used: they come
//! from the same expected time as βᶠ, so they could not show much.

use refl_core::bundle::Bundle;
use refl_core::event::{AnswerEvent, Decoded, EventPayload, InputDevice, Response, Verdict};
use refl_core::ids::{InstructionId, KcId};
use refl_core::model::{Families, KnowledgeModel, Profile, State, replay};
use refl_core::value::Timestamp;
use refl_core::{decode_event, encode_event};
use refl_sim::{BundleSpec, DAY_MS, T0, answer, event, item_ref, kc_id, synthetic_bundle};

const LAMBDA: f64 = 0.2;

fn bundle(items_per_kc: usize) -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 3,
        items_per_kc,
    })
}

/// `minutes` after the start of day `day`.
fn at(day: i64, minutes: i64) -> Timestamp {
    Timestamp(T0.0 + day * DAY_MS + minutes * 60_000)
}

/// βᶠ of the family of item `j` of KC `i`.
fn beta(b: &Bundle, i: usize, j: usize) -> f64 {
    let family = &b.projection(&item_ref(i, j)).expect("in the bundle").family;
    Families::of(b).get(family).expect("a family").beta
}

/// A correct answer to item `j` of KC `i` (choice for even `j`, typed for
/// odd) whose first input comes after `first_ms`; typing takes no time.
fn correct(i: usize, j: usize, first_ms: u32) -> AnswerEvent {
    let choice = j.is_multiple_of(2);
    let response = if choice {
        Response::Option(0)
    } else {
        Response::Text("the".into())
    };
    let mut a = answer(item_ref(i, j), Verdict::Correct, response, first_ms + 500);
    a.first_input_ms = Some(first_ms);
    if choice {
        a.typing_ms = None;
        a.chars_typed = None;
        a.edits = None;
    }
    a
}

/// A correct answer to item `j` of KC `i` retrieved in `factor × exp(βᶠ)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // seconds
fn paced(b: &Bundle, i: usize, j: usize, factor: f64) -> AnswerEvent {
    correct(
        i,
        j,
        (factor * libm::exp(beta(b, i, j)) * 1000.0).round() as u32,
    )
}

fn ev(t: Timestamp, tail: u64, a: AnswerEvent) -> Decoded {
    event(t, tail, EventPayload::Answer(a))
}

/// A first answer, on KC 2: it sets the instruction and is never counted.
fn warm_up() -> Decoded {
    ev(at(0, 0), 0, correct(2, 0, 5_000))
}

fn fold(b: &Bundle, events: &[Decoded]) -> State {
    replay(&KnowledgeModel::default(), &Profile::default(), b, events)
}

fn counted(s: &State, kc: &KcId) -> u32 {
    s.kcs.get(kc).map_or(0, |k| k.counted)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// `answers` five minutes apart from minute `start` of day 0, with event id
/// tails from `tail` on.
fn series(start: i64, tail: u64, answers: impl IntoIterator<Item = AnswerEvent>) -> Vec<Decoded> {
    answers
        .into_iter()
        .zip(0..)
        .map(|(a, n)| ev(at(0, start + 5 * n), tail + u64::try_from(n).unwrap(), a))
        .collect()
}

#[test]
fn retrieval_time_of_a_choice_is_the_first_input() {
    let b = bundle(8);
    let mut a = correct(0, 0, 3_000);
    a.latency_ms = 9_000;
    let s = fold(&b, &[warm_up(), ev(at(0, 10), 1, a)]);
    let k = &s.kcs[&kc_id(0)];
    let residual = beta(&b, 0, 0) - libm::log(3.0);
    assert_eq!(k.counted, 1);
    assert!(close(k.tau, LAMBDA * residual), "{}", k.tau);
    assert_eq!(k.residuals.len(), 1);
    assert!(close(k.residuals[0], residual));
}

#[test]
fn retrieval_time_of_typed_input_adds_typing_beyond_1_5_times_the_usual() {
    let b = bundle(8);
    // 6 characters: the usual time is 6 / 1.5 = 4 s on touch, 6 / 3 = 2 s on
    // a keyboard (the priors v₀), and 1.5 times it is allowed.
    let case = |typing_ms: u32, device: InputDevice| {
        let mut a = correct(0, 1, 4_000);
        a.typing_ms = Some(typing_ms);
        a.chars_typed = Some(6);
        a.input_device = Some(device);
        a.latency_ms = 4_000 + typing_ms;
        let s = fold(&b, &[warm_up(), ev(at(0, 10), 1, a)]);
        s.kcs[&kc_id(0)].residuals[0]
    };
    let beta = beta(&b, 0, 1);
    // Within 1.5 × 4 s = 6 s: t_ret is the first input alone.
    assert!(close(
        case(6_000, InputDevice::Touch),
        beta - libm::log(4.0)
    ));
    assert!(close(
        case(2_000, InputDevice::Touch),
        beta - libm::log(4.0)
    ));
    // 9 s of typing: 3 s beyond, added.
    assert!(close(
        case(9_000, InputDevice::Touch),
        beta - libm::log(7.0)
    ));
    // The same 6 s on a keyboard: 1.5 × 2 s = 3 s allowed, 3 s added.
    assert!(close(
        case(6_000, InputDevice::Keyboard),
        beta - libm::log(7.0)
    ));
}

#[test]
fn retrieval_time_is_capped_at_five_times_the_family_time() {
    let b = bundle(8);
    let s = fold(&b, &[warm_up(), ev(at(0, 10), 1, paced(&b, 0, 0, 40.0))]);
    assert!(close(s.kcs[&kc_id(0)].residuals[0], -libm::log(5.0)));
}

#[test]
fn each_exclusion_rule_excludes() {
    let b = bundle(8);
    let base = || correct(0, 0, 5_000);
    let with = |a: AnswerEvent| counted(&fold(&b, &[warm_up(), ev(at(0, 10), 1, a)]), &kc_id(0));
    assert_eq!(with(base()), 1, "the base answer is counted");

    let mut a = base();
    a.verdict = Verdict::Wrong;
    assert_eq!(with(a), 0, "wrong");
    let mut a = base();
    a.attempt = 2;
    assert_eq!(with(a), 0, "second attempt");
    let mut a = base();
    a.hints = 1;
    assert_eq!(with(a), 0, "hint");
    let mut a = correct(0, 0, 800);
    a.latency_ms = 1_000;
    assert_eq!(with(a), 0, "rapid guess");
    let mut a = base();
    a.hidden_ms = Some(2_000);
    assert_eq!(with(a), 0, "page hidden");
    for missing in 0..3 {
        let mut a = base();
        match missing {
            0 => a.first_input_ms = None,
            1 => a.input_device = None,
            _ => a.hidden_ms = None,
        }
        assert_eq!(with(a), 0, "timing field {missing} missing");
    }
    let mut a = correct(0, 1, 5_000);
    a.typing_ms = None;
    assert_eq!(with(a), 0, "typed input without typing time");
    let mut a = correct(0, 1, 5_000);
    a.chars_typed = None;
    assert_eq!(with(a), 0, "typed input without characters");

    // The first answer of the learner follows no instruction.
    let s = fold(&b, &[ev(at(0, 10), 1, base())]);
    assert_eq!(counted(&s, &kc_id(0)), 0, "first answer");
}

#[test]
fn the_first_answer_after_a_change_of_instruction_is_not_counted() {
    let mut b = bundle(8);
    let other = InstructionId::new("en.instr.other").unwrap();
    for e in &mut b.content.items {
        if e.item.id.as_str().contains("k00") {
            e.item.instruction = other.clone();
        }
    }
    let s = fold(
        &b,
        &[
            warm_up(),
            ev(at(0, 10), 1, correct(0, 0, 5_000)),
            ev(at(0, 15), 2, correct(0, 2, 5_000)),
            ev(at(0, 20), 3, correct(2, 2, 5_000)),
            ev(at(0, 25), 4, correct(0, 4, 5_000)),
        ],
    );
    // k00 items: the first after KC 2 is not counted, the second is; back
    // after KC 2 again, not counted.
    assert_eq!(counted(&s, &kc_id(0)), 1);
    assert_eq!(counted(&s, &kc_id(2)), 0);
}

#[test]
fn an_item_seen_within_14_days_is_not_counted() {
    let b = bundle(8);
    let log = |day: i64| {
        vec![
            warm_up(),
            ev(at(0, 10), 1, correct(0, 0, 5_000)),
            ev(at(day, 10), 2, correct(0, 0, 5_000)),
        ]
    };
    assert_eq!(counted(&fold(&b, &log(13)), &kc_id(0)), 1);
    assert_eq!(counted(&fold(&b, &log(15)), &kc_id(0)), 2);
}

#[test]
fn answers_upcast_from_schema_1_never_count() {
    let b = bundle(8);
    let v1 = |d: &Decoded| {
        let Decoded::Known(e) = d else { unreachable!() };
        let mut v: serde_json::Value = serde_json::from_str(&encode_event(e)).unwrap();
        v["schema_version"] = 1.into();
        let p = v["payload"].as_object_mut().unwrap();
        for f in [
            "first_input_ms",
            "typing_ms",
            "chars_typed",
            "edits",
            "input_device",
            "hidden_ms",
        ] {
            p.remove(f);
        }
        p.insert("backgrounded".into(), false.into());
        decode_event(&v.to_string()).unwrap()
    };
    let log = series(0, 0, (0..8).map(|j| correct(0, j, 5_000)));
    let current = fold(&b, &log);
    assert_eq!(counted(&current, &kc_id(0)), 7);
    let upcast: Vec<Decoded> = log.iter().map(v1).collect();
    let s = fold(&b, &upcast);
    let k = &s.kcs[&kc_id(0)];
    assert_eq!((k.counted, k.tau), (0, 0.0));
    assert!(s.typing.is_empty() && s.baseline.is_empty());
}

#[test]
fn typing_speed_shrinks_to_the_prior() {
    let b = bundle(90);
    let model = KnowledgeModel::default();
    // 6 characters in 2 s: 3 characters a second on touch, prior 1.5, n₀ 5.
    let typed = |j: usize| {
        let mut a = correct(0, j, 4_000);
        a.typing_ms = Some(2_000);
        a.chars_typed = Some(6);
        a.latency_ms = 6_000;
        a
    };
    let log = |n: usize| {
        let mut out = vec![warm_up()];
        out.extend(series(10, 1, (0..n).map(|k| typed(2 * k + 1))));
        fold(&b, &out)
    };
    let speed = |n: usize| model.typing_speed(&log(n), InputDevice::Touch);
    assert!(close(speed(0), 1.5));
    assert!(close(speed(1), (5.0 * 1.5 + 3.0) / 6.0));
    assert!(close(speed(3), (5.0 * 1.5 + 3.0 * 3.0) / 8.0));
    let s = log(40);
    assert_eq!(s.typing[&InputDevice::Touch].len(), 30, "a window of 30");
    assert!(close(
        model.typing_speed(&s, InputDevice::Touch),
        (5.0 * 1.5 + 30.0 * 3.0) / 35.0
    ));
    assert!(close(model.typing_speed(&s, InputDevice::Keyboard), 3.0));
}

/// 200 answers on KC 0 (it becomes known, so they make bⱼ) and 14 on KC 1,
/// each retrieved in `kc1(n) × exp(βᶠ)`, KC 0 always at 2.5 × exp(βᶠ).
fn slow_learner(b: &Bundle, kc1: impl Fn(usize) -> f64) -> State {
    let mut log = vec![warm_up()];
    log.extend(series(10, 1, (0..200).map(|j| paced(b, 0, j, 2.5))));
    log.extend(series(
        10 + 5 * 200,
        1_000,
        (0..14).map(|j| paced(b, 1, j, kc1(j))),
    ));
    fold(b, &log)
}

#[test]
fn a_uniformly_slow_steady_learner_is_fluent_by_branch_2_only() {
    let b = bundle(210);
    let model = KnowledgeModel::default();
    let s = slow_learner(&b, |_| 2.5);
    let f = model.fluency(&s, &b, &kc_id(1));
    assert!(f.counted >= 6, "{f:?}");
    assert!(f.baseline < -0.5, "bⱼ shows the learner is slow: {f:?}");
    assert!(!f.absolute && f.relative && f.holds(), "{f:?}");
}

#[test]
fn a_noisy_slow_learner_is_not_fluent() {
    let b = bundle(210);
    let model = KnowledgeModel::default();
    // The same mean pace, alternately e^±0.6 around it: spread ≈ 0.89.
    let s = slow_learner(&b, |j| 2.5 * libm::exp(if j % 2 == 0 { 0.6 } else { -0.6 }));
    let f = model.fluency(&s, &b, &kc_id(1));
    assert!(f.spread.is_some_and(|x| x > 0.35), "{f:?}");
    assert!(!f.absolute && !f.relative, "{f:?}");
}

#[test]
fn fewer_than_6_counted_answers_are_not_fluent() {
    let b = bundle(8);
    let model = KnowledgeModel::default();
    let log = |n: usize| {
        let mut out = vec![warm_up()];
        out.extend(series(10, 1, (0..n).map(|j| paced(&b, 0, j, 0.5))));
        fold(&b, &out)
    };
    let five = model.fluency(&log(5), &b, &kc_id(0));
    assert!(
        five.counted == 5 && five.tau > 0.0 && !five.holds(),
        "{five:?}"
    );
    let six = model.fluency(&log(6), &b, &kc_id(0));
    assert!(six.counted == 6 && six.absolute, "{six:?}");
}

#[test]
fn the_family_time_does_not_follow_the_learner() {
    let b = bundle(40);
    // One learner slow on KC 0, one fast; then the same answer on KC 2.
    let tau2 = |factor: f64| {
        let mut log = vec![warm_up()];
        log.extend(series(10, 1, (0..30).map(|j| paced(&b, 0, j, factor))));
        log.push(ev(at(1, 0), 500, correct(2, 2, 5_000)));
        fold(&b, &log).kcs[&kc_id(2)].tau
    };
    let expected = LAMBDA * (beta(&b, 2, 2) - libm::log(5.0));
    assert!(close(tau2(3.0), expected));
    assert!(close(tau2(0.4), expected));
    // βᶠ is a function of the bundle alone.
    assert_eq!(Families::of(&b), Families::of(&bundle(40)));
}
