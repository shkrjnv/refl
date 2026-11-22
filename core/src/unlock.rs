//! The unlock model shared by themes, frames and companions, and the
//! milestones that drive it (`docs/07-themes-achievements-frames.md`,
//! «Модель разблокировок»; `docs/06-companions.md`, «Вехи, главы и эпизоды»).

use serde::{Deserialize, Serialize};

use crate::ids::{AchievementId, KcId, UserId};
use crate::value::Cefr;

/// A learner milestone, recorded once on first achievement and never revoked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "key", rename_all = "snake_case")]
pub enum Milestone {
    /// Overall level: the minimum of domain levels with data (docs/06).
    Level(Cefr),
    /// A topic of the KC graph (`part_of`), ≥ 80% of its importance mastered.
    Topic(KcId),
}

/// What can be unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum UnlockKind {
    Theme,
    Frame,
    Companion,
}

/// Why something was unlocked. Milestone and achievement unlocks are derived
/// from the log and never taken away; admin unlocks can be revoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnlockSource {
    Milestone { milestone: Milestone },
    Achievement { achievement: AchievementId },
    Admin { by: UserId },
}

impl Milestone {
    /// `(kind, key)` as written in events, e.g. `("level", "B1")`.
    pub fn kind_key(&self) -> (&'static str, &str) {
        match self {
            Self::Level(c) => ("level", c.as_str()),
            Self::Topic(kc) => ("topic", kc.as_str()),
        }
    }
}

impl UnlockKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Frame => "frame",
            Self::Companion => "companion",
        }
    }
}
