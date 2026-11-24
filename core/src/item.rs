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

/// The `type` tag of an [`ItemBody`] variant, for filtering (lesson bank
/// queries). It carries no behaviour: that lives in each kind's
/// [`ItemKind`](crate::kind::ItemKind) implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum ItemKindTag {
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

/// Step of the ladder recognition → guided production → free production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum LadderLevel {
    Recognition,
    Guided,
    Free,
    Pronunciation,
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

/// Settings of the English input checker (`docs/02-layer1-content.md`,
/// «Язык шаблонов ответов»). Typed-input kinds carry them; choice kinds
/// compare with the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnglishCheck {
    /// Letter case takes part in comparison (tasks on capital letters).
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub contractions: Contractions,
    /// Punctuation tokens take part in comparison.
    #[serde(default)]
    pub punctuation_sensitive: bool,
}

/// One body type per kind, so each kind implements
/// [`ItemKind`](crate::kind::ItemKind) on its own and the client renders it
/// from its own generated type.
macro_rules! item_body {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub struct $name {
            pub stimulus: Stimulus,
            pub slots: Vec<Slot>,
        }
    };
    ($(#[$doc:meta])* $name:ident, english_input) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub struct $name {
            pub stimulus: Stimulus,
            pub slots: Vec<Slot>,
            #[serde(flatten)]
            #[cfg_attr(feature = "ts", ts(flatten))]
            pub check: EnglishCheck,
        }
    };
}

item_body!(
    /// Pick the option; options are diagnosed distractors.
    ChoiceBody
);
item_body!(
    /// Find the error in the sentence among options.
    FindErrorBody
);
item_body!(
    /// Matching, including IELTS matching headings.
    MatchingBody
);
item_body!(TrueFalseNotGivenBody);
item_body!(
    /// Gap fill, with or without the base form.
    GapFillBody,
    english_input
);
item_body!(
    /// "Rewrite using since".
    TransformationBody,
    english_input
);
item_body!(SentenceAssemblyBody, english_input);
item_body!(
    /// Short sentences uz/ru → en.
    TranslationBody,
    english_input
);
item_body!(
    /// Audio → text.
    DictationBody,
    english_input
);
item_body!(OwnSentenceBody);
item_body!(WritingBody);
item_body!(ShadowingBody);

/// The kind-specific part of an item: stimulus, slots and checker settings.
/// Serialized flat into [`Item`] under the `type` tag, so content files keep
/// their shape (`docs/10-subjects.md`, «Вид задания: трейт и данные»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemBody {
    Choice(ChoiceBody),
    FindError(FindErrorBody),
    Matching(MatchingBody),
    TrueFalseNotGiven(TrueFalseNotGivenBody),
    GapFill(GapFillBody),
    Transformation(TransformationBody),
    SentenceAssembly(SentenceAssemblyBody),
    Translation(TranslationBody),
    Dictation(DictationBody),
    OwnSentence(OwnSentenceBody),
    Writing(WritingBody),
    Shadowing(ShadowingBody),
}

/// Applies `$e` to the body of whichever variant `$self` is.
macro_rules! each_body {
    ($self:expr, $b:ident => $e:expr) => {
        match $self {
            ItemBody::Choice($b) => $e,
            ItemBody::FindError($b) => $e,
            ItemBody::Matching($b) => $e,
            ItemBody::TrueFalseNotGiven($b) => $e,
            ItemBody::GapFill($b) => $e,
            ItemBody::Transformation($b) => $e,
            ItemBody::SentenceAssembly($b) => $e,
            ItemBody::Translation($b) => $e,
            ItemBody::Dictation($b) => $e,
            ItemBody::OwnSentence($b) => $e,
            ItemBody::Writing($b) => $e,
            ItemBody::Shadowing($b) => $e,
        }
    };
}
pub(crate) use each_body;

impl ItemBody {
    pub fn tag(&self) -> ItemKindTag {
        match self {
            Self::Choice(_) => ItemKindTag::Choice,
            Self::FindError(_) => ItemKindTag::FindError,
            Self::Matching(_) => ItemKindTag::Matching,
            Self::TrueFalseNotGiven(_) => ItemKindTag::TrueFalseNotGiven,
            Self::GapFill(_) => ItemKindTag::GapFill,
            Self::Transformation(_) => ItemKindTag::Transformation,
            Self::SentenceAssembly(_) => ItemKindTag::SentenceAssembly,
            Self::Translation(_) => ItemKindTag::Translation,
            Self::Dictation(_) => ItemKindTag::Dictation,
            Self::OwnSentence(_) => ItemKindTag::OwnSentence,
            Self::Writing(_) => ItemKindTag::Writing,
            Self::Shadowing(_) => ItemKindTag::Shadowing,
        }
    }

    pub fn stimulus(&self) -> &Stimulus {
        each_body!(self, b => &b.stimulus)
    }

    pub fn slots(&self) -> &[Slot] {
        each_body!(self, b => &b.slots)
    }

    /// The English checker settings of a typed-input kind; `None` for the
    /// others, which compare with the defaults.
    pub fn english_check(&self) -> Option<&EnglishCheck> {
        match self {
            Self::GapFill(b) => Some(&b.check),
            Self::Transformation(b) => Some(&b.check),
            Self::SentenceAssembly(b) => Some(&b.check),
            Self::Translation(b) => Some(&b.check),
            Self::Dictation(b) => Some(&b.check),
            Self::Choice(_)
            | Self::FindError(_)
            | Self::Matching(_)
            | Self::TrueFalseNotGiven(_)
            | Self::OwnSentence(_)
            | Self::Writing(_)
            | Self::Shadowing(_) => None,
        }
    }
}

/// An item at one version: the header every kind shares, and the
/// kind-specific body. Version bumps when stimulus, options, the correct
/// answer or KC links change; not for new accepted answers or error patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Item {
    pub id: ItemId,
    pub version: ItemVersion,
    /// Matches the prefix of `family` (checked by content-build).
    pub subject: SubjectId,
    pub family: FamilyId,
    pub instruction: InstructionId,
    pub difficulty: AuthorDifficulty,
    /// Seconds; for automaticity and session planning.
    pub expected_time: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topic: Option<TopicId>,
    pub status: ItemStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license: Option<String>,
    /// Set when this version changes difficulty: stats of older versions are not carried over.
    #[serde(default)]
    pub breaks_calibration: bool,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub body: ItemBody,
}
