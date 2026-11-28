//! W24 extreme-scale stress measurements: 100 000-event folds, replay
//! scaling, 100 000 planner/placement selections, and the JSON boundary of
//! the browser bridge.
//!
//! The module is measurement code, not a test oracle: it prints raw numbers
//! and returns the final states so the tests can assert the cheap invariants
//! (determinism, hashes) without hiding the timings.

// Measurement code: counters and durations are intentionally truncated to
// the reporting integers, and the long benchmark functions stay readable as
// one sequence.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use crate::{BundleSpec, T0, event_in, session, synthetic_bundle};
use refl_core::bundle::Bundle;
use refl_core::event::{
    AnswerContext, AnswerEvent, Decoded, EventPayload, InputDevice, Response, Verdict,
};
use refl_core::ids::{ItemRef, KcId, SubjectId};
use refl_core::item::{ItemBody, KcRole};
use refl_core::kind::ItemKind;
use refl_core::model::{Families, FoldCtx, KnowledgeModel, LearnerModel, Profile, State, replay};
use refl_core::placement::{self, Placement};
use refl_core::plan::{Budget, History, Params, Phase, Plan, Planner, ReadingSpeed, Step};
use refl_core::rng::SplitMix64;
use refl_core::value::{Lang, Seed, Timestamp};

/// A full 32-KC A1-shaped bundle for benchmark selection. The real graph
/// also has 32 learning KCs; the synthetic one has the same shape and enough
/// items for a 100k selection loop.
pub fn stress_bundle() -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 32,
        items_per_kc: 12,
    })
}

#[derive(Debug, Clone, Default)]
pub struct Latency {
    pub count: usize,
    pub total_ms: f64,
    /// Microseconds, sorted.
    samples: Vec<u64>,
}

impl Latency {
    fn push(&mut self, micros: u64) {
        self.samples.push(micros);
    }

    fn finish(&mut self, total_ms: f64) {
        self.total_ms = total_ms;
        self.count = self.samples.len();
        self.samples.sort_unstable();
    }

    pub fn per_second(&self) -> f64 {
        if self.total_ms <= 0.0 {
            return f64::INFINITY;
        }
        self.count as f64 * 1000.0 / self.total_ms
    }

    pub fn percentile_us(&self, q: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }
        let rank = (q * self.samples.len() as f64).ceil() as usize;
        self.samples[rank.saturating_sub(1).min(self.samples.len() - 1)]
    }

    pub fn max_us(&self) -> u64 {
        self.samples.last().copied().unwrap_or(0)
    }

    pub fn line(&self, label: &str) -> String {
        format!(
            "{label}: n={} total_ms={:.3} ops_per_sec={:.0} p50_us={} p95_us={} p99_us={} max_us={}",
            self.count,
            self.total_ms,
            self.per_second(),
            self.percentile_us(0.50),
            self.percentile_us(0.95),
            self.percentile_us(0.99),
            self.max_us(),
        )
    }
}

/// Peak resident set size of this process, KiB; 0 when `/proc` is absent
/// (for example on wasm32-wasip1).
pub fn peak_rss_kb() -> u64 {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|n| n.parse().ok())
        })
        .unwrap_or(0)
}

/// JSON bytes of the folded state.
pub fn snapshot_bytes(state: &State) -> usize {
    serde_json::to_vec(state).map_or(0, |b| b.len())
}

struct Sample {
    item: ItemRef,
    slot: u16,
    answer: String,
    typed: bool,
    options: Vec<(String, Option<String>)>,
    wrong: Vec<(String, String)>,
    expected_time: u32,
}

fn samples(bundle: &Bundle) -> Vec<Sample> {
    let mut out = Vec::new();
    for entry in &bundle.content.items {
        let body = &entry.item.body;
        let typed = body.typed_input();
        for (slot_index, slot) in body.slots().iter().enumerate() {
            let slot_u16 = u16::try_from(slot_index).unwrap_or(0);
            out.push(Sample {
                item: ItemRef {
                    id: entry.item.id.clone(),
                    version: entry.item.version,
                    seed: None,
                },
                slot: slot_u16,
                answer: slot.answer.clone(),
                typed,
                options: slot
                    .options
                    .iter()
                    .map(|o| {
                        (
                            o.text.clone(),
                            o.misconception.as_ref().map(ToString::to_string),
                        )
                    })
                    .collect(),
                wrong: slot
                    .expected_wrong
                    .iter()
                    .map(|w| (w.pattern.0.clone(), w.misconception.to_string()))
                    .collect(),
                expected_time: entry.item.expected_time.max(1),
            });
        }
    }
    out
}

fn answer_event(sample: &Sample, correct: bool) -> AnswerEvent {
    let (response, verdict, misconception) = if correct {
        if sample.typed {
            (
                Response::Text(sample.answer.clone()),
                Verdict::Correct,
                None,
            )
        } else {
            (Response::Option(0), Verdict::Correct, None)
        }
    } else if sample.typed {
        sample.wrong.first().map_or(
            (Response::Text("xx".into()), Verdict::Wrong, None),
            |(_, mis)| {
                (
                    Response::Text("xx".into()),
                    Verdict::Wrong,
                    Some(mis.clone()),
                )
            },
        )
    } else {
        sample
            .options
            .first()
            .map_or((Response::Option(1), Verdict::Wrong, None), |(_, mis)| {
                (Response::Option(1), Verdict::Wrong, mis.clone())
            })
    };
    AnswerEvent {
        item: sample.item.clone(),
        slot: sample.slot,
        scaffold: 0,
        revealed_deps: false,
        context: AnswerContext::Practice,
        response,
        verdict,
        misconception: misconception.and_then(refl_core::ids::MisconceptionId::new),
        // Above both rapid thresholds (25% of expected time and 2 s for typed
        // answers), so the 100k log exercises the knowledge, memory and
        // misconception updates instead of being discarded as rapid taps.
        latency_ms: (sample.expected_time * 1000 * 3 / 5).max(2_600),
        first_input_ms: Some((sample.expected_time * 1000 * 3 / 5).max(2_600)),
        typing_ms: Some(0),
        chars_typed: Some(0),
        edits: Some(0),
        input_device: Some(InputDevice::Touch),
        hidden_ms: Some(0),
        confidence: None,
        hints: 0,
        attempt: 1,
    }
}

/// `count` valid answer events cycling through the bundle's items. Every
/// fourth answer is wrong, so the state stays active and varied.
pub fn events(bundle: &Bundle, count: usize) -> Vec<Decoded> {
    let samples = samples(bundle);
    assert!(!samples.is_empty(), "the stress bundle has items");
    (0..count)
        .map(|i| {
            let sample = &samples[i % samples.len()];
            let payload = EventPayload::Answer(answer_event(sample, i % 4 != 0));
            event_in(
                Some(session(1)),
                Timestamp(T0.0 + i64::try_from(i).unwrap_or(0)),
                i as u64,
                payload,
            )
        })
        .collect()
}

/// Incremental fold of `events` into one state, with per-event timings.
pub fn fold_native(bundle: &Bundle, events: &[Decoded]) -> (State, Latency, usize, usize) {
    let model = KnowledgeModel::default();
    let mut state = model.init(&Profile::default());
    let families = Families::of(bundle);
    let ctx = FoldCtx::new(bundle, &families);
    let start_bytes = snapshot_bytes(&state);
    let start = Instant::now();
    let mut latency = Latency::default();
    for event in events {
        let t = Instant::now();
        model
            .apply(&mut state, event, &ctx)
            .expect("events are folded in event_id order");
        latency.push(t.elapsed().as_micros() as u64);
    }
    let total = start.elapsed().as_secs_f64() * 1000.0;
    latency.finish(total);
    let end_bytes = snapshot_bytes(&state);
    (state, latency, start_bytes, end_bytes)
}

/// Full replay from scratch, timed once.
pub fn replay_once(bundle: &Bundle, events: &[Decoded]) -> (State, f64) {
    let model = KnowledgeModel::default();
    let start = Instant::now();
    let state = replay(&model, &Profile::default(), bundle, events);
    (state, start.elapsed().as_secs_f64() * 1000.0)
}

/// Replay the same prefix three times and return the median wall time.
pub fn replay_median(bundle: &Bundle, events: &[Decoded]) -> f64 {
    let mut times: Vec<f64> = (0..3).map(|_| replay_once(bundle, events).1).collect();
    times.sort_by(f64::total_cmp);
    times[1]
}

/// First `n` in `1..=len` whose median replay exceeds `target_ms`.
pub fn replay_threshold(
    bundle: &Bundle,
    events: &[Decoded],
    target_ms: f64,
) -> Option<(usize, f64)> {
    let (mut lo, mut hi) = (1usize, events.len());
    let (mut best, mut best_ms) = (None, 0.0);
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let ms = replay_median(bundle, &events[..mid]);
        if ms > target_ms {
            best = Some(mid);
            best_ms = ms;
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    best.map(|n| (n, best_ms))
}

/// A fixed state after `n` events, for selection benchmarks.
pub fn state_after(bundle: &Bundle, events: &[Decoded], n: usize) -> State {
    let model = KnowledgeModel::default();
    replay(
        &model,
        &Profile::default(),
        bundle,
        &events[..n.min(events.len())],
    )
}

fn first_kc(bundle: &Bundle) -> KcId {
    bundle
        .content
        .kcs
        .iter()
        .find(|e| {
            bundle
                .content
                .items_by_kc
                .get(&e.kc.id)
                .is_some_and(|i| !i.primary.is_empty())
        })
        .map(|e| e.kc.id.clone())
        .expect("the bundle has an item")
}

/// `iterations` calls of the planner's next-item selection on a fixed
/// one-step plan.
pub fn plan_selection(bundle: &Bundle, state: &State, iterations: usize) -> Latency {
    let model = KnowledgeModel::default();
    let params = Params::default();
    let speed = ReadingSpeed::default();
    let planner = Planner::new(&model, bundle, &params, &speed);
    let kc = first_kc(bundle);
    let plan = Plan {
        budget: Budget::Ten,
        started: Timestamp(T0.0),
        seed: Seed::new(1).expect("small"),
        learn: Vec::new(),
        candidates: vec![kc.clone()],
        deferred_today: Vec::new(),
        reviews: Vec::new(),
        done: Vec::new(),
        steps: vec![Step {
            kc,
            phase: Phase::Block,
            production: false,
            family: None,
            seconds: 0,
        }],
        break_suggested: false,
        lang: Lang::En,
        end: None,
    };
    let history = History::default();
    let start = Instant::now();
    let mut latency = Latency::default();
    for i in 0..iterations {
        let t = Instant::now();
        let plan_seed = Seed::new((i as u64) % 1_000_000).expect("small");
        let _ = planner.next_item(state, &plan, &history, Timestamp(T0.0), plan_seed);
        latency.push(t.elapsed().as_micros() as u64);
    }
    latency.finish(start.elapsed().as_secs_f64() * 1000.0);
    latency
}

/// `iterations` placement selection calls on a fixed state.
pub fn placement_selection(bundle: &Bundle, state: &State, iterations: usize) -> Latency {
    let model = KnowledgeModel::default();
    let placement = Placement {
        subject: SubjectId::new("en").expect("valid subject"),
        asked: Vec::new(),
        correct: 0,
    };
    let start = Instant::now();
    let mut latency = Latency::default();
    for _ in 0..iterations {
        let t = Instant::now();
        let _ = placement::next(&model, bundle, state, &placement, Timestamp(T0.0));
        latency.push(t.elapsed().as_micros() as u64);
    }
    latency.finish(start.elapsed().as_secs_f64() * 1000.0);
    latency
}

/// A shuffled copy of a prefix, for the replay-vs-incremental proof.
pub fn shuffled(events: &[Decoded], seed: u64) -> Vec<Decoded> {
    let mut out = events.to_vec();
    let mut rng = SplitMix64::new(seed);
    for i in (1..out.len()).rev() {
        let j = usize::try_from(rng.next_u64() % (i as u64 + 1)).expect("small");
        out.swap(i, j);
    }
    out
}

/// KCs with items, used by the selection benchmark's plan.
pub fn kcs_with_items(bundle: &Bundle) -> BTreeMap<KcId, usize> {
    bundle
        .content
        .items_by_kc
        .iter()
        .map(|(kc, items)| (kc.clone(), items.primary.len()))
        .collect()
}

/// Slots where `kc` is primary; measurement helper for per-KC reports.
pub fn primary_slots(bundle: &Bundle, kc: &KcId) -> usize {
    let Some(refs) = bundle.content.items_by_kc.get(kc) else {
        return 0;
    };
    refs.primary
        .iter()
        .filter_map(|r| bundle.projection(r))
        .map(|p| {
            p.slots
                .iter()
                .filter(|s| {
                    s.kcs
                        .iter()
                        .any(|k| &k.kc == kc && k.role == KcRole::Primary)
                })
                .count()
        })
        .sum()
}

/// The body kind of an item, for reporting.
pub fn item_kind(bundle: &Bundle, item: &ItemRef) -> Option<&'static str> {
    let projection = bundle.projection(item)?;
    Some(match projection.kind {
        refl_core::item::ItemKindTag::Choice => "choice",
        refl_core::item::ItemKindTag::FindError => "find_error",
        refl_core::item::ItemKindTag::Matching => "matching",
        refl_core::item::ItemKindTag::TrueFalseNotGiven => "true_false_not_given",
        refl_core::item::ItemKindTag::GapFill => "gap_fill",
        refl_core::item::ItemKindTag::Transformation => "transformation",
        refl_core::item::ItemKindTag::SentenceAssembly => "sentence_assembly",
        refl_core::item::ItemKindTag::Translation => "translation",
        refl_core::item::ItemKindTag::Dictation => "dictation",
        refl_core::item::ItemKindTag::OwnSentence => "own_sentence",
        refl_core::item::ItemKindTag::Writing => "writing",
        refl_core::item::ItemKindTag::Shadowing => "shadowing",
    })
}

/// Slots of a projection, for callers that need the body shape.
pub fn body_slots(body: &ItemBody) -> usize {
    body.slots().len()
}
