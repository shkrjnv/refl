//! K1/K2 quality measurements from a learner's log (`docs/11-quality.md`,
//! «К1. Прогноз», «К2. Диагноз»). Every number is a pure fold of the log;
//! no separate analytics store is involved (`docs/09-stats.md`).
//!
//! The family audit is deliberately gated: a family is reported only after
//! 30 answers from precise learner states (σ < 0.5), and distractor signals
//! additionally need 50 option shows.

#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::bundle::Bundle;
use crate::event::{AnswerContext, Decoded, EventPayload, Response, Verdict};
use crate::ids::{FamilyId, KcId};
use crate::item::LadderLevel;
use crate::model::{Corrections, Families, FoldCtx, KnowledgeModel, LearnerModel, Profile};
use crate::value::Timestamp;

const PRECISE_SIGMA: f64 = 0.5;
const FAMILY_MIN_PRECISE: u32 = 30;
const DISTRACTOR_MIN_SHOWS: u32 = 50;
const DEAD_DISTRACTOR_SHARE: f64 = 0.02;
const MAGNETIC_DIFFERENCE: f64 = 0.05;
const SLOW_CORRECT_FACTOR: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Metric {
    pub sample: u32,
    pub correct: u32,
    /// Mean squared error of the prediction; `None` without answers.
    pub brier: Option<f64>,
    /// Binary log-loss with probabilities clamped to `[1e-6, 1 − 1e-6]`.
    pub log_loss: Option<f64>,
    /// Area under the ROC curve; `None` when one class is absent.
    pub auc: Option<f64>,
    pub mean_predicted: Option<f64>,
}

impl Metric {
    fn from_pairs(pairs: &[(f64, bool)]) -> Self {
        let sample = u32::try_from(pairs.len()).unwrap_or(u32::MAX);
        let correct = pairs.iter().filter(|(_, y)| *y).count();
        if pairs.is_empty() {
            return Self {
                sample: 0,
                correct: 0,
                brier: None,
                log_loss: None,
                auc: None,
                mean_predicted: None,
            };
        }
        let mut brier = 0.0;
        let mut loss = 0.0;
        let mut predicted = 0.0;
        for &(p, y) in pairs {
            let q = p.clamp(1e-6, 1.0 - 1e-6);
            let target = if y { 1.0 } else { 0.0 };
            brier += (p - target) * (p - target);
            loss -= target * libm::log(q) + (1.0 - target) * libm::log(1.0 - q);
            predicted += p;
        }
        let n = pairs.len() as f64;
        Self {
            sample,
            correct: u32::try_from(correct).unwrap_or(u32::MAX),
            brier: Some(brier / n),
            log_loss: Some(loss / n),
            auc: auc(pairs),
            mean_predicted: Some(predicted / n),
        }
    }
}

/// AUC by the rank-sum identity, with average ranks for ties. Returns
/// `None` when only one class is present.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // sample counts are small relative to f64 precision
fn auc(pairs: &[(f64, bool)]) -> Option<f64> {
    let positives = pairs.iter().filter(|(_, y)| *y).count();
    let negatives = pairs.len() - positives;
    if positives == 0 || negatives == 0 {
        return None;
    }
    let mut sorted: Vec<&(f64, bool)> = pairs.iter().collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut rank_sum = 0.0;
    let mut i = 0;
    while i < sorted.len() {
        let mut j = i + 1;
        while j < sorted.len() && sorted[j].0 == sorted[i].0 {
            j += 1;
        }
        // Average 1-based rank over the tie group.
        let average = (i + 1 + j) as f64 / 2.0;
        for (_, positive) in &sorted[i..j] {
            if *positive {
                rank_sum += average;
            }
        }
        i = j;
    }
    let n_pos = positives as f64;
    let n_neg = negatives as f64;
    Some((rank_sum - n_pos * (n_pos + 1.0) / 2.0) / (n_pos * n_neg))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContextMetric {
    pub context: AnswerContext,
    pub metric: Metric,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LadderMetric {
    pub ladder: LadderLevel,
    pub metric: Metric,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CalibrationBin {
    pub lower: f64,
    pub upper: f64,
    pub sample: u32,
    pub observed: Option<f64>,
    pub predicted: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FamilyAudit {
    pub family: FamilyId,
    pub answers: u32,
    pub precise_answers: u32,
    /// The audit is shown only above the 30 precise-answer threshold.
    pub gated: bool,
    pub unrecognized_rate: Option<f64>,
    pub dead_distractors: Vec<u16>,
    pub magnetic_distractors: Vec<u16>,
    /// Empirical family difficulty minus the mean author difficulty, logits.
    pub difficulty_delta: Option<f64>,
    /// Point-biserial correlation of correctness with the learner's μ.
    pub point_biserial: Option<f64>,
    pub slow_correct_median_ms: Option<f64>,
    /// Median correct latency / mean expected latency.
    pub slow_correct_ratio: Option<f64>,
    /// The audit rule flagged a median above twice the expected time.
    pub slow_correct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QualityReport {
    pub answers: u32,
    pub correct: u32,
    pub rapid: u32,
    pub timing_inconsistent: u32,
    pub timing_inconsistent_rate: Option<f64>,
    pub overall: Metric,
    pub by_context: Vec<ContextMetric>,
    pub by_ladder: Vec<LadderMetric>,
    pub calibration: Vec<CalibrationBin>,
    /// `plan_issued` events that carried at least one validator code.
    pub validator_events: u32,
    pub validator_violations: BTreeMap<String, u32>,
    pub families: Vec<FamilyAudit>,
}

struct Record {
    p: f64,
    correct: bool,
    rapid: bool,
    context: AnswerContext,
    ladder: LadderLevel,
    timing_inconsistent: bool,
    verdict: Verdict,
    response: Response,
    latency_ms: u32,
    expected_ms: f64,
    author_b: f64,
    family: FamilyId,
    mu: f64,
    precise: bool,
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        f64::midpoint(values[n / 2 - 1], values[n / 2])
    })
}

fn point_biserial(records: &[&Record]) -> Option<f64> {
    if records.len() < 2 {
        return None;
    }
    let n = records.len() as f64;
    let mean_y = records.iter().filter(|r| r.correct).count() as f64 / n;
    let mean_x = records.iter().map(|r| r.mu).sum::<f64>() / n;
    if mean_y <= 0.0 || mean_y >= 1.0 {
        return None;
    }
    let mut cov = 0.0;
    let mut var_x = 0.0;
    for r in records {
        let dx = r.mu - mean_x;
        cov += dx * (f64::from(u8::from(r.correct)) - mean_y);
        var_x += dx * dx;
    }
    (var_x > 0.0).then_some(cov / libm::sqrt(var_x * n * mean_y * (1.0 - mean_y)))
}

fn family_audit(family: FamilyId, records: &[&Record]) -> FamilyAudit {
    let answers = u32::try_from(records.len()).unwrap_or(u32::MAX);
    let precise_answers = u32::try_from(records.iter().filter(|r| r.precise).count()).unwrap_or(0);
    let gated = precise_answers >= FAMILY_MIN_PRECISE;
    if !gated {
        return FamilyAudit {
            family,
            answers,
            precise_answers,
            gated: false,
            unrecognized_rate: None,
            dead_distractors: Vec::new(),
            magnetic_distractors: Vec::new(),
            difficulty_delta: None,
            point_biserial: None,
            slow_correct_median_ms: None,
            slow_correct_ratio: None,
            slow_correct: false,
        };
    }
    let unrecognized = records
        .iter()
        .filter(|r| r.verdict == Verdict::Unrecognized)
        .count();
    let option_records: Vec<&Record> = records
        .iter()
        .copied()
        .filter(|r| matches!(r.response, Response::Option(_)))
        .collect();
    let shows = u32::try_from(option_records.len()).unwrap_or(0);
    let mut selected: BTreeMap<u16, u32> = BTreeMap::new();
    for r in &option_records {
        if let Response::Option(option) = r.response {
            *selected.entry(option).or_insert(0) += 1;
        }
    }
    let dead = if shows >= DISTRACTOR_MIN_SHOWS {
        selected
            .iter()
            .filter(|(_, n)| f64::from(**n) / f64::from(shows) < DEAD_DISTRACTOR_SHARE)
            .map(|(option, _)| *option)
            .collect()
    } else {
        Vec::new()
    };
    let magnetic = if shows >= DISTRACTOR_MIN_SHOWS {
        let median_mu =
            median(&mut option_records.iter().map(|r| r.mu).collect::<Vec<_>>()).unwrap_or(0.0);
        let mut high: BTreeMap<u16, u32> = BTreeMap::new();
        let mut low: BTreeMap<u16, u32> = BTreeMap::new();
        let (mut high_n, mut low_n) = (0u32, 0u32);
        for r in &option_records {
            if let Response::Option(option) = r.response {
                if r.mu >= median_mu {
                    *high.entry(option).or_insert(0) += 1;
                    high_n += 1;
                } else {
                    *low.entry(option).or_insert(0) += 1;
                    low_n += 1;
                }
            }
        }
        if high_n > 0 && low_n > 0 {
            high.iter()
                .filter_map(|(option, n)| {
                    let high_share = f64::from(*n) / f64::from(high_n);
                    let low_share =
                        f64::from(low.get(option).copied().unwrap_or(0)) / f64::from(low_n);
                    (high_share - low_share > MAGNETIC_DIFFERENCE).then_some(*option)
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let correct_rate =
        f64::from(u32::try_from(records.iter().filter(|r| r.correct).count()).unwrap_or(0))
            / f64::from(answers.max(1));
    let p = correct_rate.clamp(0.01, 0.99);
    let empirical_b = libm::log((1.0 - p) / p);
    let author_b = records.iter().map(|r| r.author_b).sum::<f64>() / f64::from(answers.max(1));
    let mut latencies: Vec<f64> = records
        .iter()
        .filter(|r| r.correct)
        .map(|r| f64::from(r.latency_ms))
        .collect();
    let slow_median = median(&mut latencies);
    let expected_mean =
        records.iter().map(|r| r.expected_ms).sum::<f64>() / f64::from(answers.max(1));
    let slow_ratio = slow_median.map(|m| m / expected_mean.max(1.0));
    FamilyAudit {
        family,
        answers,
        precise_answers,
        gated: true,
        unrecognized_rate: Some(
            f64::from(u32::try_from(unrecognized).unwrap_or(u32::MAX)) / f64::from(answers.max(1)),
        ),
        dead_distractors: dead,
        magnetic_distractors: magnetic,
        difficulty_delta: Some(empirical_b - author_b),
        point_biserial: point_biserial(records),
        slow_correct_median_ms: slow_median,
        slow_correct_ratio: slow_ratio,
        slow_correct: slow_ratio.is_some_and(|ratio| ratio > SLOW_CORRECT_FACTOR),
    }
}

/// Compute K1/K2 metrics for one learner's log. The fold uses the same
/// model and corrections as replay; a malformed log is treated as the replay
/// would treat it (invalid order/answer is not inventable, so this function
/// assumes decoded events from a valid log).
///
/// # Panics
/// If an event cannot be folded after sorting; a valid log with its latest
/// corrections cannot reach that state (`docs/03`, `docs/13`). Callers on an
/// untrusted log should decode and replay it first.
pub fn report(
    model: &KnowledgeModel,
    bundle: &Bundle,
    profile: &Profile,
    events: &[Decoded],
    _now: Timestamp,
) -> QualityReport {
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
    let mut records = Vec::new();
    let mut validator_events = 0;
    let mut validator_violations: BTreeMap<String, u32> = BTreeMap::new();
    for ev in &order {
        let Decoded::Known(event) = ev else {
            continue;
        };
        if let EventPayload::PlanIssued(plan) = &event.payload
            && !plan.violations.is_empty()
        {
            validator_events += 1;
            for code in &plan.violations {
                *validator_violations.entry(code.clone()).or_insert(0) += 1;
            }
        }
        let answer = match &event.payload {
            EventPayload::Answer(a) => Some(a),
            _ => None,
        };
        let before = answer.and_then(|a| {
            let projection = bundle.projection(&a.item)?;
            let slot = projection.slots.get(usize::from(a.slot))?;
            let primary: Vec<&KcId> = slot
                .kcs
                .iter()
                .filter(|k| k.role == crate::item::KcRole::Primary)
                .map(|k| &k.kc)
                .collect();
            let beliefs: Vec<_> = primary
                .iter()
                .filter_map(|kc| model.belief(&state, bundle, kc, event.client_ts))
                .collect();
            let precise = !primary.is_empty()
                && primary.iter().all(|kc| {
                    model
                        .belief(&state, bundle, kc, event.client_ts)
                        .is_some_and(|b| libm::sqrt(b.var) < PRECISE_SIGMA)
                });
            let mu = if beliefs.is_empty() {
                0.0
            } else {
                beliefs.iter().map(|b| b.mu).sum::<f64>() / beliefs.len() as f64
            };
            Some((projection, precise, mu))
        });
        let applied = model
            .apply(&mut state, ev, &ctx)
            .expect("a sorted log with its corrections folds");
        let Some(a) = answer else {
            continue;
        };
        let Some((projection, precise, mu)) = before else {
            continue;
        };
        let crate::model::Applied::Answer(outcome) = applied else {
            continue;
        };
        let verdict = corrections
            .get(&event.event_id)
            .map_or(a.verdict, |correction| correction.verdict);
        records.push(Record {
            p: outcome.predicted,
            correct: outcome.correct,
            rapid: outcome.rapid,
            context: outcome.context,
            ladder: outcome.ladder,
            timing_inconsistent: outcome.timing_inconsistent,
            verdict,
            response: a.response.clone(),
            latency_ms: a.latency_ms,
            expected_ms: f64::from(projection.expected_time) * 1000.0,
            author_b: f64::from(projection.difficulty.get()) - 3.0,
            family: projection.family.clone(),
            mu,
            precise,
        });
    }
    let pairs: Vec<(f64, bool)> = records.iter().map(|r| (r.p, r.correct)).collect();
    let overall = Metric::from_pairs(&pairs);
    let mut contexts: BTreeMap<AnswerContext, Vec<(f64, bool)>> = BTreeMap::new();
    let mut ladders: BTreeMap<LadderLevel, Vec<(f64, bool)>> = BTreeMap::new();
    for r in &records {
        contexts
            .entry(r.context)
            .or_default()
            .push((r.p, r.correct));
        ladders.entry(r.ladder).or_default().push((r.p, r.correct));
    }
    let by_context = contexts
        .into_iter()
        .map(|(context, pairs)| ContextMetric {
            context,
            metric: Metric::from_pairs(&pairs),
        })
        .collect();
    let by_ladder = ladders
        .into_iter()
        .map(|(ladder, pairs)| LadderMetric {
            ladder,
            metric: Metric::from_pairs(&pairs),
        })
        .collect();
    let mut calibration: Vec<CalibrationBin> = (0..10)
        .map(|i| {
            let lower = f64::from(i) / 10.0;
            let upper = f64::from(i + 1) / 10.0;
            let bin: Vec<&Record> = records
                .iter()
                .filter(|r| r.p >= lower && (i == 9 || r.p < upper))
                .collect();
            let sample = u32::try_from(bin.len()).unwrap_or(u32::MAX);
            let predicted =
                (!bin.is_empty()).then(|| bin.iter().map(|r| r.p).sum::<f64>() / bin.len() as f64);
            let observed = (!bin.is_empty()).then(|| {
                f64::from(u32::try_from(bin.iter().filter(|r| r.correct).count()).unwrap_or(0))
                    / bin.len() as f64
            });
            CalibrationBin {
                lower,
                upper,
                sample,
                observed,
                predicted,
            }
        })
        .collect();
    calibration.retain(|bin| bin.sample > 0);
    let mut by_family: BTreeMap<FamilyId, Vec<&Record>> = BTreeMap::new();
    for r in &records {
        by_family.entry(r.family.clone()).or_default().push(r);
    }
    let families = by_family
        .into_iter()
        .map(|(family, records)| family_audit(family, &records))
        .collect();
    let timing_inconsistent =
        u32::try_from(records.iter().filter(|r| r.timing_inconsistent).count()).unwrap_or(u32::MAX);
    let answers = u32::try_from(records.len()).unwrap_or(u32::MAX);
    QualityReport {
        answers,
        correct: u32::try_from(records.iter().filter(|r| r.correct).count()).unwrap_or(0),
        rapid: u32::try_from(records.iter().filter(|r| r.rapid).count()).unwrap_or(0),
        timing_inconsistent,
        timing_inconsistent_rate: (answers > 0)
            .then(|| f64::from(timing_inconsistent) / f64::from(answers)),
        overall,
        by_context,
        by_ladder,
        calibration,
        validator_events,
        validator_violations,
        families,
    }
}
