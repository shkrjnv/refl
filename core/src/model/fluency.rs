//! Fluency by retrieval time (`docs/03-layer2-learner-model.md`, «Беглость:
//! время извлечения»): what the fold reads about item families, and the
//! robust statistics of the fluency condition.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::bundle::Bundle;
use crate::ids::{FamilyId, InstructionId};
use crate::kind::ItemKind;

/// Motor speed of the βᶠ prior, characters per second: the reference is a
/// phone (proposal).
pub const PRIOR_CHARS_PER_S: f64 = 1.5;

/// The βᶠ prior is at least the log of this many seconds.
pub const PRIOR_FLOOR_S: f64 = 1.0;

/// MAD × this estimates σ of a normal distribution.
const MAD_TO_SIGMA: f64 = 1.4826;

/// One row of the family table (`docs/02-layer1-content.md`, «Таблица семей»).
#[derive(Debug, Clone, PartialEq)]
pub struct Family {
    /// The instruction all items of the family share (build check
    /// `family-instruction`).
    pub instruction: InstructionId,
    /// βᶠ, the log of seconds of retrieval; in the prototype the prior.
    pub beta: f64,
}

/// The family table of a bundle. Built from the current item bodies, so it
/// is not stored in the bundle and frozen bundles need no rebuild; its rows
/// are part of `fold_version`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Families(BTreeMap<FamilyId, Family>);

impl Families {
    /// Every family with at least one item in `bundle`, of any status:
    /// `βᶠ = ln(max(1 s, median over its items' slots of (expected_time −
    /// motor time of the slot's reference answer)))`, the motor time 0 for a
    /// kind without typing. The instruction is that of the family's first
    /// item by id.
    #[allow(clippy::cast_precision_loss)] // answers are short
    pub fn of(bundle: &Bundle) -> Self {
        let mut acc: BTreeMap<&FamilyId, (&InstructionId, Vec<f64>)> = BTreeMap::new();
        for e in &bundle.content.items {
            let item = &e.item;
            let typed = item.body.typed_input();
            let expected = f64::from(item.expected_time);
            let (_, values) = acc
                .entry(&item.family)
                .or_insert_with(|| (&item.instruction, Vec::new()));
            let slots = item.body.slots();
            if slots.is_empty() {
                values.push(expected);
            }
            for slot in slots {
                let motor = if typed {
                    slot.answer.chars().count() as f64 / PRIOR_CHARS_PER_S
                } else {
                    0.0
                };
                values.push(expected - motor);
            }
        }
        Self(
            acc.into_iter()
                .map(|(id, (instruction, mut values))| {
                    let seconds = median(&mut values).unwrap_or(0.0);
                    let family = Family {
                        instruction: instruction.clone(),
                        beta: libm::log(seconds.max(PRIOR_FLOOR_S)),
                    };
                    (id.clone(), family)
                })
                .collect(),
        )
    }

    pub fn get(&self, id: &FamilyId) -> Option<&Family> {
        self.0.get(id)
    }

    /// Rows in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&FamilyId, &Family)> {
        self.0.iter()
    }
}

/// Median of `values`, sorted in place; the mean of the two middle ones for
/// an even count; `None` for none.
pub fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        f64::midpoint(values[n / 2 - 1], values[n / 2])
    })
}

/// `1.4826 × MAD`: a spread robust to a stray time; `None` for no values.
pub fn spread(values: &[f64]) -> Option<f64> {
    let mut v = values.to_vec();
    let m = median(&mut v)?;
    let mut deviations: Vec<f64> = values.iter().map(|x| libm::fabs(x - m)).collect();
    median(&mut deviations).map(|mad| MAD_TO_SIGMA * mad)
}

/// Typing speed shrunk to the prior: `(n₀ v₀ + n m) / (n₀ + n)`, m the
/// median of `samples`.
#[allow(clippy::cast_precision_loss)] // at most a window of samples
pub fn shrunk_speed(samples: &[f64], v0: f64, n0: f64) -> f64 {
    let mut v = samples.to_vec();
    match median(&mut v) {
        Some(m) => {
            let n = samples.len() as f64;
            (n0 * v0 + n * m) / (n0 + n)
        }
        None => v0,
    }
}
