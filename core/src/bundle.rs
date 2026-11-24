//! The compiled content bundle produced by content-build
//! (`docs/02-layer1-content.md`, «Проверки при сборке контента»;
//! `docs/01-architecture.md`: performance is built in at content build time).
//!
//! Everything the runtime would otherwise compute is precomputed here: KC
//! status, the transitive closure of `requires`, item indexes by KC, compiled
//! answer automata. Collections are sorted, so equal content serializes to
//! equal bytes.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::answer::dfa::{Dfa, TokenTable};
use crate::content::{Edge, Explanation, Instruction, Kc, Misconception};
use crate::ids::{ContentVersion, ItemRef, KcId, MisconceptionId};
use crate::item::Item;
use crate::lesson::Lesson;
use crate::lexicon::LexiconRef;
use crate::subject::Subject;

/// Layout version of [`Bundle`].
pub const BUNDLE_FORMAT_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub format_version: u32,
    /// Hex SHA-256 of the serialized `content`.
    pub content_version: ContentVersion,
    pub content: BundleContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BundleContent {
    /// Normalization the automata were compiled with; answers must be
    /// normalized with the same version.
    pub normalization_version: u32,
    /// The checker's word list, a separate file; with `normalization_version`
    /// it makes the checker version.
    pub lexicon: LexiconRef,
    /// Sorted by id.
    pub subjects: Vec<Subject>,
    /// Sorted by id.
    pub kcs: Vec<KcEntry>,
    /// Sorted.
    pub edges: Vec<Edge>,
    /// Every KC mapped to all KCs it requires directly or transitively, sorted.
    pub requires_closure: BTreeMap<KcId, Vec<KcId>>,
    /// Sorted by id.
    pub misconceptions: Vec<Misconception>,
    /// Sorted by id.
    pub explanations: Vec<Explanation>,
    /// Sorted by id.
    pub instructions: Vec<Instruction>,
    /// Sorted by id.
    pub lessons: Vec<Lesson>,
    /// Sorted by id.
    pub items: Vec<ItemEntry>,
    /// Session-eligible items per KC, by the KC's role in their slots.
    pub items_by_kc: BTreeMap<KcId, KcItems>,
    /// Token alphabet of all automata.
    pub tokens: TokenTable,
}

/// Computed by content-build from the live conditions; never authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KcStatus {
    Draft,
    Live,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcEntry {
    pub kc: Kc,
    pub status: KcStatus,
    /// Rank of `kc.level` in its subject's scale, lowest = 0.
    pub level_rank: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEntry {
    pub item: Item,
    /// Live, and every primary KC of the item is live.
    pub session_eligible: bool,
    /// One per slot, in slot order.
    pub slots: Vec<SlotAutomata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotAutomata {
    /// Union of the slot's accepted patterns.
    pub accepted: Dfa,
    /// In pattern order: the first match wins.
    pub expected_wrong: Vec<ErrorAutomaton>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorAutomaton {
    pub misconception: MisconceptionId,
    pub dfa: Dfa,
}

/// Item references, sorted.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KcItems {
    pub primary: Vec<ItemRef>,
    pub secondary: Vec<ItemRef>,
}
