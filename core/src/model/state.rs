//! State of the version-1 learner model. Only ordered collections, so equal
//! logs give equal bytes; the canonical hash compares native and WASM folds.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::Profile;
use super::fsrs::Card;
use crate::ids::{EventId, ItemId, KcId, MisconceptionId, SubjectId};
use crate::value::Timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub profile: Profile,
    /// The greatest folded `event_id`.
    pub position: Option<EventId>,
    /// An event from a newer core was skipped (`docs/13-sync-versions.md`).
    pub incomplete: bool,
    /// Answers skipped because the bundle lacks their item version or slot.
    pub unresolved: u32,
    /// KCs with any update; a KC not here holds its starting belief.
    pub kcs: BTreeMap<KcId, KcState>,
    /// KCs whose explanation the learner has viewed: their memory may start
    /// in a lesson.
    pub explained: BTreeSet<KcId>,
    pub misconceptions: BTreeMap<MisconceptionId, MisconceptionState>,
    /// Time of the last answer per subject: the break threshold.
    pub last_answer: BTreeMap<SubjectId, Timestamp>,
    pub rapid: RapidStats,
}

/// One KC: knowledge, memory, automaticity and the mastery counters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcState {
    /// μ in logits.
    pub mu: f64,
    /// σ² at `var_at`; it grows by η per day after that.
    pub var: f64,
    pub var_at: Timestamp,
    /// Direct observations with weight > 0 (not propagated ones).
    pub observations: u32,
    /// FSRS memory; `None` until the first success that may start it.
    pub card: Option<Card>,
    /// τ: how much faster than expected, ln(expected / actual), smoothed.
    pub tau: f64,
    /// Correct answers whose time was used for τ.
    pub timed: u32,
    /// Distinct production items answered correctly, at most two.
    pub production: Vec<ItemId>,
    /// The mastery criterion held at some event.
    pub ever_mastered: bool,
}

/// Beta belief about one misconception.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MisconceptionState {
    pub alpha: f64,
    pub beta: f64,
    /// Start from the content prior; decay pulls α and β back to it.
    pub alpha0: f64,
    pub beta0: f64,
    pub updated_at: Timestamp,
    /// Real manifestations, without decay.
    pub manifested: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapidStats {
    pub total: u32,
    /// Rapid guesses in a row up to the last answer.
    pub streak: u32,
}

impl State {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            position: None,
            incomplete: false,
            unresolved: 0,
            kcs: BTreeMap::new(),
            explained: BTreeSet::new(),
            misconceptions: BTreeMap::new(),
            last_answer: BTreeMap::new(),
            rapid: RapidStats::default(),
        }
    }

    /// Canonical bytes: the JSON serialization. Floats print as their
    /// shortest round-trip form, so equal bits give equal bytes.
    ///
    /// # Panics
    /// Never: the state holds only finite floats, strings and maps with
    /// string keys.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("learner state serializes")
    }

    /// SHA-256 of [`State::canonical_bytes`], hex.
    ///
    /// # Panics
    /// As [`State::canonical_bytes`].
    pub fn hash(&self) -> String {
        Sha256::digest(self.canonical_bytes())
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            })
    }
}
