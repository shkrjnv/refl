//! The unlock model shared by themes, frames and companions, and the
//! milestones that drive it (`docs/07-themes-achievements-frames.md`,
//! «Модель разблокировок»; `docs/06-companions.md`, «Вехи, главы и эпизоды»).

use serde::{Deserialize, Serialize};

use crate::ids::{AchievementId, KcId, UserId};
use crate::value::{Cefr, ValueError};

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
    Companion,
    Avatar,
    Frame,
    Background,
    Title,
    Effect,
}

/// Why something was unlocked. Milestone and achievement unlocks are derived
/// from the log and never taken away; admin unlocks can be revoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnlockSource {
    Milestone {
        milestone: Milestone,
    },
    Achievement {
        achievement: AchievementId,
        #[serde(default)]
        tier: AchievementTier,
    },
    Admin {
        by: UserId,
    },
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
            Self::Companion => "companion",
            Self::Avatar => "avatar",
            Self::Frame => "frame",
            Self::Background => "background",
            Self::Title => "title",
            Self::Effect => "effect",
        }
    }
}

/// Tier of an achievement, I–VI as `1..=6`; an achievement without tiers has
/// only tier I (`docs/07-themes-achievements-frames.md`, «Ачивки»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u8", into = "u8")]
pub struct AchievementTier(u8);

impl AchievementTier {
    pub const I: Self = Self(1);

    /// Returns `None` unless `1 <= v <= 6`.
    pub fn new(v: u8) -> Option<Self> {
        (1..=6).contains(&v).then_some(Self(v))
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl Default for AchievementTier {
    fn default() -> Self {
        Self::I
    }
}

impl TryFrom<u8> for AchievementTier {
    type Error = ValueError;
    fn try_from(v: u8) -> Result<Self, ValueError> {
        Self::new(v).ok_or(ValueError::TierOutOfRange)
    }
}

impl From<AchievementTier> for u8 {
    fn from(v: AchievementTier) -> u8 {
        v.0
    }
}

/// Profile slot a cosmetic is worn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum CosmeticSlot {
    Avatar,
    Frame,
    Background,
    Title,
    Effect,
}

/// What a profile showcase shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum ShowcaseKind {
    /// Topic cards; keys are topic KC ids (`docs/07-themes-achievements-frames.md`, «Карточки»).
    Cards,
}
