//! The learner simulator of `docs/11-quality.md`, «К5»: a cohort of
//! synthetic learners in daily sessions, planned by our session builder or
//! by the «только FSRS» baseline, folded by learner model v1.
//!
//! The learners come from a different model than ours on purpose, so the
//! planner is not checked against its own assumptions: their prerequisites
//! drop some of our `requires` edges and add hidden ones; they forget
//! exponentially, at a rate of their own; some hold wrong ideas our
//! misconception list does not have; guessing, slips, speed and fatigue
//! late in a session differ per learner. Everything is a function of a seed.

use std::collections::BTreeMap;

use refl_core::bundle::Bundle;
use refl_core::content::{ExplanationDepth, ExplanationTarget};
use refl_core::event::PlanIssuedEvent;
use refl_core::event::{AnswerContext, AnswerEvent, Confidence, EventPayload, Response, Verdict};
use refl_core::ids::ExplanationId;
use refl_core::ids::{ItemRef, KcId, MisconceptionId};
use refl_core::item::KcRole;
use refl_core::kind::ItemKind;
use refl_core::model::derived::KcProgress;
use refl_core::model::math::sigmoid;
use refl_core::model::{
    AnswerOutcome, Applied, FoldCtx, KnowledgeModel, LearnerModel, Profile, State,
};
use refl_core::plan::{
    Answered, Budget, Done, History, Next, Outcome, Params, Phase, Plan, Planner, ReadingSpeed,
    Shown, Step,
};
use refl_core::rng::SplitMix64;
use refl_core::value::Lang;
use refl_core::value::{Seed, Timestamp};

use crate::{DAY_MS, T0, event};

/// One reply of a learner to one slot.
#[derive(Debug, Clone)]
pub struct Reply {
    pub response: Response,
    pub verdict: Verdict,
    pub misconception: Option<MisconceptionId>,
    pub latency_ms: u32,
    pub confidence: Option<Confidence>,
}

/// Hidden state of one KC of a learner.
#[derive(Debug, Clone)]
struct TrueKc {
    /// Skill without practice, logits.
    base: f64,
    /// Skill added by practice, before forgetting.
    gain: f64,
    /// Days; forgetting is `exp(−rate · days / stability)`.
    stability: f64,
    last: Option<Timestamp>,
    /// Chance of answering from a wrong idea our content does not diagnose.
    bug: f64,
}

/// A synthetic learner of the generator described in the module docs.
#[derive(Debug, Clone)]
pub struct Learner {
    rng: SplitMix64,
    learn_rate: f64,
    forget_rate: f64,
    slip: f64,
    rapid: f64,
    fatigue: f64,
    /// Latency multiplier.
    pace: f64,
    kcs: BTreeMap<KcId, TrueKc>,
    /// The learner's own prerequisites: (KC, what it needs).
    requires: Vec<(KcId, KcId)>,
}

/// Discrimination of the learner's response curve (ours is 1.0).
const SLOPE: f64 = 1.3;

/// A fixed per-item shift of the true difficulty from the author's,
/// in `[−0.5, 0.5]`: FNV-1a of the id.
#[allow(clippy::cast_precision_loss)] // 16 bits
fn difficulty_shift(item: &ItemRef) -> f64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in item.id.as_str().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (h & 0xFFFF) as f64 / 65_535.0 - 0.5
}

#[allow(clippy::cast_precision_loss)] // timestamps are within 2^53
fn days(from: Timestamp, to: Timestamp) -> f64 {
    (to.0 - from.0).max(0) as f64 / DAY_MS as f64
}

/// Uniform index below `n`.
fn pick(rng: &mut SplitMix64, n: usize) -> usize {
    let n = u64::try_from(n).expect("usize fits u64");
    usize::try_from(rng.next_u64() % n).expect("below n")
}

impl Learner {
    pub fn new(bundle: &Bundle, seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let offset = 0.7 * rng.next_normal();
        let learn_rate = 0.25 + 0.35 * rng.next_f64();
        let forget_rate = 0.5 + 1.5 * rng.next_f64();
        let slip = 0.02 + 0.08 * rng.next_f64();
        let rapid = 0.05 * rng.next_f64() * rng.next_f64();
        let fatigue = 0.002 + 0.006 * rng.next_f64();
        let pace = 0.7 + 0.8 * rng.next_f64();
        let mut kcs = BTreeMap::new();
        for e in &bundle.content.kcs {
            let base =
                offset - 1.2 - 0.6 * (f64::from(e.level_rank) - 1.0) + 0.8 * rng.next_normal();
            let bug = if rng.next_f64() < 0.3 {
                0.15 + 0.25 * rng.next_f64()
            } else {
                0.0
            };
            kcs.insert(
                e.kc.id.clone(),
                TrueKc {
                    base,
                    gain: 0.0,
                    stability: 1.0,
                    last: None,
                    bug,
                },
            );
        }
        // Our direct `requires`, each kept with probability 0.7, and hidden
        // ones from a KC to any KC before it in (level, id) order, so the
        // learner's graph stays acyclic.
        let mut requires = Vec::new();
        for (kc, reqs) in &bundle.content.requires_closure {
            for r in reqs.iter().filter(|r| r.distance == 1) {
                if rng.next_f64() < 0.7 {
                    requires.push((kc.clone(), r.kc.clone()));
                }
            }
        }
        let order: Vec<(u16, &KcId)> = bundle
            .content
            .kcs
            .iter()
            .map(|e| (e.level_rank, &e.kc.id))
            .collect();
        for a in &order {
            for b in order.iter().filter(|b| *b < a) {
                if rng.next_f64() < 0.1 && !requires.iter().any(|(x, y)| x == a.1 && y == b.1) {
                    requires.push((a.1.clone(), b.1.clone()));
                }
            }
        }
        Self {
            rng,
            learn_rate,
            forget_rate,
            slip,
            rapid,
            fatigue,
            pace,
            kcs,
            requires,
        }
    }

    /// The same learner, guessing rapidly half the time: exercises the
    /// rapid-guess rule in tests.
    #[must_use]
    pub fn impatient(mut self) -> Self {
        self.rapid = 0.5;
        self
    }

    /// True skill on `kc` at `t`.
    fn skill(&self, kc: &KcId, t: Timestamp) -> f64 {
        self.kcs.get(kc).map_or(-3.0, |k| {
            let kept = k.last.map_or(0.0, |last| {
                libm::exp(-self.forget_rate * days(last, t) / k.stability)
            });
            k.base + k.gain * kept
        })
    }

    /// 0.35 for every own prerequisite still unknown at `t`.
    fn readiness(&self, kc: &KcId, t: Timestamp) -> f64 {
        self.requires
            .iter()
            .filter(|(k, _)| k == kc)
            .map(|(_, j)| if self.skill(j, t) < 0.0 { 0.35 } else { 1.0 })
            .product()
    }

    /// Practice on `kc` at `t`: the kept skill plus a step that shrinks near
    /// the ceiling; spaced practice makes memory more stable.
    fn practise(&mut self, kc: &KcId, t: Timestamp, weight: f64) {
        let now = self.skill(kc, t);
        let step = weight * self.learn_rate * self.readiness(kc, t) * (1.0 - sigmoid(now - 2.5));
        let Some(k) = self.kcs.get_mut(kc) else {
            return;
        };
        let spaced = k.last.is_none_or(|last| days(last, t) >= 0.5);
        if spaced && k.last.is_some() {
            k.stability *= 1.6;
        } else if !spaced {
            k.stability += 0.1;
        }
        k.gain = now - k.base + step;
        k.last = Some(t);
    }

    /// Seconds this learner takes to read `words`: 100 words a minute at
    /// their pace, with noise; at least 10.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // positive and bounded
    pub fn reading_seconds(&mut self, words: usize) -> u32 {
        let s = words as f64 * 0.6 * self.pace * libm::exp(0.3 * self.rng.next_normal());
        s.clamp(10.0, 1800.0) as u32
    }

    /// The learner reads the explanation of `kc`.
    pub fn lesson(&mut self, kc: &KcId, at: Timestamp) {
        self.practise(kc, at, 0.8);
        if let Some(k) = self.kcs.get_mut(kc) {
            k.bug *= 0.7;
        }
    }

    /// Probability of a correct answer to a typed item of medium difficulty
    /// on `kc` at `t`, without guessing or fatigue: the retention measure.
    pub fn recall(&self, kc: &KcId, t: Timestamp) -> f64 {
        (1.0 - self.slip) * sigmoid(SLOPE * self.skill(kc, t))
    }

    /// The learner answers `slot` of `item` at `at`, `minutes` into the
    /// session, and learns from it.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::too_many_lines,
        clippy::many_single_char_names
    )] // latencies are positive and bounded; one generator, names of the formulas
    pub fn answer(
        &mut self,
        bundle: &Bundle,
        item: &ItemRef,
        slot: u16,
        at: Timestamp,
        minutes: f64,
    ) -> Reply {
        let items = &bundle.content.items;
        let entry = &items[items
            .binary_search_by(|e| e.item.id.cmp(&item.id))
            .expect("shown items are in the bundle")];
        let body = &entry.item.body;
        let s = &body.slots()[usize::from(slot)];
        let typed = body.typed_input();
        let expected = f64::from(entry.item.expected_time) * 1000.0;
        let b = f64::from(entry.item.difficulty.get()) - 3.0 + difficulty_shift(item);
        let primary: Vec<&KcId> = s
            .kcs
            .iter()
            .filter(|k| k.role == KcRole::Primary)
            .map(|k| &k.kc)
            .collect();

        if self.rng.next_f64() < self.rapid * (1.0 + minutes / 10.0) {
            let latency_ms = if typed { 800 } else { (0.1 * expected) as u32 };
            let (response, verdict, misconception) = if typed {
                (Response::Text("?".into()), Verdict::Unrecognized, None)
            } else {
                let i = pick(&mut self.rng, s.options.len() + 1);
                if i == 0 {
                    (Response::Option(0), Verdict::Correct, None)
                } else {
                    let o = &s.options[i - 1];
                    let idx = u16::try_from(i).expect("few options");
                    (
                        Response::Option(idx),
                        Verdict::Wrong,
                        o.misconception.clone(),
                    )
                }
            };
            return Reply {
                response,
                verdict,
                misconception,
                latency_ms,
                confidence: Some(Confidence::Unsure),
            };
        }

        let bug = primary
            .iter()
            .filter_map(|kc| self.kcs.get(*kc))
            .map(|k| k.bug)
            .fold(0.0, f64::max);
        let from_bug = self.rng.next_f64() < bug;
        let p_skill: f64 = s
            .kcs
            .iter()
            .map(|k| sigmoid(SLOPE * (self.skill(&k.kc, at) - b)))
            .product();
        let g = body.guessing(slot);
        let slip = (self.slip + self.fatigue * minutes).min(0.4);
        let correct = !from_bug && self.rng.next_f64() < g + (1.0 - g - slip) * p_skill;
        let theta = primary
            .iter()
            .map(|kc| self.skill(kc, at))
            .fold(f64::INFINITY, f64::min);
        let mut latency = expected
            * self.pace
            * libm::exp(-0.2 * (theta - b))
            * libm::exp(0.25 * self.rng.next_normal());
        latency = latency.clamp(0.3 * expected, 5.0 * expected);
        if typed {
            latency = latency.max(2500.0);
        }
        let confidence = if from_bug || p_skill > 0.7 {
            Confidence::Sure
        } else {
            Confidence::Unsure
        };
        let (response, verdict, misconception) = if correct {
            if typed && self.rng.next_f64() < 0.03 {
                (
                    Response::Text(format!("{}x", s.answer)),
                    Verdict::Typo,
                    None,
                )
            } else if typed {
                (Response::Text(s.answer.clone()), Verdict::Correct, None)
            } else {
                (Response::Option(0), Verdict::Correct, None)
            }
        } else if typed {
            let diagnosed = !from_bug && !s.expected_wrong.is_empty() && self.rng.next_f64() < 0.7;
            if diagnosed {
                let w = &s.expected_wrong[pick(&mut self.rng, s.expected_wrong.len())];
                (
                    Response::Text("x".into()),
                    Verdict::Wrong,
                    Some(w.misconception.clone()),
                )
            } else {
                (Response::Text("xx".into()), Verdict::Unrecognized, None)
            }
        } else {
            // A wrong idea we do not list picks the undiagnosed option when
            // there is one; otherwise any distractor.
            let noise = s.options.iter().position(|o| o.misconception.is_none());
            let k = match noise {
                Some(k) if from_bug => k,
                _ => pick(&mut self.rng, s.options.len().max(1)),
            };
            let o = s.options.get(k);
            let idx = u16::try_from(k + 1).expect("few options");
            (
                Response::Option(idx),
                Verdict::Wrong,
                o.and_then(|o| o.misconception.clone()),
            )
        };
        for kc in &primary {
            self.practise(kc, at, if correct { 1.0 } else { 0.6 });
            if !correct
                && !from_bug
                && let Some(k) = self.kcs.get_mut(*kc)
            {
                k.bug *= 0.85;
            }
        }
        Reply {
            response,
            verdict,
            misconception,
            latency_ms: latency as u32,
            confidence: Some(confidence),
        }
    }
}

/// One learner's log as it grows: the folded state, the clock and the
/// mastery counters.
#[derive(Debug, Clone)]
pub struct Track<'a> {
    pub model: &'a KnowledgeModel,
    pub bundle: &'a Bundle,
    pub state: State,
    /// Families shown and KCs deferred, from every folded event.
    pub history: History,
    /// Seconds of every explanation view, for the lesson time.
    pub views: Vec<(ExplanationId, u32)>,
    /// Milliseconds; strictly increases with every event.
    pub clock: i64,
    rng: SplitMix64,
    /// Non-rapid answers per KC where it is primary.
    pub answers: BTreeMap<KcId, u32>,
    /// Answers until the model first counted the KC mastered.
    pub to_mastery: BTreeMap<KcId, u32>,
}

impl<'a> Track<'a> {
    pub fn new(model: &'a KnowledgeModel, bundle: &'a Bundle, start: Timestamp, seed: u64) -> Self {
        Self {
            model,
            bundle,
            state: model.init(&Profile::default()),
            history: History::default(),
            views: Vec::new(),
            clock: start.0,
            rng: SplitMix64::new(seed),
            answers: BTreeMap::new(),
            to_mastery: BTreeMap::new(),
        }
    }

    /// This learner's reading speed from their own views (docs/04).
    pub fn speed(&self, params: &Params) -> ReadingSpeed {
        ReadingSpeed::from_views(
            self.bundle,
            params,
            self.views.iter().map(|(id, s)| (id, *s)),
        )
    }

    pub fn now(&self) -> Timestamp {
        Timestamp(self.clock)
    }

    /// A fresh seed for one decision.
    pub fn seed(&mut self) -> Seed {
        Seed::new(self.rng.next_u64() >> 11).expect("53 bits")
    }

    /// Moves the clock on by `ms` (at least 1).
    pub fn wait(&mut self, ms: i64) {
        self.clock += ms.max(1);
    }

    /// Folds an event at the next millisecond.
    fn fold(&mut self, payload: EventPayload) -> Applied {
        self.wait(1);
        let ev = event(self.now(), self.rng.next_u64(), payload);
        self.history.record(self.bundle, &ev);
        self.model
            .apply(&mut self.state, &ev, &FoldCtx::new(self.bundle))
            .expect("events are folded in order")
    }

    fn plan_issued(&mut self, ev: PlanIssuedEvent) {
        self.fold(EventPayload::PlanIssued(ev));
    }

    /// The learner reads `explanation` of `kc` at their own pace.
    fn lesson(&mut self, learner: &mut Learner, kc: &KcId, explanation: &ExplanationId) {
        let e = self
            .bundle
            .content
            .explanations
            .iter()
            .find(|e| &e.id == explanation)
            .expect("the lesson's explanation is in the bundle");
        let words = e.body.split_whitespace().count()
            + e.l1_contrast
                .as_deref()
                .map_or(0, |t| t.split_whitespace().count())
            + e.examples
                .iter()
                .map(|x| x.text.split_whitespace().count())
                .sum::<usize>();
        let seconds = learner.reading_seconds(words);
        self.wait(i64::from(seconds) * 1000);
        learner.lesson(kc, self.now());
        self.fold(EventPayload::ExplanationViewed {
            explanation: e.id.clone(),
            depth: e.depth,
            seconds,
        });
        self.views.push((e.id.clone(), seconds));
    }

    /// The learner answers; the answer is folded and counted.
    fn answer(
        &mut self,
        learner: &mut Learner,
        item: &ItemRef,
        slot: u16,
        context: AnswerContext,
        minutes: f64,
    ) -> (AnswerEvent, AnswerOutcome) {
        let reply = learner.answer(self.bundle, item, slot, self.now(), minutes);
        self.wait(i64::from(reply.latency_ms));
        let a = AnswerEvent {
            item: item.clone(),
            slot,
            scaffold: 0,
            revealed_deps: false,
            context,
            response: reply.response,
            verdict: reply.verdict,
            misconception: reply.misconception,
            latency_ms: reply.latency_ms,
            backgrounded: false,
            confidence: reply.confidence,
            hints: 0,
            attempt: 1,
        };
        let Applied::Answer(outcome) = self.fold(EventPayload::Answer(a.clone())) else {
            panic!("the answer's item is in the bundle");
        };
        let projection = self.bundle.projection(item).expect("in the bundle");
        if !outcome.rapid {
            for k in &projection.slots[usize::from(slot)].kcs {
                if k.role == KcRole::Primary {
                    *self.answers.entry(k.kc.clone()).or_insert(0) += 1;
                }
            }
        }
        for (kc, k) in &self.state.kcs {
            if k.ever_mastered && !self.to_mastery.contains_key(kc) {
                let n = self.answers.get(kc).copied().unwrap_or(0);
                self.to_mastery.insert(kc.clone(), n);
            }
        }
        self.wait(3000);
        (a, outcome)
    }
}

/// What one session did.
#[derive(Debug, Clone, Default)]
pub struct SessionReport {
    pub answers: u32,
    pub correct: u32,
    /// At some point fewer than half of the last 8 answers were correct.
    pub frustrated: bool,
    pub seconds: f64,
    /// Plans and decisions validated.
    pub checks: u32,
    /// Violation codes over every check.
    pub violations: BTreeMap<String, u32>,
    pub rules: BTreeMap<&'static str, u32>,
    pub issued: Vec<PlanIssuedEvent>,
    pub plan: Option<Plan>,
    /// Why the session ended; `None` if only the simulation's time guard
    /// stopped it.
    pub end: Option<&'static str>,
    window: Vec<bool>,
}

impl SessionReport {
    fn check(&mut self, codes: impl IntoIterator<Item = String>) {
        self.checks += 1;
        for c in codes {
            *self.violations.entry(c).or_insert(0) += 1;
        }
    }

    fn answered(&mut self, correct: bool) {
        self.answers += 1;
        self.correct += u32::from(correct);
        self.window.push(correct);
        if self.window.len() > 8 {
            self.window.remove(0);
        }
        if self.window.len() == 8 && self.window.iter().filter(|c| **c).count() < 4 {
            self.frustrated = true;
        }
    }
}

fn context(phase: Phase) -> AnswerContext {
    match phase {
        Phase::Review => AnswerContext::Review,
        Phase::Lesson | Phase::Block => AnswerContext::Lesson,
        Phase::Interleaved | Phase::Prerequisite | Phase::Final => AnswerContext::Practice,
    }
}

#[allow(clippy::cast_precision_loss)] // milliseconds within a day
fn minutes_since(track: &Track<'_>, started: i64) -> f64 {
    (track.clock - started) as f64 / 60_000.0
}

/// One session planned by our session builder; every plan and decision is
/// validated.
pub fn session_ours(
    track: &mut Track<'_>,
    params: &Params,
    learner: &mut Learner,
    budget: Budget,
) -> SessionReport {
    let pace = track.speed(params);
    let planner = Planner::new(track.model, track.bundle, params, &pace);
    let mut r = SessionReport::default();
    let started = track.clock;
    let seed = track.seed();
    let session = planner.build_session(
        &track.state,
        &track.history,
        track.now(),
        budget,
        Lang::Ru,
        seed,
    );
    r.check(
        planner
            .validate_plan(&session.plan, &track.state, track.now())
            .iter()
            .map(|v| v.code().to_string()),
    );
    if let Some(ev) = session.issued {
        r.issued.push(ev.clone());
        track.plan_issued(ev);
    }
    let mut plan = session.plan;
    // A plan always ends; the bound only guards the simulation.
    let limit = 3 * i64::from(budget.seconds()) * 1000;
    while track.clock - started < limit {
        let seed = track.seed();
        match planner.next_item(&track.state, &plan, &track.history, track.now(), seed) {
            Next::Finished { end } => {
                r.end = Some(end.code());
                break;
            }
            Next::Lesson { kc, explanation } => {
                track.lesson(learner, &kc, &explanation);
                let adapted = planner.adapt(&track.state, &plan, &Outcome::Lesson, track.now());
                plan = adapted.plan;
            }
            Next::Item(choice) => {
                r.check(choice.issued.violations.iter().cloned());
                r.issued.push(choice.issued.clone());
                track.plan_issued(choice.issued);
                let phase = plan.steps[0].phase;
                let minutes = minutes_since(track, started);
                let (a, outcome) =
                    track.answer(learner, &choice.item, choice.slot, context(phase), minutes);
                r.answered(outcome.correct);
                let answered = Outcome::Answer(Answered::new(&a, &outcome));
                let adapted = planner.adapt(&track.state, &plan, &answered, track.now());
                if let Some(rule) = &adapted.rule {
                    *r.rules.entry(rule.code()).or_insert(0) += 1;
                }
                if let Some(ev) = adapted.issued {
                    r.check(ev.violations.iter().cloned());
                    r.issued.push(ev.clone());
                    track.plan_issued(ev);
                }
                r.check(
                    planner
                        .validate_plan(&adapted.plan, &track.state, track.now())
                        .iter()
                        .map(|v| v.code().to_string()),
                );
                plan = adapted.plan;
            }
        }
    }
    #[allow(clippy::cast_precision_loss)] // within a day
    {
        r.seconds = (track.clock - started) as f64 / 1000.0;
    }
    r.plan = Some(plan);
    r
}

/// The items a baseline session shows and how it picks them: at random, or
/// by our `next_item` on a one-step plan of the same KC and phase.
struct Baseline<'p> {
    /// Our next-item choice; `None` for the random pick.
    planner: Option<Planner<'p>>,
    /// The answered items as plan steps, so `next_item` skips them.
    plan: Plan,
}

impl Baseline<'_> {
    /// Eligible unseen slots where `kc` is primary.
    fn eligible(&self, bundle: &Bundle, kc: &KcId) -> Vec<(ItemRef, u16)> {
        let Some(refs) = bundle.content.items_by_kc.get(kc) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in &refs.primary {
            let seen = self
                .plan
                .done
                .iter()
                .any(|d| d.shown.as_ref().is_some_and(|s| s.item.id == item.id));
            if seen {
                continue;
            }
            let p = bundle.projection(item).expect("in the bundle");
            for (slot, s) in (0u16..).zip(&p.slots) {
                if s.kcs
                    .iter()
                    .any(|k| &k.kc == kc && k.role == KcRole::Primary)
                {
                    out.push((item.clone(), slot));
                }
            }
        }
        out
    }

    /// The next item on `kc` in `phase`; `None` when none is left.
    fn pick(&mut self, track: &mut Track<'_>, kc: &KcId, phase: Phase) -> Option<(ItemRef, u16)> {
        let options = self.eligible(track.bundle, kc);
        if options.is_empty() {
            return None;
        }
        let Some(planner) = &self.planner else {
            return Some(options[pick(&mut track.rng, options.len())].clone());
        };
        self.plan.steps = vec![Step {
            kc: kc.clone(),
            phase,
            production: false,
            family: None,
            seconds: 0,
        }];
        let seed = track.seed();
        match planner.next_item(&track.state, &self.plan, &track.history, track.now(), seed) {
            Next::Item(c) => Some((c.item, c.slot)),
            Next::Lesson { .. } | Next::Finished { .. } => None,
        }
    }

    fn record(
        &mut self,
        bundle: &Bundle,
        kc: &KcId,
        phase: Phase,
        a: &AnswerEvent,
        o: &AnswerOutcome,
    ) {
        let p = bundle.projection(&a.item).expect("in the bundle");
        self.plan.done.push(Done {
            step: Step {
                kc: kc.clone(),
                phase,
                production: false,
                family: None,
                seconds: p.expected_time,
            },
            shown: Some(Shown {
                item: a.item.clone(),
                slot: a.slot,
                family: p.family.clone(),
                kind: p.kind,
                seconds: p.expected_time,
                correct: o.correct,
                rapid: o.rapid,
                sure_error: false,
            }),
            rule: None,
        });
    }
}

/// One session of the «только FSRS» baseline (`docs/11-quality.md`, «К5»):
/// due reviews by FSRS, lowest R first, up to the same review share, then
/// the first KC in prerequisite order the model has not yet mastered, with
/// its explanation once and eligible items until the budget is spent. No
/// blocks, root causes or final. With `ours`, every item is picked by our
/// `next_item` (ladder, target difficulty, family freshness): the
/// «только FSRS + наш выбор задания» arm; without, at random.
#[allow(clippy::too_many_lines)] // one session recipe
pub fn session_fsrs(
    track: &mut Track<'_>,
    learner: &mut Learner,
    budget: Budget,
    params: &Params,
    ours: bool,
) -> SessionReport {
    let mut r = SessionReport::default();
    let started = track.clock;
    let bundle = track.bundle;
    let model = track.model;
    let speed = track.speed(params);
    let mut base = Baseline {
        planner: ours.then(|| Planner::new(model, bundle, params, &speed)),
        plan: Plan {
            budget,
            started: track.now(),
            seed: Seed::new(0).expect("small"),
            learn: None,
            reviews: Vec::new(),
            done: Vec::new(),
            steps: Vec::new(),
            break_suggested: false,
            lang: Lang::Ru,
            end: None,
        },
    };
    let mut used = 0.0;
    let total = f64::from(budget.seconds());
    let review_share = params.review_share;

    let now = track.now();
    let mut due: Vec<(f64, &KcId)> = bundle
        .content
        .kcs
        .iter()
        .filter_map(|e| {
            let card = track.state.kcs.get(&e.kc.id)?.card.as_ref()?;
            let r = model.recall(Some(card), now);
            (r < 0.9).then_some((r, &e.kc.id))
        })
        .collect();
    due.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let reviewed = !due.is_empty();
    for (_, kc) in due {
        let Some((item, slot)) = base.pick(track, kc, Phase::Review) else {
            continue;
        };
        let seconds = bundle
            .projection(&item)
            .expect("in the bundle")
            .expected_time;
        if used + f64::from(seconds) > review_share * total {
            break;
        }
        used += f64::from(seconds);
        let minutes = minutes_since(track, started);
        let (a, outcome) = track.answer(learner, &item, slot, AnswerContext::Review, minutes);
        r.answered(outcome.correct);
        base.record(bundle, kc, Phase::Review, &a, &outcome);
    }

    // Prerequisite order: depth in `requires`, then id.
    let mut order: Vec<(usize, &KcId)> = bundle
        .content
        .kcs
        .iter()
        .filter(|e| {
            bundle
                .content
                .items_by_kc
                .get(&e.kc.id)
                .is_some_and(|r| !r.primary.is_empty())
        })
        .map(|e| {
            let depth = bundle
                .content
                .requires_closure
                .get(&e.kc.id)
                .map_or(0, |r| {
                    r.iter().map(|x| usize::from(x.distance)).max().unwrap_or(0)
                });
            (depth, &e.kc.id)
        })
        .collect();
    order.sort();
    let topic = order
        .into_iter()
        .map(|(_, kc)| kc)
        .find(|kc| track.state.kcs.get(*kc).is_none_or(|k| !k.ever_mastered));
    r.end = Some(match topic {
        None if reviewed => "reviews_done",
        None => "nothing_due",
        Some(kc) => {
            if !track.state.explained.contains(kc)
                && let Some(e) = bundle.content.explanations.iter().find(|e| {
                    e.target == ExplanationTarget::Kc(kc.clone())
                        && e.depth == ExplanationDepth::Short
                        && e.review.is_shown()
                })
            {
                let id = e.id.clone();
                track.lesson(learner, kc, &id);
                used += f64::from(track.views.last().map_or(0, |v| v.1));
            }
            loop {
                let Some((item, slot)) = base.pick(track, kc, Phase::Block) else {
                    break "items_exhausted";
                };
                let seconds = bundle
                    .projection(&item)
                    .expect("in the bundle")
                    .expected_time;
                if used + f64::from(seconds) > total {
                    break "budget";
                }
                used += f64::from(seconds);
                let minutes = minutes_since(track, started);
                let (a, outcome) =
                    track.answer(learner, &item, slot, AnswerContext::Lesson, minutes);
                r.answered(outcome.correct);
                base.record(bundle, kc, Phase::Block, &a, &outcome);
            }
        }
    });
    #[allow(clippy::cast_precision_loss)] // within a day
    {
        r.seconds = (track.clock - started) as f64 / 1000.0;
    }
    r
}

/// Which planner runs the sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Ours,
    FsrsOnly,
    /// «Только FSRS» sessions with our next-item choice.
    FsrsNextItem,
}

impl Policy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ours => "ours",
            Self::FsrsOnly => "only FSRS",
            Self::FsrsNextItem => "only FSRS + our next_item",
        }
    }
}

/// Sessions of days 1–5 and of days 6 on.
pub const DAY_GROUPS: [&str; 2] = ["days 1-5", "days 6-30"];

/// Size and seed of a cohort run.
#[derive(Debug, Clone, Copy)]
pub struct CohortSpec {
    pub learners: usize,
    pub days: u32,
    pub seed: u64,
    pub budget: Budget,
}

/// The K5 metrics of one policy over one cohort.
#[derive(Debug, Clone)]
pub struct CohortReport {
    pub policy: Policy,
    pub learners: usize,
    pub sessions: u32,
    pub answers: u32,
    pub seconds: f64,
    pub frustrated: u32,
    pub checks: u32,
    pub violations: BTreeMap<String, u32>,
    pub rules: BTreeMap<&'static str, u32>,
    /// Per KC: answers to mastery of each learner who reached it.
    pub to_mastery: BTreeMap<KcId, Vec<u32>>,
    /// Per learner at the end: KCs the model ever counted mastered.
    pub mastered: Vec<usize>,
    /// Per learner at the end: KCs mastered now (mastered or long-term).
    pub mastered_now: Vec<usize>,
    /// True recall at the end, per (learner, KC the model counted mastered).
    pub retention: Vec<f64>,
    /// Per learner at the end: KCs with true recall of at least 0.8,
    /// whatever the model thinks.
    pub known: Vec<usize>,
    /// Session minutes per [`DAY_GROUPS`] group.
    pub minutes: [Vec<f64>; 2],
    /// Sessions per end reason, per [`DAY_GROUPS`] group.
    pub ends: [BTreeMap<&'static str, u32>; 2],
}

/// Runs `spec.learners` learners for `spec.days` daily sessions under
/// `policy`. Learner `i` is the same under both policies.
pub fn cohort(
    model: &KnowledgeModel,
    bundle: &Bundle,
    params: &Params,
    spec: CohortSpec,
    policy: Policy,
) -> CohortReport {
    let mut report = CohortReport {
        policy,
        learners: spec.learners,
        sessions: 0,
        answers: 0,
        seconds: 0.0,
        frustrated: 0,
        checks: 0,
        violations: BTreeMap::new(),
        rules: BTreeMap::new(),
        to_mastery: BTreeMap::new(),
        mastered: Vec::new(),
        mastered_now: Vec::new(),
        retention: Vec::new(),
        known: Vec::new(),
        minutes: [Vec::new(), Vec::new()],
        ends: [BTreeMap::new(), BTreeMap::new()],
    };
    let mut seeds = SplitMix64::new(spec.seed);
    for _ in 0..spec.learners {
        let learner_seed = seeds.next_u64();
        let track_seed = seeds.next_u64();
        let mut learner = Learner::new(bundle, learner_seed);
        let mut track = Track::new(model, bundle, T0, track_seed);
        let mut jitter = SplitMix64::new(track_seed ^ 0x5EED);
        for day in 0..spec.days {
            // Evenings, 18:00 to 20:00.
            let start = T0.0
                + i64::from(day) * DAY_MS
                + 18 * 3_600_000
                + i64::try_from(jitter.next_u64() % 7_200_000).expect("small");
            track.clock = track.clock.max(start);
            let s = match policy {
                Policy::Ours => session_ours(&mut track, params, &mut learner, spec.budget),
                Policy::FsrsOnly => {
                    session_fsrs(&mut track, &mut learner, spec.budget, params, false)
                }
                Policy::FsrsNextItem => {
                    session_fsrs(&mut track, &mut learner, spec.budget, params, true)
                }
            };
            let group = usize::from(day >= 5);
            report.minutes[group].push(s.seconds / 60.0);
            *report.ends[group]
                .entry(s.end.unwrap_or("time_guard"))
                .or_insert(0) += 1;
            report.sessions += 1;
            report.answers += s.answers;
            report.seconds += s.seconds;
            report.frustrated += u32::from(s.frustrated);
            report.checks += s.checks;
            for (c, n) in s.violations {
                *report.violations.entry(c).or_insert(0) += n;
            }
            for (c, n) in s.rules {
                *report.rules.entry(c).or_insert(0) += n;
            }
        }
        let end = track.now();
        for (kc, n) in &track.to_mastery {
            report.to_mastery.entry(kc.clone()).or_default().push(*n);
        }
        let mastered: Vec<&KcId> = track
            .state
            .kcs
            .iter()
            .filter(|(_, k)| k.ever_mastered)
            .map(|(kc, _)| kc)
            .collect();
        report.mastered.push(mastered.len());
        report.mastered_now.push(
            track
                .state
                .kcs
                .keys()
                .filter(|kc| {
                    matches!(
                        model.progress(&track.state, kc, end),
                        KcProgress::Mastered | KcProgress::LongTerm
                    )
                })
                .count(),
        );
        for kc in mastered {
            report.retention.push(learner.recall(kc, end));
        }
        report.known.push(
            bundle
                .content
                .items_by_kc
                .keys()
                .filter(|kc| learner.recall(kc, end) >= 0.8)
                .count(),
        );
    }
    report
}
