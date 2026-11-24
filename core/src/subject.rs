//! Subjects: what differs between English, Japanese and mathematics is data
//! here, not code elsewhere (`docs/10-subjects.md`, «Предмет», «Шкала
//! уровней»). Every KC names its subject, a level key from the subject's
//! scale and a domain from the subject's list; content-build checks all three.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

use crate::ids::{DomainId, KcId, SubjectId};
use crate::value::{Finite, Localized, ValueError};

/// Key of a level in a subject's scale as content and the interface name it,
/// e.g. `pre-A1`, `B2`, `N4`, `g7`. The core works with its rank.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct LevelKey(String);

impl LevelKey {
    /// Returns `None` unless `s` is non-empty ASCII letters, digits and `-`.
    pub fn new(s: impl Into<String>) -> Option<Self> {
        let s = s.into();
        let valid = !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        valid.then_some(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LevelKey {
    type Error = ValueError;
    fn try_from(s: String) -> Result<Self, ValueError> {
        Self::new(s).ok_or(ValueError::InvalidLevelKey)
    }
}

impl From<LevelKey> for String {
    fn from(k: LevelKey) -> String {
        k.0
    }
}

impl fmt::Display for LevelKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScaleLevel {
    pub key: LevelKey,
    pub title: Localized,
}

/// The subject's one main scale: levels from lowest to highest, and the step
/// between neighbours in logits for the starting μ (`docs/03-layer2-learner-model.md`,
/// «Сложность заданий и стартовые значения», h).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScaleDef {
    pub levels: Vec<ScaleLevel>,
    pub step_logit: Finite,
}

impl ScaleDef {
    /// Position of `key` in the scale, lowest = 0.
    pub fn rank(&self, key: &LevelKey) -> Option<u16> {
        let i = self.levels.iter().position(|l| &l.key == key)?;
        u16::try_from(i).ok()
    }
}

/// How typed answers of a subject are normalized and tokenized
/// (`docs/10-subjects.md`, «Проверщик ввода»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "kebab-case"))]
#[serde(rename_all = "kebab-case")]
pub enum Checker {
    /// English: the normalization of docs/02, word tokens.
    EnWords,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Subject {
    pub id: SubjectId,
    pub title: Localized,
    pub scale: ScaleDef,
    pub domains: Vec<DomainId>,
    /// Default checker of the subject's typed input.
    pub checker: Checker,
    /// The orthography KC a typo updates as wrong, while the item's target
    /// KC counts as passed (`docs/02-layer1-content.md`, «Проверка ответов на ввод»).
    pub spelling_kc: KcId,
}
