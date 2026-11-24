//! The model projection of an item version: what the fold reads from the
//! item and nothing else (`docs/02-layer1-content.md`, «Идентификаторы и
//! версии»). The bundle carries the projections of every published version,
//! so an old event's item resolves even after the item's version was bumped;
//! `content/item-versions.lock` keeps them append-only.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::ids::{FamilyId, ItemId, ItemVersion, MisconceptionId, SubjectId};
use crate::item::{AuthorDifficulty, Item, ItemKindTag, LadderLevel, SlotKc};
use crate::kind::ItemKind;

/// Chance of a correct answer by guessing among `choices` answers; 0 when
/// there is no pick (`docs/03-layer2-learner-model.md`, g).
pub fn guessing_rate(choices: u16) -> f64 {
    if choices == 0 {
        0.0
    } else {
        1.0 / f64::from(choices)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotProjection {
    /// The slot's Q-matrix row.
    pub kcs: Vec<SlotKc>,
    /// Answers to pick among, the correct one included; 0 for typed input
    /// and for kinds without a pick.
    pub choices: u16,
    /// Misconceptions the slot's options and error patterns diagnose,
    /// ascending, without repeats.
    pub misconceptions: Vec<MisconceptionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemProjection {
    pub subject: SubjectId,
    pub family: FamilyId,
    pub kind: ItemKindTag,
    pub difficulty: AuthorDifficulty,
    /// Seconds.
    pub expected_time: u32,
    /// In slot order.
    pub slots: Vec<SlotProjection>,
}

impl ItemProjection {
    /// The one function that takes a projection from an item: content-build
    /// and the tests use it, so a stored projection and a fresh one compare.
    pub fn of(item: &Item) -> Self {
        let slots = item.body.slots();
        Self {
            subject: item.subject.clone(),
            family: item.family.clone(),
            kind: item.body.tag(),
            difficulty: item.difficulty,
            expected_time: item.expected_time,
            slots: slots
                .iter()
                .zip(0u16..)
                .map(|(slot, index)| SlotProjection {
                    kcs: slot.kcs.clone(),
                    choices: item.body.choices(index),
                    misconceptions: slot
                        .options
                        .iter()
                        .filter_map(|o| o.misconception.as_ref())
                        .chain(slot.expected_wrong.iter().map(|w| &w.misconception))
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .cloned()
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn ladder(&self) -> LadderLevel {
        self.kind.ladder()
    }

    pub fn is_production(&self) -> bool {
        self.kind.is_production()
    }

    pub fn typed_input(&self) -> bool {
        self.kind.typed_input()
    }

    /// Chance of a correct answer by guessing on `slot`; 0 for a slot the
    /// item does not have.
    pub fn guessing(&self, slot: u16) -> f64 {
        self.slots
            .get(usize::from(slot))
            .map_or(0.0, |s| guessing_rate(s.choices))
    }
}

/// One published (item, version) with its projection: an entry of the bundle
/// and of the lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemVersionEntry {
    pub item: ItemId,
    pub version: ItemVersion,
    pub projection: ItemProjection,
}
