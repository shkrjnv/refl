//! Arbitrary-window statistics (`docs/09-stats.md`, «Что закладывается
//! сейчас»). Every value is a fold of the event log; a window does not add
//! events or a separate analytics database.
//!
//! Counts of answers and study time use `client_ts` inside `[from, to)`.
//! Mastery and memory are read at the boundaries, so a month comparison is
//! the difference of two folds, not a second source of truth.

#![allow(clippy::cast_precision_loss)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::bundle::{Bundle, KcStatus};
use crate::event::{AnswerContext, Decoded, EventPayload};
use crate::model::derived::KcProgress;
use crate::model::{Corrections, Families, FoldCtx, KnowledgeModel, LearnerModel, Profile, replay};
use crate::value::Timestamp;

use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DayActivity {
    /// UTC day number since the Unix epoch.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub day: i64,
    pub answers: u32,
    pub correct: u32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContextCount {
    pub context: AnswerContext,
    pub answers: u32,
    pub correct: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PeriodStats {
    pub from: Timestamp,
    pub to: Timestamp,
    pub answers: u32,
    pub correct: u32,
    /// Sum of recorded answer latencies and explanation-view seconds.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub study_seconds: u64,
    pub active_days: u32,
    pub answers_by_day: Vec<DayActivity>,
    pub by_context: Vec<ContextCount>,
    /// KCs mastered (including long-term) at the start/end of the window.
    pub mastered_start: u32,
    pub mastered_end: u32,
    /// Mean FSRS recall at the end over KCs mastered then; `None` without one.
    pub in_memory: Option<f64>,
    pub active_misconceptions_end: u32,
    pub cured_misconceptions_end: u32,
    /// Answers whose timing fields were internally inconsistent.
    pub timing_inconsistent: u32,
    pub timing_inconsistent_rate: Option<f64>,
    /// Validator codes in `plan_issued` events inside the window.
    pub validator_violations: BTreeMap<String, u32>,
}

fn state_at(
    model: &KnowledgeModel,
    bundle: &Bundle,
    profile: &Profile,
    events: &[Decoded],
    at: Timestamp,
) -> crate::model::State {
    let filtered: Vec<Decoded> = events
        .iter()
        .filter(|ev| matches!(ev, Decoded::Known(e) if e.client_ts < at))
        .cloned()
        .collect();
    replay(model, profile, bundle, &filtered)
}

/// The statistics of `[from, to)`; `from >= to` gives an empty window.
///
/// # Panics
/// If an event cannot be folded after sorting. A valid log with its latest
/// corrections cannot reach that state; callers on an untrusted log should
/// decode and replay it first.
#[allow(clippy::too_many_lines)] // one explicit fold of the documented blocks
pub fn period(
    model: &KnowledgeModel,
    bundle: &Bundle,
    profile: &Profile,
    events: &[Decoded],
    from: Timestamp,
    to: Timestamp,
) -> PeriodStats {
    let mut order: Vec<&Decoded> = events.iter().collect();
    order.sort_by(|a, b| crate::model::event_id(a).cmp(crate::model::event_id(b)));
    let corrections = Corrections::from_log(order.iter().copied());
    let families = Families::of(bundle);
    let ctx = FoldCtx {
        bundle,
        families: &families,
        corrections: &corrections,
    };
    let mut state = model.init(profile);
    let mut answers = 0u32;
    let mut correct = 0u32;
    let mut timing_inconsistent = 0u32;
    let mut study_ms = 0u64;
    let mut days: BTreeMap<i64, DayActivity> = BTreeMap::new();
    let mut contexts: BTreeMap<AnswerContext, ContextCount> = BTreeMap::new();
    let mut validator: BTreeMap<String, u32> = BTreeMap::new();
    let empty = from >= to;
    for ev in &order {
        let Decoded::Known(event) = ev else {
            continue;
        };
        if let EventPayload::PlanIssued(plan) = &event.payload
            && !empty
            && event.client_ts >= from
            && event.client_ts < to
        {
            for code in &plan.violations {
                *validator.entry(code.clone()).or_insert(0) += 1;
            }
        }
        if let EventPayload::ExplanationViewed { seconds, .. } = &event.payload
            && !empty
            && event.client_ts >= from
            && event.client_ts < to
        {
            study_ms += u64::from(*seconds) * 1000;
        }
        let applied = model
            .apply(&mut state, ev, &ctx)
            .expect("a sorted log with its corrections folds");
        if empty || event.client_ts < from || event.client_ts >= to {
            continue;
        }
        let EventPayload::Answer(answer) = &event.payload else {
            continue;
        };
        let crate::model::Applied::Answer(outcome) = applied else {
            continue;
        };
        answers += 1;
        correct += u32::from(outcome.correct);
        timing_inconsistent += u32::from(outcome.timing_inconsistent);
        study_ms += u64::from(answer.latency_ms);
        let day = event.client_ts.0.div_euclid(86_400_000);
        let entry = days.entry(day).or_insert(DayActivity {
            day,
            answers: 0,
            correct: 0,
            seconds: 0,
        });
        entry.answers += 1;
        entry.correct += u32::from(outcome.correct);
        entry.seconds += u64::from(answer.latency_ms) / 1000;
        let context = contexts.entry(outcome.context).or_insert(ContextCount {
            context: outcome.context,
            answers: 0,
            correct: 0,
        });
        context.answers += 1;
        context.correct += u32::from(outcome.correct);
    }

    let start = if empty {
        state.clone()
    } else {
        state_at(model, bundle, profile, events, from)
    };
    let end = if empty {
        state.clone()
    } else {
        state_at(model, bundle, profile, events, to)
    };
    let mastered = |s: &crate::model::State, at: Timestamp| {
        u32::try_from(
            bundle
                .content
                .kcs
                .iter()
                .filter(|e| e.status == KcStatus::Live)
                .filter(|e| {
                    matches!(
                        model.progress(s, bundle, &e.kc.id, at),
                        KcProgress::Mastered | KcProgress::LongTerm
                    )
                })
                .count(),
        )
        .unwrap_or(u32::MAX)
    };
    let mut in_memory_values = Vec::new();
    for e in &bundle.content.kcs {
        if e.status != KcStatus::Live {
            continue;
        }
        if matches!(
            model.progress(&end, bundle, &e.kc.id, to),
            KcProgress::Mastered | KcProgress::LongTerm
        ) && let Some(card) = end.kcs.get(&e.kc.id).and_then(|k| k.card.as_ref())
        {
            in_memory_values.push(model.recall(Some(card), to));
        }
    }
    let in_memory = (!in_memory_values.is_empty())
        .then(|| in_memory_values.iter().sum::<f64>() / in_memory_values.len() as f64);
    let active_misconceptions_end = bundle
        .content
        .misconceptions
        .iter()
        .filter(|m| {
            model
                .misconception(&end, &m.id, to)
                .is_some_and(|v| v.status == crate::model::derived::MisconceptionStatus::Active)
        })
        .count();
    let cured_misconceptions_end = bundle
        .content
        .misconceptions
        .iter()
        .filter(|m| {
            model
                .misconception(&end, &m.id, to)
                .is_some_and(|v| v.status == crate::model::derived::MisconceptionStatus::Cured)
        })
        .count();
    let answers_by_day: Vec<DayActivity> = days.into_values().collect();
    let active_days = u32::try_from(answers_by_day.len()).unwrap_or(u32::MAX);
    let by_context = contexts.into_values().collect();
    PeriodStats {
        from,
        to,
        answers,
        correct,
        study_seconds: study_ms / 1000,
        active_days,
        answers_by_day,
        by_context,
        mastered_start: mastered(&start, from),
        mastered_end: mastered(&end, to),
        in_memory,
        active_misconceptions_end: u32::try_from(active_misconceptions_end).unwrap_or(0),
        cured_misconceptions_end: u32::try_from(cured_misconceptions_end).unwrap_or(0),
        timing_inconsistent,
        timing_inconsistent_rate: (answers > 0)
            .then(|| f64::from(timing_inconsistent) / f64::from(answers)),
        validator_violations: validator,
    }
}
