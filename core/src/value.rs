//! Small validated value types shared by the domain model.

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};

/// A number in `[0, 1]` (importance, prior frequency). NaN is rejected.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "f64", into = "f64")]
pub struct UnitInterval(f64);

impl UnitInterval {
    /// Returns `None` unless `0 <= v <= 1`.
    pub fn new(v: f64) -> Option<Self> {
        (0.0..=1.0).contains(&v).then_some(Self(v))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for UnitInterval {
    type Error = ValueError;
    fn try_from(v: f64) -> Result<Self, ValueError> {
        Self::new(v).ok_or(ValueError::OutOfUnitInterval)
    }
}

impl From<UnitInterval> for f64 {
    fn from(v: UnitInterval) -> f64 {
        v.0
    }
}

/// Milliseconds since the Unix epoch, UTC. Always passed in, never read from a clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct Timestamp(pub i64);

/// Interface and explanation languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Uz,
    Ru,
    En,
}

/// Text in the interface languages. Completeness requirements (e.g. "ru and en
/// for every live KC explanation") are content-build checks, not type rules.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Localized {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uz: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub en: Option<String>,
}

/// CEFR level; `level()` gives the numeric scale pre-A1 = 0 … C2 = 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Cefr {
    #[serde(rename = "pre-A1")]
    #[cfg_attr(feature = "ts", ts(rename = "pre-A1"))]
    PreA1,
    A1,
    A2,
    B1,
    B2,
    C1,
    C2,
}

impl Cefr {
    pub fn level(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    OutOfUnitInterval,
    InvalidSlug,
    NotUuidV7,
    DifficultyOutOfRange,
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfUnitInterval => "value must be within [0, 1]",
            Self::InvalidSlug => {
                "slug must be non-empty [a-z0-9.-] without leading, trailing or doubled separators"
            }
            Self::NotUuidV7 => "event id must be a UUIDv7",
            Self::DifficultyOutOfRange => "author difficulty must be 1..=5",
        })
    }
}

impl core::error::Error for ValueError {}
