//! Identifiers. Content ids are readable slugs that never change
//! (`docs/02-layer1-content.md`, «Идентификаторы и версии»); renaming means a
//! new id plus an alias. Runtime ids are UUIDs created on the client.

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::value::ValueError;

fn is_valid_slug(s: &str) -> bool {
    let is_sep = |b: u8| b == b'.' || b == b'-';
    let bytes = s.as_bytes();
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || is_sep(b))
        && !is_sep(bytes[0])
        && !is_sep(bytes[bytes.len() - 1])
        && !bytes.windows(2).any(|w| is_sep(w[0]) && is_sep(w[1]))
}

macro_rules! slug_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Returns `None` unless `s` is a valid slug (`[a-z0-9]` words joined by `.` or `-`).
            pub fn new(s: impl Into<String>) -> Option<Self> {
                let s = s.into();
                is_valid_slug(&s).then_some(Self(s))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ValueError;
            fn try_from(s: String) -> Result<Self, ValueError> {
                Self::new(s).ok_or(ValueError::InvalidSlug)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

slug_id!(
    /// Knowledge component, e.g. `gram.article.first-mention`.
    KcId
);
slug_id!(
    /// Misconception, e.g. `mis.article.omission-l1`.
    MisconceptionId
);
slug_id!(
    /// Item (task).
    ItemId
);
slug_id!(
    /// Family of item variants sharing one difficulty estimate.
    FamilyId
);
slug_id!(
    /// Explanation record (one target, depth and language).
    ExplanationId
);
slug_id!(
    /// Instruction template, translated once.
    InstructionId
);
slug_id!(
    /// Lesson.
    LessonId
);
slug_id!(
    /// Content topic: travel, tech, family…
    TopicId
);
slug_id!(
    /// Free-form tag, e.g. `ielts-relevant`, `l1-hard-uz`.
    Tag
);

/// Item version; an event always references `(item_id, version)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ItemVersion(pub u32);

/// Exactly what was shown: an item at a specific version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ItemRef {
    pub id: ItemId,
    pub version: ItemVersion,
}

/// Version of a content build (see `reclassified` in `docs/02-layer1-content.md`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ContentVersion(pub String);

/// Event id: `UUIDv7`, time-ordered, created on the client, so offline sync is idempotent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "Uuid", into = "Uuid")]
pub struct EventId(Uuid);

impl EventId {
    /// Returns `None` unless `u` is a version-7 UUID.
    pub fn new(u: Uuid) -> Option<Self> {
        (u.get_version_num() == 7).then_some(Self(u))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl TryFrom<Uuid> for EventId {
    type Error = ValueError;
    fn try_from(u: Uuid) -> Result<Self, ValueError> {
        Self::new(u).ok_or(ValueError::NotUuidV7)
    }
}

impl From<EventId> for Uuid {
    fn from(id: EventId) -> Uuid {
        id.0
    }
}

macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
    };
}

uuid_id!(
    /// Learner account.
    LearnerId
);
uuid_id!(
    /// Learning session.
    SessionId
);

#[cfg(test)]
mod tests {
    use super::is_valid_slug;

    #[test]
    fn slug_rules() {
        for ok in [
            "gram.article.first-mention",
            "mis.article.omission-l1",
            "a",
            "b1",
        ] {
            assert!(is_valid_slug(ok), "{ok}");
        }
        for bad in ["", ".a", "a.", "a..b", "a.-b", "A", "a b", "a_b", "é"] {
            assert!(!is_valid_slug(bad), "{bad}");
        }
    }
}
