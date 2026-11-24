//! Items (tasks), slots and item families (`docs/02-layer1-content.md`,
//! «Задание: общая часть», «Типы заданий»). The unit of assessment is the slot.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::content::AudioRef;
use crate::ids::{
    FamilyId, InstructionId, ItemId, ItemVersion, KcId, MisconceptionId, SubjectId, TopicId,
};
use crate::value::{Localized, ValueError};

/// Item types along the ladder recognition → guided production → free production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Choice,
    FindError,
    Matching,
    TrueFalseNotGiven,
    GapFill,
    Transformation,
    SentenceAssembly,
    Translation,
    Dictation,
    OwnSentence,
    Writing,
    Shadowing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum LadderLevel {
    Recognition,
    Guided,
    Free,
    Pronunciation,
}

impl ItemType {
    pub fn ladder_level(self) -> LadderLevel {
        match self {
            Self::Choice | Self::FindError | Self::Matching | Self::TrueFalseNotGiven => {
                LadderLevel::Recognition
            }
            Self::GapFill
            | Self::Transformation
            | Self::SentenceAssembly
            | Self::Translation
            | Self::Dictation => LadderLevel::Guided,
            Self::OwnSentence | Self::Writing => LadderLevel::Free,
            Self::Shadowing => LadderLevel::Pronunciation,
        }
    }

    /// Counts towards the "≥ 3 production items" live condition of a KC.
    /// Dictation and sentence assembly do not: the learner transcribes or
    /// orders given words rather than producing the form.
    pub fn is_production(self) -> bool {
        matches!(
            self,
            Self::GapFill
                | Self::Transformation
                | Self::Translation
                | Self::OwnSentence
                | Self::Writing
        )
    }

    /// Quality rule a live item of this type must satisfy, per slot.
    /// `None`: no rule is defined yet, so the type cannot be live.
    pub fn quality_rule(self) -> Option<QualityRule> {
        match self {
            Self::Choice | Self::FindError => Some(QualityRule::Choice),
            Self::GapFill
            | Self::Transformation
            | Self::Translation
            | Self::Dictation
            | Self::SentenceAssembly => Some(QualityRule::TypedInput),
            Self::Matching
            | Self::TrueFalseNotGiven
            | Self::OwnSentence
            | Self::Writing
            | Self::Shadowing => None,
        }
    }
}

/// Per-slot quality rule for live items (`docs/02-layer1-content.md`,
/// «Задание: общая часть»).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityRule {
    /// ≥ 2 diagnosed distractors; no distractor is accepted; no duplicates.
    Choice,
    /// ≥ 2 expected-error patterns.
    TypedInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Draft,
    Review,
    Live,
    Retired,
}

/// Author's difficulty 1–5 within the KC; mapped to IRT logits by the learner model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u8", into = "u8")]
pub struct AuthorDifficulty(u8);

impl AuthorDifficulty {
    pub fn new(v: u8) -> Option<Self> {
        (1..=5).contains(&v).then_some(Self(v))
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for AuthorDifficulty {
    type Error = ValueError;
    fn try_from(v: u8) -> Result<Self, ValueError> {
        Self::new(v).ok_or(ValueError::DifficultyOutOfRange)
    }
}

impl From<AuthorDifficulty> for u8 {
    fn from(v: AuthorDifficulty) -> u8 {
        v.0
    }
}

/// Path of an image, relative to the media root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ImageRef(pub String);

/// The task itself: English text with gap markers, optional audio and image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Stimulus {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub audio: Option<AudioRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub image: Option<ImageRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum KcRole {
    /// Checked directly.
    Primary,
    /// Needed, but not the target.
    Secondary,
}

/// One row of the Q-matrix: a KC this slot depends on, with its role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SlotKc {
    pub kc: KcId,
    pub role: KcRole,
}

/// How contractions compare (`docs/02-layer1-content.md`, «Язык шаблонов ответов»).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum Contractions {
    /// don't and do not are different answers.
    #[default]
    Strict,
    /// Both sides are expanded by the fixed contraction table.
    Equivalent,
}

/// Answer pattern in the template language, e.g. `(I have|I've) been [already]`.
/// Compiled to an automaton by content-build; stored here as source text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct AnswerPattern(pub String);

/// A choice option: diagnosed misconception, or `None` for noise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChoiceOption {
    pub text: String,
    pub misconception: Option<MisconceptionId>,
    /// The option is the correct answer for this neighbouring KC: a confusion
    /// between contrasting topics (`docs/03-layer2-learner-model.md`,
    /// «Расширенная диагностика»).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub confuses_with: Option<KcId>,
}

/// Expected wrong answer for typed input, tied to a misconception.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExpectedWrong {
    pub pattern: AnswerPattern,
    pub misconception: MisconceptionId,
    /// As in [`ChoiceOption::confuses_with`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub confuses_with: Option<KcId>,
}

/// A gap: the unit of assessment, with its own KCs, distractors and log event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Slot {
    pub kcs: Vec<SlotKc>,
    /// Reference answer shown to the learner as correct; must be accepted by `accepted`.
    pub answer: String,
    pub accepted: Vec<AnswerPattern>,
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
    #[serde(default)]
    pub expected_wrong: Vec<ExpectedWrong>,
    /// Optional hint; its use is logged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hint: Option<Localized>,
    /// Why the correct answer is correct in this particular sentence.
    pub why: Localized,
}

/// An item at one version. Version bumps when stimulus, options, the correct
/// answer or KC links change; not for new accepted answers or error patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Item {
    pub id: ItemId,
    pub version: ItemVersion,
    /// Matches the prefix of `family` (checked by content-build).
    pub subject: SubjectId,
    pub family: FamilyId,
    #[serde(rename = "type")]
    #[cfg_attr(feature = "ts", ts(rename = "type"))]
    pub item_type: ItemType,
    pub instruction: InstructionId,
    pub stimulus: Stimulus,
    pub slots: Vec<Slot>,
    pub difficulty: AuthorDifficulty,
    /// Seconds; for automaticity and session planning.
    pub expected_time: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topic: Option<TopicId>,
    pub status: ItemStatus,
    /// Letter case takes part in comparison (tasks on capital letters).
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub contractions: Contractions,
    /// Punctuation tokens take part in comparison.
    #[serde(default)]
    pub punctuation_sensitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license: Option<String>,
    /// Set when this version changes difficulty: stats of older versions are not carried over.
    #[serde(default)]
    pub breaks_calibration: bool,
}
