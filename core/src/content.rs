//! Knowledge components, the KC graph, misconceptions and explanations
//! (`docs/02-layer1-content.md`). The learner model, placement and planner only
//! read these; nothing in the core mutates content.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::ids::{DomainId, ExplanationId, InstructionId, KcId, MisconceptionId, SubjectId, Tag};
use crate::subject::LevelKey;
use crate::value::{Lang, Localized, UnitInterval};

/// Knowledge component: the smallest rule one can know or not know.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Kc {
    pub id: KcId,
    pub subject: SubjectId,
    /// One of the subject's domains.
    pub domain: DomainId,
    pub title: Localized,
    /// Key in the subject's scale: the level where the rule is usually learned.
    pub level: LevelKey,
    /// Construction frequency × goal weight; planner input.
    pub importance: UnitInterval,
    /// Short statement for content authors, not shown to learners.
    pub rule: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Hard prerequisite: `from` is not taught without `to`. Acyclic (checked at build).
    Requires,
    /// Soft link: `to` helps with `from`.
    Supports,
    /// Confusable topics, interleaved in practice. Symmetric in meaning.
    Contrasts,
    /// Hierarchy KC → topic → section.
    PartOf,
}

/// Directed graph edge `from --kind--> to` (e.g. `from` requires `to`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: KcId,
    pub to: KcId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "kebab-case"))]
#[serde(rename_all = "kebab-case")]
pub enum MisconceptionOrigin {
    L1Transfer,
    Overgeneralization,
    FormConfusion,
    SpellingBased,
}

/// Which native languages transfer this misconception: uz, ru, both or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct L1Set {
    pub uz: bool,
    pub ru: bool,
}

/// Frequency among uz and ru native speakers; placement probes frequent ones first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct L1Prior {
    pub uz: UnitInterval,
    pub ru: UnitInterval,
}

/// A stable wrong idea that produces predictable errors. Its micro-explanation
/// is found by key: target = this misconception, depth = short, language by
/// the chain interface -> ru -> en.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Misconception {
    pub id: MisconceptionId,
    pub title: Localized,
    pub affects: Vec<KcId>,
    pub origin: MisconceptionOrigin,
    pub l1: L1Set,
    pub prior: L1Prior,
    /// Topic that treats the cause; usually a prerequisite.
    pub remedy: KcId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ExplanationTarget {
    Kc(KcId),
    Misconception(MisconceptionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum ExplanationDepth {
    Short,
    Why,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Draft,
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Review {
    pub status: ReviewStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
}

/// Path of a generated audio file, relative to the audio root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct AudioRef(pub String);

/// English example sentence, shared by all languages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Example {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub audio: Option<AudioRef>,
}

/// One explanation record: one target, one depth, one language.
/// Full explanations target a KC; micro-explanations (≤ 2 sentences) target a misconception.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Explanation {
    pub id: ExplanationId,
    pub target: ExplanationTarget,
    pub depth: ExplanationDepth,
    pub lang: Lang,
    /// Markdown with explanation components (audio example, table, decision tree, timeline).
    pub body: String,
    /// Native-language contrast block; authored separately, not translated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub l1_contrast: Option<String>,
    #[serde(default)]
    pub examples: Vec<Example>,
    pub review: Review,
}

/// Item instruction, translated once instead of in every item
/// (`docs/02-layer1-content.md`, «Задание: общая часть»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Instruction {
    pub id: InstructionId,
    pub text: Localized,
}
