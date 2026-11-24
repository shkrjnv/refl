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
use crate::event::{AnswerEvent, Confidence, Decoded, EventPayload, PlanIssuedEvent};
use crate::ids::{FamilyId, ItemRef, KcId};
use crate::item::ItemKindTag;
use crate::model::{AnswerOutcome, KnowledgeModel};
use crate::value::{Seed, Timestamp};

pub use validate::Violation;
pub use values::{LearnValue, ReviewValue};

/// Version of the session rules; part of every `plan_issued`.
const PLANNER_NAME: &str = "session-prototype-1";

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
    /// Micro-randomisation: candidates within this share of the best.
    pub tie_share: f64,
    /// Planned length of the block.
    pub block_items: u32,
    /// Correct answers that close the block, a correct production among them.
    pub close_correct: u32,
    /// Items on the learning KC without closing the block before it is deferred.
    pub kc_cap: u32,
    /// At most this many steps in a row on one KC after the block.
    pub max_run: usize,
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
            block_items: 4,
            close_correct: 3,
            kc_cap: 12,
            max_run: 3,
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
}

impl Phase {
    /// Steps of the learning KC and its contrasts, the ones a blocked KC
    /// must not get.
    fn is_learning(self) -> bool {
        matches!(self, Self::Lesson | Self::Block | Self::Interleaved)
    }

    /// Counted by the «no more than three in a row» rule.
    fn counts_in_runs(self) -> bool {
        !matches!(self, Self::Lesson | Self::Block)
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

/// The learning KC of a session.
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
    pub learn: Option<Learn>,
    /// In V^rev order.
    pub reviews: Vec<PlannedReview>,
    pub done: Vec<Done>,
    /// Recomputed after every answer.
    pub steps: Vec<Step>,
    /// The rapid-guess rule fired in this session.
    pub break_suggested: bool,
}

impl Plan {
    /// The items shown in this session.
    fn shown(&self) -> impl Iterator<Item = (&Step, &Shown)> {
        self.done
            .iter()
            .filter_map(|d| d.shown.as_ref().map(|s| (&d.step, s)))
    }

    /// The learning KC's block is closed.
    fn block_closed(&self) -> bool {
        self.done.iter().any(|d| d.rule == Some(Rule::BlockClosed))
    }

    /// The learning KC was deferred to the next session.
    fn deferred(&self) -> bool {
        self.done.iter().any(|d| d.rule == Some(Rule::KcDeferred))
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
    /// The explanation of this KC; then [`Planner::adapt`] with
    /// [`Outcome::Lesson`].
    Lesson {
        kc: KcId,
    },
    Item(Choice),
    /// No steps left.
    Finished,
}

/// The chosen item and the `plan_issued` of the choice.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub item: ItemRef,
    pub slot: u16,
    pub issued: PlanIssuedEvent,
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

/// The plan after an outcome; `issued` when a rule changed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Adapted {
    pub plan: Plan,
    pub rule: Option<Rule>,
    pub issued: Option<PlanIssuedEvent>,
}

/// When each item family was last shown, from the log before the session:
/// the «seen» of the next-item score.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen(BTreeMap<FamilyId, Timestamp>);

impl Seen {
    /// Records a showing of `family` at `at`.
    pub fn record(&mut self, family: &FamilyId, at: Timestamp) {
        let t = self.0.entry(family.clone()).or_insert(at);
        *t = (*t).max(at);
    }

    /// From the answers of a log, in any order; answers whose item version
    /// the bundle lacks are skipped.
    pub fn from_log<'a>(bundle: &Bundle, events: impl IntoIterator<Item = &'a Decoded>) -> Self {
        let mut seen = Self::default();
        for ev in events {
            if let Decoded::Known(e) = ev
                && let EventPayload::Answer(a) = &e.payload
                && let Some(p) = bundle.projection(&a.item)
            {
                seen.record(&p.family, e.client_ts);
            }
        }
        seen
    }

    pub fn last(&self, family: &FamilyId) -> Option<Timestamp> {
        self.0.get(family).copied()
    }
}

/// The prototype session builder over learner model v1 and one bundle.
#[derive(Debug, Clone, Copy)]
pub struct Planner<'a> {
    pub model: &'a KnowledgeModel,
    pub bundle: &'a Bundle,
    pub params: &'a Params,
}

impl<'a> Planner<'a> {
    pub fn new(model: &'a KnowledgeModel, bundle: &'a Bundle, params: &'a Params) -> Self {
        Self {
            model,
            bundle,
            params,
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
