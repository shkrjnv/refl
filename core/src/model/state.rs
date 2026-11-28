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
use crate::event::{
    BookMarkKind, CustomLexemeContext, CustomLexemeSource, InputDevice, NoteAnchor,
};
use crate::ids::{
    BookId, BookMarkId, CustomLexemeId, EventId, FamilyId, InstructionId, ItemId, KcId, LexemeId,
    MisconceptionId, NoteId, SessionId, SubjectId,
};
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
    /// When each KC first crossed the mastery criterion. Audit probes use
    /// the 7/30/90-day milestones (`docs/11-quality.md`, «К4»); a state
    /// written before this field existed simply has no dates yet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mastered_at: BTreeMap<KcId, Timestamp>,
    /// The session of the latest answer and the instructions shown in it
    /// (`docs/03-layer2-learner-model.md`, «Беглость»).
    pub session: Option<SessionId>,
    pub session_instructions: BTreeSet<InstructionId>,
    /// Answers whose parts of time exceed the whole: `timing-inconsistent`
    /// of the quality report (`docs/11-quality.md`).
    pub timing_inconsistent: u32,
    /// Display moment of the latest answer per item, any version, kept for
    /// the item window only.
    pub seen: BTreeMap<ItemId, Timestamp>,
    /// Typing speeds of the latest counted typed answers per input device,
    /// characters per second, oldest first.
    pub typing: BTreeMap<InputDevice, Vec<f64>>,
    /// bⱼ per subject: residuals of counted answers on known KCs.
    pub baseline: BTreeMap<SubjectId, Baseline>,
    /// Lexical FSRS cards, separate from the KC graph. Nested rather than
    /// keyed by a tuple because JSON map keys must be strings.
    #[serde(default)]
    pub lexical_cards: BTreeMap<LexemeId, BTreeMap<LexicalDirection, LexicalCard>>,
    /// Passive exposure to lexical items in correct grammar answers.
    #[serde(default)]
    pub lexical_exposures: BTreeMap<LexemeId, LexicalExposure>,
    /// Learner-authored words and their archived/tombstone state.
    #[serde(default)]
    pub custom_lexemes: BTreeMap<CustomLexemeId, CustomLexemeRecord>,
    /// Anchored Markdown notes, with tombstones.
    #[serde(default)]
    pub notes: BTreeMap<NoteId, NoteRecord>,
    /// Book highlights and bookmarks, with tombstones.
    #[serde(default)]
    pub book_marks: BTreeMap<BookMarkId, BookMarkRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LexicalDirection {
    Recognize,
    Produce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LexicalSource {
    Bundle,
    Custom,
}

/// FSRS-6 state of one word sense in one direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexicalCard {
    pub stability: f64,
    pub difficulty: f64,
    pub last_review: Timestamp,
    /// When the card is next due; `last_review + stability` for now.
    pub due: Timestamp,
    pub direction: LexicalDirection,
    pub source: LexicalSource,
    /// Passive meetings in correct grammar items, 0.15 each
    /// (`docs/16-vocabulary-library-notes-graph.md`, §1).
    #[serde(default)]
    pub exposures: f64,
    pub review_count: u32,
    pub lapse_count: u32,
}

/// Passive encounters of a lexeme in solved grammar items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalExposure {
    pub count: u32,
    pub items: BTreeSet<ItemId>,
    pub families: BTreeSet<FamilyId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomLexemeRecord {
    pub custom_id: CustomLexemeId,
    pub source: CustomLexemeSource,
    pub bundle_lexeme_id: Option<LexemeId>,
    pub lemma: String,
    pub pos: Option<String>,
    pub definition_en: String,
    pub gloss_uz: String,
    pub gloss_ru: String,
    pub examples: Vec<String>,
    pub collocations: Vec<String>,
    pub ipa: Option<String>,
    pub audio_key: Option<String>,
    pub context: Option<CustomLexemeContext>,
    pub archived: bool,
    pub event_id: EventId,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRecord {
    pub note_id: NoteId,
    pub anchor: NoteAnchor,
    pub body: String,
    pub deleted: bool,
    pub event_id: EventId,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookMarkRecord {
    pub mark_id: BookMarkId,
    pub book_id: BookId,
    pub book_version: String,
    pub kind: BookMarkKind,
    pub chapter_id: String,
    pub paragraph_id: String,
    pub sentence_id: String,
    pub start_token: u64,
    pub end_token: u64,
    pub quote_hash: String,
    pub color: Option<String>,
    pub deleted: bool,
    pub event_id: EventId,
    pub updated_at: Timestamp,
}

/// Sum and count of the residuals behind a learner's baseline speed bⱼ.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub sum: f64,
    pub n: u32,
}

/// One KC: knowledge, memory, fluency and the mastery counters.
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
    /// τ: how much faster than the family's βᶠ the learner retrieves,
    /// residuals `r = βᶠ − ln t_ret`, smoothed.
    pub tau: f64,
    /// Counted answers, those that updated τ.
    pub counted: u32,
    /// r of the latest counted answers, oldest first: the spread.
    pub residuals: Vec<f64>,
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
            mastered_at: BTreeMap::new(),
            session: None,
            session_instructions: BTreeSet::new(),
            timing_inconsistent: 0,
            seen: BTreeMap::new(),
            typing: BTreeMap::new(),
            baseline: BTreeMap::new(),
            lexical_cards: BTreeMap::new(),
            lexical_exposures: BTreeMap::new(),
            custom_lexemes: BTreeMap::new(),
            notes: BTreeMap::new(),
            book_marks: BTreeMap::new(),
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

    /// The FSRS card of one sense and direction, if it exists.
    pub fn lexical_card(
        &self,
        lexeme: &LexemeId,
        direction: LexicalDirection,
    ) -> Option<&LexicalCard> {
        self.lexical_cards
            .get(lexeme)
            .and_then(|cards| cards.get(&direction))
    }

    pub fn has_lexical_card(&self, lexeme: &LexemeId, direction: LexicalDirection) -> bool {
        self.lexical_card(lexeme, direction).is_some()
    }

    /// Stores a card under its lexeme and direction, returned from the card.
    pub fn insert_lexical_card(&mut self, lexeme: &LexemeId, card: LexicalCard) {
        self.lexical_cards
            .entry(lexeme.clone())
            .or_default()
            .insert(card.direction, card);
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
