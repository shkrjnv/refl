//! Lessons: an ordered set of steps over a set of KCs (`docs/02-layer1-content.md`,
//! «Урок и шаги»). Practice steps reference bank queries, not concrete items.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::content::{Example, ExplanationDepth};
use crate::ids::{KcId, LessonId};
use crate::item::{AuthorDifficulty, ItemType};

/// Query to the item bank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BankQuery {
    pub kcs: Vec<KcId>,
    pub types: Vec<ItemType>,
    pub min_difficulty: AuthorDifficulty,
    pub max_difficulty: AuthorDifficulty,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum LessonStep {
    /// Attempt before the explanation (productive failure).
    Pretest {
        query: BankQuery,
    },
    Explanation {
        kc: KcId,
        depth: ExplanationDepth,
    },
    WorkedExamples {
        examples: Vec<Example>,
    },
    /// Ordered from recognition to production.
    Practice {
        query: BankQuery,
    },
    Production {
        query: BankQuery,
    },
    /// Repeat of the pretest or members of the same item families.
    Posttest {
        query: BankQuery,
    },
    /// Mastery, misconceptions and next step, produced by the learner model.
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Lesson {
    pub id: LessonId,
    pub kcs: Vec<KcId>,
    pub steps: Vec<LessonStep>,
}
