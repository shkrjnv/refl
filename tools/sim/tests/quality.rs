//! K1/K2 and arbitrary-window statistics on the synthetic log. The metrics
//! are folded from the same events as replay; this test checks that every
//! requested facet is populated and that the family audit is gated.

use refl_core::model::{KnowledgeModel, Profile};
use refl_core::quality;
use refl_core::stats;
use refl_core::value::Timestamp;
use refl_sim::{DAY_MS, LogSpec, T0, synthetic_bundle, synthetic_log};

fn report() -> (quality::QualityReport, stats::PeriodStats) {
    let bundle = synthetic_bundle(refl_sim::BundleSpec {
        kcs: 4,
        items_per_kc: 8,
    });
    let log = synthetic_log(
        &bundle,
        LogSpec {
            events: 3_000,
            seed: 41,
        },
    );
    let mut model = KnowledgeModel::default();
    model.params.eta = 0.0;
    model.params.process_noise = 0.0;
    let profile = Profile::default();
    let now = Timestamp(T0.0 + 40 * DAY_MS);
    let quality = quality::report(&model, &bundle, &profile, &log, now);
    let period = stats::period(
        &model,
        &bundle,
        &profile,
        &log,
        T0,
        Timestamp(T0.0 + 30 * DAY_MS),
    );
    (quality, period)
}

#[test]
fn quality_report_exposes_calibration_and_psychometric_facets() {
    let (quality, _) = report();
    assert!(quality.answers > 0);
    assert!(quality.overall.brier.is_some());
    assert!(quality.overall.log_loss.is_some());
    assert!(quality.overall.auc.is_some());
    assert!(quality.by_context.iter().any(|row| row.metric.sample > 0));
    assert!(quality.by_ladder.iter().any(|row| row.metric.sample > 0));
    assert!(quality.calibration.iter().any(|bin| bin.sample > 0));
    assert!(
        quality
            .families
            .iter()
            .any(|family| family.gated && family.unrecognized_rate.is_some())
    );
    assert!(
        quality
            .families
            .iter()
            .any(|family| family.gated && family.point_biserial.is_some())
    );
}

#[test]
fn period_stats_fold_the_requested_window() {
    let (_, period) = report();
    assert!(period.answers > 0);
    assert!(period.correct <= period.answers);
    assert!(period.study_seconds > 0);
    assert!(period.active_days > 0);
    assert!(period.mastered_end >= period.mastered_start);
    assert!(period.answers_by_day.iter().all(|day| day.answers > 0));
    assert!(period.in_memory.is_some() || period.mastered_end == 0);
}

#[test]
fn an_empty_or_reversed_window_has_no_answers() {
    let bundle = synthetic_bundle(refl_sim::BundleSpec {
        kcs: 4,
        items_per_kc: 8,
    });
    let log = synthetic_log(
        &bundle,
        LogSpec {
            events: 40,
            seed: 7,
        },
    );
    let model = KnowledgeModel::default();
    let profile = Profile::default();
    let at = Timestamp(T0.0 + 100 * DAY_MS);
    let empty = stats::period(&model, &bundle, &profile, &log, at, at);
    assert_eq!(empty.answers, 0);
    let reversed = stats::period(&model, &bundle, &profile, &log, Timestamp(at.0 + 1), at);
    assert_eq!(reversed.answers, 0);
}

#[test]
fn timing_inconsistent_answers_are_counted_in_quality_stats() {
    use refl_core::event::{
        AnswerContext, AnswerEvent, EventPayload, InputDevice, Response, Verdict,
    };
    use refl_core::model::{KnowledgeModel, Profile};
    use refl_core::value::Timestamp;

    let bundle = synthetic_bundle(refl_sim::BundleSpec {
        kcs: 1,
        items_per_kc: 2,
    });
    let event = refl_sim::event(
        T0,
        1,
        EventPayload::Answer(AnswerEvent {
            item: refl_sim::item_ref(0, 0),
            slot: 0,
            scaffold: 0,
            revealed_deps: false,
            context: AnswerContext::Review,
            response: Response::Option(0),
            verdict: Verdict::Correct,
            misconception: None,
            latency_ms: 100,
            first_input_ms: Some(200),
            typing_ms: Some(0),
            chars_typed: Some(0),
            edits: Some(0),
            input_device: Some(InputDevice::Touch),
            hidden_ms: Some(0),
            confidence: None,
            hints: 0,
            attempt: 1,
        }),
    );
    let report = quality::report(
        &KnowledgeModel::default(),
        &bundle,
        &Profile::default(),
        &[event],
        Timestamp(T0.0 + 1),
    );
    assert_eq!(report.timing_inconsistent, 1);
    assert_eq!(report.timing_inconsistent_rate, Some(1.0));
}
