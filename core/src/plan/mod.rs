//! Layer 4: the session builder of the prototype
//! (`docs/04-placement-planner.md`, «Сборка сессии в прототипе»).
//!
//! A plan is the session's answers so far and the steps still ahead; the
//! steps are a pure function of the answers, the learner state and the
//! bundle, recomputed after every answer. Every random choice draws from the
//! core PRNG with a seed that comes in as an argument and goes into the
//! `plan_issued` event of that decision.

mod adapt;
mod build;
mod issue;
mod next;
mod validate;
mod values;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bundle::Bundle;
use crate::event::{
    AnswerContext, AnswerEvent, Confidence, Decoded, EventPayload, PlanAction, PlanIssuedEvent,
};
use crate::ids::{ExplanationId, FamilyId, ItemRef, KcId};
use crate::item::ItemKindTag;
use crate::model::{AnswerOutcome, KnowledgeModel};
use crate::value::{Lang, Seed, Timestamp};

pub use validate::Violation;
pub use values::{LearnValue, ReviewValue};

/// Version of the session rules; part of every `plan_issued`.
const PLANNER_NAME: &str = "session-prototype-4";

/// Parameters of the prototype session builder with their starting values
/// (`docs/04-placement-planner.md`, «Сборка сессии в прототипе»). They are
/// part of the planner version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// Reviews take at most this share of the budget.
    pub review_share: f64,
    /// The final block takes at most this share of the budget.
    pub final_share: f64,
    /// r*: a KC with a card is due when R falls below this.
    pub target_recall: f64,
    /// Δ of V^rev: the time to the next session, days.
    pub review_horizon_days: f64,
    /// A KC with Ready below this is blocked.
    pub ready_min: f64,
    /// G: weight of the learner's goal; 1 in the prototype.
    pub goal_weight: f64,
    /// ζ: weight of unlocking in V^learn.
    pub zeta: f64,
    /// Micro-randomisation of the learning KC: candidates within this share
    /// of the best V^learn.
    pub tie_share: f64,
    /// ε: next-item candidates within this of the best score, in |p − p*|
    /// units.
    pub tie_epsilon: f64,
    /// Expert prior of explanation time: words read per minute.
    pub reading_words_per_minute: f64,
    /// n₀: views the prior of explanation time weighs as.
    pub explanation_n0: f64,
    /// Planned length of the block.
    pub block_items: u32,
    /// Correct answers that close the block, a correct production among them.
    pub close_correct: u32,
    /// Items on the learning KC without closing the block before it is deferred.
    pub kc_cap: u32,
    /// Learning KCs a session may have: the first and one opened by rule 6.
    pub learn_max: usize,
    /// At most this many steps in a row on one KC after the block, while
    /// anything else can get a step.
    pub max_run: usize,
    /// Interleaved steps a learning KC gets in a session when it goes on
    /// alone, nothing else being left.
    pub alone_cap: u32,
    /// p* of the block, the interleaved part and the final, a prerequisite check.
    pub p_block: f64,
    pub p_interleaved: f64,
    pub p_prerequisite: f64,
    /// ν: penalty for a family seen within `seen_days`.
    pub nu: f64,
    pub seen_days: f64,
    /// `κ_I`: weight of the information per second.
    pub kappa_info: f64,
    /// M below this: recognition kinds; at or above: guided production.
    pub ladder_mastery: f64,
    /// Errors in a row on one KC that start a prerequisite check.
    pub errors_in_row: usize,
    /// A prerequisite is checked only while its suspicion D is above this.
    pub suspicion_min: f64,
    /// Rapid guesses in a row that suggest a break.
    pub rapid_streak: u32,
    /// At least this many next-item candidates go into `plan_issued`.
    pub logged_candidates: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            review_share: 0.35,
            final_share: 0.10,
            target_recall: 0.9,
            review_horizon_days: 1.0,
            ready_min: 0.5,
            goal_weight: 1.0,
            zeta: 0.3,
            tie_share: 0.95,
            tie_epsilon: 0.05,
            reading_words_per_minute: 100.0,
            explanation_n0: 5.0,
            block_items: 4,
            close_correct: 3,
            kc_cap: 12,
            learn_max: 2,
            max_run: 3,
            alone_cap: 12,
            p_block: 0.80,
            p_interleaved: 0.70,
            p_prerequisite: 0.55,
            nu: 0.3,
            seen_days: 3.0,
            kappa_info: 0.1,
            ladder_mastery: 0.3,
            errors_in_row: 2,
            suspicion_min: 0.5,
            rapid_streak: 3,
            logged_candidates: 10,
        }
    }
}

/// Session length the learner picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Budget {
    Ten,
    Fifteen,
    /// One pomodoro.
    #[default]
    TwentyFive,
}

impl Budget {
    pub fn seconds(self) -> u32 {
        match self {
            Self::Ten => 600,
            Self::Fifteen => 900,
            Self::TwentyFive => 1500,
        }
    }
}

/// What a step is for; it sets p* and which rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// The explanation of the learning KC; no item.
    Lesson,
    Block,
    Interleaved,
    Review,
    Prerequisite,
    Final,
    /// A mastery-audit probe (`docs/11-quality.md`, «К4»): chosen by the
    /// audit rule, not by value.
    Audit,
    /// A check after a break (`docs/04-placement-planner.md`).
    ReturnCheck,
}

impl Phase {
    /// Steps of the learning KC and its contrasts, the ones a blocked KC
    /// must not get.
    fn is_learning(self) -> bool {
        matches!(self, Self::Lesson | Self::Block | Self::Interleaved)
    }

    /// Counted by the «no more than three in a row» rule. The final has its
    /// own 10% budget and is explicitly outside the run limit, even when it
    /// reviews a KC that was just practised (`docs/04`, «Сборка сессии в
    /// прототипе»).
    fn counts_in_runs(self) -> bool {
        !matches!(
            self,
            Self::Lesson | Self::Block | Self::Final | Self::Audit | Self::ReturnCheck
        )
    }

    /// A probe is chosen for measurement, not by the value formula.
    pub fn is_probe(self) -> bool {
        matches!(self, Self::Audit | Self::ReturnCheck)
    }

    /// The `answer.context` the runner writes when this step is answered.
    pub fn context(self) -> AnswerContext {
        match self {
            Self::Lesson | Self::Block => AnswerContext::Lesson,
            Self::Interleaved | Self::Prerequisite | Self::Final => AnswerContext::Practice,
            Self::Review => AnswerContext::Review,
            Self::Audit => AnswerContext::Audit,
            Self::ReturnCheck => AnswerContext::ReturnCheck,
        }
    }
}

/// One item on one KC, or the lesson; the item is picked when shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub kc: KcId,
    pub phase: Phase,
    /// Only production kinds, whatever the ladder.
    pub production: bool,
    /// The family a final step draws from.
    pub family: Option<FamilyId>,
    /// Estimated seconds: the largest expected time the step can draw for
    /// reviews and the final, the mean for the rest, 0 for the lesson.
    pub seconds: u32,
}

/// The item a step was answered with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    pub item: ItemRef,
    pub slot: u16,
    pub family: FamilyId,
    pub kind: ItemKindTag,
    /// The item's expected time.
    pub seconds: u32,
    /// Correct for the model.
    pub correct: bool,
    pub rapid: bool,
    /// Answered wrong with confidence «sure».
    pub sure_error: bool,
}

/// A step of this session already done, with the rule it fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Done {
    pub step: Step,
    /// `None` for the lesson.
    pub shown: Option<Shown>,
    pub rule: Option<Rule>,
}

/// An adaptation rule (`docs/04-placement-planner.md`, «Сборка сессии в
/// прототипе», «Адаптация»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Rule {
    BlockClosed,
    KcDeferred,
    PrerequisiteCheck { kc: KcId },
    ConfidentError,
    RapidGuesses,
}

impl Rule {
    /// The `trigger` code in `plan_issued`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BlockClosed => "block_closed",
            Self::KcDeferred => "kc_deferred",
            Self::PrerequisiteCheck { .. } => "prerequisite_check",
            Self::ConfidentError => "confident_error",
            Self::RapidGuesses => "rapid_guesses",
        }
    }
}

/// `trigger` of the `plan_issued` that opens a second learning KC
/// (`docs/04-placement-planner.md`, «Адаптация», rule 6). It is decided in
/// the replan, not by the last answer, so it is not a [`Rule`].
pub const SECOND_KC: &str = "second_kc";

/// A learning KC of a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Learn {
    pub kc: KcId,
    /// KCs it is interleaved with, sorted.
    pub contrasts: Vec<KcId>,
    /// The learner has not viewed its explanation: the session starts with it.
    pub lesson: bool,
}

/// A review chosen when the session was built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedReview {
    pub kc: KcId,
    pub seconds: u32,
}

/// A session plan: what was fixed at the start, what is done, what is ahead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub budget: Budget,
    pub started: Timestamp,
    /// Seed of the session decision.
    pub seed: Seed,
    /// Log-derived family recency and deferrals, kept with the plan so audit
    /// probes can require a family unseen for 30 days after every replan
    /// without another argument to `adapt`.
    #[serde(default)]
    pub history: History,
    /// The session started after a break; its return check is fixed for the
    /// whole session even after the first answer moves `last_answer`.
    #[serde(default)]
    pub return_check: bool,
    /// At most `learn_max`: the one picked at the start, then one opened by
    /// rule 6.
    pub learn: Vec<Learn>,
    /// The learning candidates at the start, by V^learn: the pool of the
    /// started KCs interleaved.
    pub candidates: Vec<KcId>,
    /// KCs deferred by the 12-items rule earlier on the UTC day the session
    /// started, read from the log: rule 6 does not open them.
    pub deferred_today: Vec<KcId>,
    /// In V^rev order.
    pub reviews: Vec<PlannedReview>,
    pub done: Vec<Done>,
    /// Recomputed after every answer.
    pub steps: Vec<Step>,
    /// The rapid-guess rule fired in this session.
    pub break_suggested: bool,
    /// The interface language: the explanation a lesson shows.
    pub lang: Lang,
    /// Why the session ended; set when no step is left.
    pub end: Option<End>,
}

/// Why a session ended (`docs/04-placement-planner.md`, «Конец сессии»).
/// T is an upper bound, not a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum End {
    /// The next step of the main part does not fit the time left.
    Budget,
    /// Neither reviews nor a learning KC.
    NothingDue,
    /// No learning KC, and the reviews are done.
    ReviewsDone,
    /// The learning KC went on alone and had its interleaved steps of the
    /// session; nothing else is left and no second learning KC can open.
    KcCap,
    /// The learning KCs were mastered in the session; nothing else is left
    /// and no second learning KC can open.
    KcMastered,
    /// The learning KC was deferred, or its Ready fell below the threshold.
    KcDeferred,
    /// An otherwise eligible step was rejected by the run rule, and no
    /// other valid step remained.
    RunLimit,
    /// The steps left have no unseen item.
    ItemsExhausted,
}

impl End {
    pub fn code(self) -> &'static str {
        match self {
            Self::Budget => "budget",
            Self::NothingDue => "nothing_due",
            Self::ReviewsDone => "reviews_done",
            Self::KcCap => "kc_cap",
            Self::KcMastered => "kc_mastered",
            Self::KcDeferred => "kc_deferred",
            Self::RunLimit => "run_limit",
            Self::ItemsExhausted => "items_exhausted",
        }
    }
}

impl Plan {
    /// The items shown in this session.
    fn shown(&self) -> impl Iterator<Item = (&Step, &Shown)> {
        self.done
            .iter()
            .filter_map(|d| d.shown.as_ref().map(|s| (&d.step, s)))
    }

    /// The block of learning KC `kc` is closed.
    fn block_closed(&self, kc: &KcId) -> bool {
        self.fired_on(kc, &Rule::BlockClosed)
    }

    /// Learning KC `kc` was deferred to the next session.
    fn deferred(&self, kc: &KcId) -> bool {
        self.fired_on(kc, &Rule::KcDeferred)
    }

    fn fired_on(&self, kc: &KcId, rule: &Rule) -> bool {
        self.done
            .iter()
            .any(|d| &d.step.kc == kc && d.rule.as_ref() == Some(rule))
    }

    /// `kc` is a learning KC of this session.
    fn is_learn(&self, kc: &KcId) -> bool {
        self.learn.iter().any(|l| &l.kc == kc)
    }

    /// Interleaved steps done on `kc` in this session.
    fn interleaved_done(&self, kc: &KcId) -> u32 {
        let n = self
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::Interleaved && &d.step.kc == kc)
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    }
}

/// A plan and the `plan_issued` of its decision; `None` for an empty plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub plan: Plan,
    pub issued: Option<PlanIssuedEvent>,
}

/// What to show next.
#[derive(Debug, Clone, PartialEq)]
pub enum Next {
    /// The explanation of this KC, the record the learner's language gets;
    /// then [`Planner::adapt`] with [`Outcome::Lesson`].
    Lesson {
        kc: KcId,
        explanation: ExplanationId,
    },
    Item(Choice),
    /// No steps left, and why.
    Finished {
        end: End,
    },
}

/// The chosen item and the `plan_issued` of the choice.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub item: ItemRef,
    pub slot: u16,
    pub issued: PlanIssuedEvent,
    /// The step's answer context: lesson, review, audit, return check, …
    pub context: AnswerContext,
}

/// An answer as the planner reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    pub item: ItemRef,
    pub slot: u16,
    /// Correct for the model (`correct` or `typo`).
    pub correct: bool,
    /// A rapid guess (`docs/03-layer2-learner-model.md`, «Черты ученика»).
    pub rapid: bool,
    /// Answered with confidence «sure».
    pub sure: bool,
}

impl Answered {
    /// From the answer event and what the fold reported for it.
    pub fn new(answer: &AnswerEvent, outcome: &AnswerOutcome) -> Self {
        Self {
            item: answer.item.clone(),
            slot: answer.slot,
            correct: outcome.correct,
            rapid: outcome.rapid,
            sure: answer.confidence == Some(Confidence::Sure),
        }
    }
}

/// What happened to the first step of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The learner viewed the explanation.
    Lesson,
    Answer(Answered),
}

/// The plan after an outcome and its `plan_issued`, in order: one for a
/// rule that changed it, one for a second learning KC it opened, and the
/// reason it ended on the last of them (or on its own event).
#[derive(Debug, Clone, PartialEq)]
pub struct Adapted {
    pub plan: Plan,
    pub rule: Option<Rule>,
    pub issued: Vec<PlanIssuedEvent>,
}

/// What the planner reads from the learner's log before the session: when
/// each item family was last shown (the «seen» of the next-item score) and
/// when each KC was last deferred by the 12-items rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    families: BTreeMap<FamilyId, Timestamp>,
    deferred: BTreeMap<KcId, Timestamp>,
}

impl History {
    /// Takes one event of the log, in any order: an answer records its
    /// family, a `plan_issued` of the deferral rule its chosen KC. Answers
    /// whose item version the bundle lacks are skipped.
    pub fn record(&mut self, bundle: &Bundle, ev: &Decoded) {
        let Decoded::Known(e) = ev else {
            return;
        };
        match &e.payload {
            EventPayload::Answer(a) => {
                if let Some(p) = bundle.projection(&a.item) {
                    latest(&mut self.families, &p.family, e.client_ts);
                }
            }
            EventPayload::PlanIssued(p)
                if p.trigger.as_deref() == Some(Rule::KcDeferred.code()) =>
            {
                if let Some(PlanAction::Learn { kc }) =
                    p.candidates.get(usize::from(p.chosen)).map(|c| &c.action)
                {
                    latest(&mut self.deferred, kc, e.client_ts);
                }
            }
            _ => {}
        }
    }

    pub fn from_log<'a>(bundle: &Bundle, events: impl IntoIterator<Item = &'a Decoded>) -> Self {
        let mut history = Self::default();
        for ev in events {
            history.record(bundle, ev);
        }
        history
    }

    /// When `family` was last shown.
    pub fn last_shown(&self, family: &FamilyId) -> Option<Timestamp> {
        self.families.get(family).copied()
    }

    /// When `kc` was last deferred.
    pub fn last_deferred(&self, kc: &KcId) -> Option<Timestamp> {
        self.deferred.get(kc).copied()
    }
}

fn latest<K: Ord + Clone>(map: &mut BTreeMap<K, Timestamp>, key: &K, at: Timestamp) {
    let t = map.entry(key.clone()).or_insert(at);
    *t = (*t).max(at);
}

/// How fast this learner reads explanations, against the expert prior of
/// 100 words a minute (`docs/04-placement-planner.md`, «Время объяснения»):
/// the median ratio of actual to expected seconds over their
/// `explanation_viewed` events, pulled towards 1 with weight `n₀`.
/// A per-explanation correction waits for population statistics; when it
/// comes it is versioned calibration data and its value goes into
/// `plan_issued`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReadingSpeed {
    /// Multiplies the expected seconds; 1 without views.
    pub factor: f64,
    /// Views the factor is made of.
    pub views: u32,
}

impl Default for ReadingSpeed {
    fn default() -> Self {
        Self {
            factor: 1.0,
            views: 0,
        }
    }
}

impl ReadingSpeed {
    /// From the `explanation_viewed` events of a learner's log, in any
    /// order. Views of explanations the bundle lacks are skipped.
    pub fn from_log<'a>(
        bundle: &Bundle,
        params: &Params,
        events: impl IntoIterator<Item = &'a Decoded>,
    ) -> Self {
        Self::from_views(
            bundle,
            params,
            events.into_iter().filter_map(|ev| match ev {
                Decoded::Known(e) => match &e.payload {
                    EventPayload::ExplanationViewed {
                        explanation,
                        seconds,
                        ..
                    } => Some((explanation, *seconds)),
                    _ => None,
                },
                Decoded::Unknown(_) => None,
            }),
        )
    }

    /// From (explanation, seconds) views, in any order.
    #[allow(clippy::cast_precision_loss)] // view counts are small
    pub fn from_views<'a>(
        bundle: &Bundle,
        params: &Params,
        views: impl IntoIterator<Item = (&'a ExplanationId, u32)>,
    ) -> Self {
        let mut ratios: Vec<f64> = views
            .into_iter()
            .filter_map(|(id, seconds)| {
                let e = bundle.content.explanations.iter().find(|e| &e.id == id)?;
                let expected = prior_seconds(params, e);
                (expected > 0.0).then(|| f64::from(seconds) / expected)
            })
            .collect();
        if ratios.is_empty() {
            return Self::default();
        }
        ratios.sort_by(f64::total_cmp);
        let n = ratios.len();
        let median = if n % 2 == 1 {
            ratios[n / 2]
        } else {
            f64::midpoint(ratios[n / 2 - 1], ratios[n / 2])
        };
        let weight = n as f64;
        Self {
            factor: (params.explanation_n0 + weight * median) / (params.explanation_n0 + weight),
            views: u32::try_from(n).unwrap_or(u32::MAX),
        }
    }
}

/// Expected seconds of reading `e` for the average reader: the words of its
/// body, native-language block and examples at the expert reading speed.
#[allow(clippy::cast_precision_loss)] // word counts are small
fn prior_seconds(params: &Params, e: &crate::content::Explanation) -> f64 {
    let words = e.body.split_whitespace().count()
        + e.l1_contrast
            .as_deref()
            .map_or(0, |t| t.split_whitespace().count())
        + e.examples
            .iter()
            .map(|x| x.text.split_whitespace().count())
            .sum::<usize>();
    words as f64 * 60.0 / params.reading_words_per_minute
}

/// The prototype session builder over learner model v1 and one bundle.
#[derive(Debug, Clone, Copy)]
pub struct Planner<'a> {
    pub model: &'a KnowledgeModel,
    pub bundle: &'a Bundle,
    pub params: &'a Params,
    /// This learner's reading speed, for the time of a lesson.
    pub speed: &'a ReadingSpeed,
}

impl<'a> Planner<'a> {
    pub fn new(
        model: &'a KnowledgeModel,
        bundle: &'a Bundle,
        params: &'a Params,
        speed: &'a ReadingSpeed,
    ) -> Self {
        Self {
            model,
            bundle,
            params,
            speed,
        }
    }

    /// Rules name and a hash of the parameters: a new value is a new planner.
    ///
    /// # Panics
    /// Never: the parameters always serialize.
    pub fn version(&self) -> String {
        let params = serde_json::to_vec(self.params).expect("params serialize");
        let hash = Sha256::digest(params);
        let short = hash[..8].iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        });
        format!("{PLANNER_NAME}+{short}")
    }
}

/// The seed of the second learning KC's draw: the second output of
/// `SplitMix64(seed)`, 53 bits; the first one drew the first learning KC.
///
/// # Panics
/// Never: 53 bits are a valid seed.
fn second_seed(seed: Seed) -> Seed {
    let mut rng = crate::rng::SplitMix64::new(seed.get());
    rng.next_u64();
    Seed::new(rng.next_u64() >> 11).expect("53 bits")
}

/// The index `⌊u·n⌋` among `n` admissible candidates, u the first uniform of
/// `SplitMix64(seed)`, and its propensity `1/n`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // n is a small count; u·n < n
fn draw(seed: Seed, n: usize) -> (usize, f64) {
    let u = crate::rng::SplitMix64::new(seed.get()).next_f64();
    (((u * n as f64) as usize).min(n - 1), 1.0 / n as f64)
}
