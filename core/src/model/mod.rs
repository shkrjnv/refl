//! Layer 2: the learner model (`docs/03-layer2-learner-model.md`).
//!
//! A model is a pure fold over the event log in `event_id` order: the state
//! depends only on the log, the bundle, the learner profile and the model
//! parameters. [`LearnerModel`] lets a shadow model run beside the live one on
//! the same log; [`KnowledgeModel`] is version 1.

pub mod derived;
pub mod fsrs;
pub mod math;
mod state;
mod v1;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::bundle::Bundle;
use crate::event::{AnswerContext, Decoded, EventPayload, Verdict};
use crate::ids::{EventId, ItemRef, MisconceptionId, SubjectId};
use crate::item::LadderLevel;
use crate::value::{Finite, Timestamp};

pub use state::{KcState, MisconceptionState, RapidStats, State};
pub use v1::{Belief, KnowledgeModel, Params};

/// What the model needs to know about the learner that is not in the log yet
/// (`docs/03-layer2-learner-model.md`, open question 9).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Starting level L per subject as a rank of its scale; absent: 0
    /// («начинаю с нуля»).
    pub start_level: BTreeMap<SubjectId, Finite>,
    /// Native language: picks the misconception prior; absent: the mean of uz and ru.
    pub l1: Option<NativeLanguage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeLanguage {
    Uz,
    Ru,
}

/// A `reclassified` event as the fold applies it: the answer it points to is
/// folded in its own place with this verdict (`docs/02-layer1-content.md`,
/// «Переклассификация»).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// The `reclassified` event this correction comes from.
    pub by: EventId,
    pub verdict: Verdict,
    pub misconception: Option<MisconceptionId>,
}

/// The latest correction of every corrected answer in a log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Corrections(BTreeMap<EventId, Correction>);

impl Corrections {
    /// Collects corrections from a log in any order; for an answer corrected
    /// more than once, the `reclassified` with the greatest `event_id` wins.
    pub fn from_log<'a>(events: impl IntoIterator<Item = &'a Decoded>) -> Self {
        let mut out: BTreeMap<EventId, Correction> = BTreeMap::new();
        for ev in events {
            let Decoded::Known(ev) = ev else { continue };
            let EventPayload::Reclassified(r) = &ev.payload else {
                continue;
            };
            let newer = out.get(&r.original).is_none_or(|c| c.by < ev.event_id);
            if newer {
                out.insert(
                    r.original,
                    Correction {
                        by: ev.event_id,
                        verdict: r.verdict,
                        misconception: r.misconception.clone(),
                    },
                );
            }
        }
        Self(out)
    }

    pub fn get(&self, answer: &EventId) -> Option<&Correction> {
        self.0.get(answer)
    }
}

static NO_CORRECTIONS: Corrections = Corrections(BTreeMap::new());

/// What the fold needs besides the state and the event.
///
/// Corrections belong to a replay only. Folding new events as they arrive
/// never needs them: a correction already received is newer than the answer
/// it corrects, so that answer would arrive late and force a replay anyway.
/// Hence a `reclassified` folded with [`FoldCtx::new`] always asks for a
/// replay, and [`replay`] passes the corrections of the whole log.
#[derive(Debug, Clone, Copy)]
pub struct FoldCtx<'a> {
    pub bundle: &'a Bundle,
    pub corrections: &'a Corrections,
}

impl<'a> FoldCtx<'a> {
    /// For folding new events onto a state as they arrive.
    pub fn new(bundle: &'a Bundle) -> Self {
        Self {
            bundle,
            corrections: &NO_CORRECTIONS,
        }
    }
}

/// The event cannot be folded onto this state; the caller recomputes from
/// the whole log with [`replay`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsReplay {
    /// Its `event_id` is below the state's position.
    Late,
    /// A `reclassified` outside a replay: the answer it corrects is already
    /// folded with the old verdict.
    Reclassified,
}

/// The model's own record of one answer, for the K1 metrics
/// (`docs/11-quality.md`): the prediction made before the update.
#[derive(Debug, Clone, PartialEq)]
pub struct AnswerOutcome {
    /// p at display time, with base guessing and slip.
    pub predicted: f64,
    /// Correct for the model: `correct` or `typo` after corrections.
    pub correct: bool,
    /// A rapid guess: changed nothing but the rapid-guess counters.
    pub rapid: bool,
    pub subject: SubjectId,
    pub context: AnswerContext,
    pub ladder: LadderLevel,
}

/// What [`LearnerModel::apply`] did with an event.
#[derive(Debug, Clone, PartialEq)]
pub enum Applied {
    Answer(AnswerOutcome),
    /// A known event the model does not use (e.g. `plan_issued`).
    Skipped,
    /// The same `event_id` as the position: already folded.
    Duplicate,
    /// Written by a newer core: skipped, state marked incomplete
    /// (`docs/13-sync-versions.md`, «Неизвестные события»).
    Unknown,
    /// An answer whose item version or slot has no projection in the bundle:
    /// skipped and counted. Only a corrupt bundle gets here, as it carries
    /// the projection of every published version
    /// (`docs/02-layer1-content.md`, «Идентификаторы и версии»).
    Unresolved,
}

/// A learner model: one state type, a prediction and an event step
/// (`docs/03-layer2-learner-model.md`, «Реализация»).
pub trait LearnerModel {
    type State: Clone + Serialize + DeserializeOwned;

    /// Model name and a hash of its parameters: a new value means a full
    /// recompute, never a silent drift.
    fn version(&self) -> String;

    fn init(&self, profile: &Profile) -> Self::State;

    /// Probability of a correct answer to `slot` of `item` at `now`; `None`
    /// if the bundle has no such item version or slot. The fold, the planner
    /// and the calibration metrics all use this one function.
    fn predict(
        &self,
        s: &Self::State,
        bundle: &Bundle,
        item: &ItemRef,
        slot: u16,
        now: Timestamp,
    ) -> Option<f64>;

    /// Folds one event in `event_id` order.
    ///
    /// # Errors
    /// [`NeedsReplay`] if the event cannot be folded onto this state; the
    /// state is then unchanged.
    fn apply(
        &self,
        s: &mut Self::State,
        ev: &Decoded,
        ctx: &FoldCtx<'_>,
    ) -> Result<Applied, NeedsReplay>;

    /// `fold(state, event) → state`.
    ///
    /// # Errors
    /// As [`LearnerModel::apply`].
    fn fold(
        &self,
        mut s: Self::State,
        ev: &Decoded,
        ctx: &FoldCtx<'_>,
    ) -> Result<Self::State, NeedsReplay> {
        self.apply(&mut s, ev, ctx)?;
        Ok(s)
    }
}

/// `event_id` of a decoded event, known or not.
pub fn event_id(ev: &Decoded) -> &EventId {
    match ev {
        Decoded::Known(e) => &e.event_id,
        Decoded::Unknown(u) => &u.event_id,
    }
}

/// The state of a whole log: events sorted by `event_id` (the input may be in
/// any order; duplicates fold once), every answer with its latest correction.
///
/// # Panics
/// Never for a correct model: a sorted log folded with all its corrections
/// has no late events and no unapplied `reclassified`.
pub fn replay<M: LearnerModel>(
    model: &M,
    profile: &Profile,
    bundle: &Bundle,
    events: &[Decoded],
) -> M::State {
    let mut order: Vec<&Decoded> = events.iter().collect();
    order.sort_by(|a, b| event_id(a).cmp(event_id(b)));
    let corrections = Corrections::from_log(order.iter().copied());
    let ctx = FoldCtx {
        bundle,
        corrections: &corrections,
    };
    let mut state = model.init(profile);
    for ev in order {
        model
            .apply(&mut state, ev, &ctx)
            .expect("a sorted log with all its corrections never needs a replay");
    }
    state
}
