//! The short adaptive placement check (`docs/04-placement-planner.md`,
//! «Placement: требования и этапы», stage 3 without the propagation term).
//!
//! The check is not a second learner model: it picks live items from the
//! bundle by expected information gain per second from the same
//! [`KnowledgeModel`], and the caller folds every answer as an ordinary
//! `answer` event with `context: placement`. The result is the model's
//! continuous level and 80% interval, recorded by a `placement_completed`
//! event so every device folds the same profile.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::bundle::{Bundle, KcEntry, KcStatus};
use crate::event::PlacementSource;
use crate::ids::{DomainId, FamilyId, ItemRef, SubjectId};
use crate::item::{ItemKindTag, KcRole};
use crate::model::derived::Level;
use crate::model::{KnowledgeModel, State};
use crate::value::Timestamp;

/// Hard cap on checked items; the check usually stops earlier.
pub const MAX_ITEMS: usize = 10;
/// Do not apply the variance stop before this many answers.
pub const MIN_ITEMS: usize = 4;
/// Stop once every live, teachable KC has σ below this value.
pub const STOP_SIGMA: f64 = 0.8;
/// A learner with this many answers and few correct ones exits early.
pub const BEGINNER_ITEMS: usize = 3;
pub const BEGINNER_MAX_CORRECT: u16 = 1;
pub const BEGINNER_LEVEL: f64 = 0.6;

/// One item and the slot that was shown. One item can have several slots.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlacementItem {
    pub item: ItemRef,
    pub slot: u16,
}

/// Device-local progress of one check; it is not a log event. The confirmed
/// result is `placement_completed`, and the answers are ordinary `answer`
/// events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub subject: SubjectId,
    #[serde(default)]
    pub asked: Vec<PlacementItem>,
    #[serde(default)]
    pub correct: u16,
}

/// The end of a check: the model's level at that moment, or 0 for a beginner.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementResult {
    pub level: Option<Level>,
    pub start_level: f64,
    pub answered: u16,
    pub source: PlacementSource,
}

/// What the caller should show next.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementNext {
    Item {
        item: ItemRef,
        slot: u16,
    },
    Done(PlacementResult),
    /// The live bank has no automatically checked item for this subject.
    Unavailable,
}

#[derive(Debug)]
struct Candidate {
    item: ItemRef,
    slot: u16,
    score: f64,
    typed: bool,
}

/// Picks the next item, or finishes. Deterministic for the same bundle,
/// state and placement progress: ties break by item id and slot.
pub fn next(
    model: &KnowledgeModel,
    bundle: &Bundle,
    state: &State,
    placement: &Placement,
    now: Timestamp,
) -> PlacementNext {
    let live = live_kcs(bundle, &placement.subject);
    if live.is_empty() {
        return PlacementNext::Unavailable;
    }
    // The level estimate walks every live KC; only the beginner exit and a
    // finished check need it. A selection call on a short progress stays
    // cheap.
    let level = level_for_next(model, bundle, state, placement, now);

    if beginner_exit(placement, level.as_ref()) {
        return PlacementNext::Done(PlacementResult {
            level,
            start_level: start_level(level.as_ref(), bundle, &placement.subject),
            answered: answered(placement),
            source: PlacementSource::StartFromZero,
        });
    }
    if placement.asked.len() >= MAX_ITEMS {
        return done(model, bundle, state, placement, now);
    }
    if placement.asked.len() >= MIN_ITEMS && measured(&live, state) {
        return done(model, bundle, state, placement, now);
    }

    let asked_items: BTreeSet<&ItemRef> = placement.asked.iter().map(|a| &a.item).collect();
    let mut used_families: BTreeSet<&FamilyId> = BTreeSet::new();
    let mut asked_pairs: BTreeSet<(&ItemRef, u16)> = BTreeSet::new();
    for a in &placement.asked {
        asked_pairs.insert((&a.item, a.slot));
        if let Some(p) = bundle.projection(&a.item) {
            used_families.insert(&p.family);
        }
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen_pairs: BTreeSet<(ItemRef, u16)> = BTreeSet::new();
    for entry in &live {
        let Some(items) = bundle.content.items_by_kc.get(&entry.kc.id) else {
            continue;
        };
        for item in &items.primary {
            if asked_items.contains(item) {
                continue;
            }
            let Some(projection) = bundle.projection(item) else {
                continue;
            };
            if used_families.contains(&projection.family)
                || recently_seen(state, item, model.params.unseen_days, now)
                || !auto_checked(projection.kind)
            {
                continue;
            }
            for (index, slot) in projection.slots.iter().enumerate() {
                let Ok(slot_index) = u16::try_from(index) else {
                    continue;
                };
                if !slot
                    .kcs
                    .iter()
                    .any(|k| k.kc == entry.kc.id && k.role == KcRole::Primary)
                    || asked_pairs.contains(&(item, slot_index))
                {
                    continue;
                }
                let pair = (item.clone(), slot_index);
                if !seen_pairs.insert(pair) {
                    continue;
                }
                let score = score_slot(
                    model,
                    bundle,
                    state,
                    item,
                    slot_index,
                    projection.expected_time,
                    now,
                );
                if score <= 0.0 {
                    continue;
                }
                candidates.push(Candidate {
                    item: item.clone(),
                    slot: slot_index,
                    score,
                    typed: projection.typed_input(),
                });
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b.typed.cmp(&a.typed))
            .then_with(|| a.item.cmp(&b.item))
            .then_with(|| a.slot.cmp(&b.slot))
    });
    match candidates.first() {
        Some(best) => PlacementNext::Item {
            item: best.item.clone(),
            slot: best.slot,
        },
        None if placement.asked.is_empty() => PlacementNext::Unavailable,
        None => done(model, bundle, state, placement, now),
    }
}

/// Records a checked answer. Returns `false` if the item was already asked,
/// does not belong to the subject, or is not an eligible checked item.
pub fn record(
    bundle: &Bundle,
    placement: &mut Placement,
    item: ItemRef,
    slot: u16,
    correct: bool,
) -> bool {
    if placement.asked.iter().any(|a| a.item == item) {
        return false;
    }
    let Some(projection) = bundle.projection(&item) else {
        return false;
    };
    if projection.subject != placement.subject
        || !bundle
            .content
            .items_by_kc
            .values()
            .any(|items| items.primary.contains(&item))
    {
        return false;
    }
    let Some(row) = projection.slots.get(usize::from(slot)) else {
        return false;
    };
    if !row.kcs.iter().any(|k| k.role == KcRole::Primary) {
        return false;
    }
    placement.asked.push(PlacementItem { item, slot });
    if correct {
        placement.correct = placement.correct.saturating_add(1);
    }
    true
}

fn done(
    model: &KnowledgeModel,
    bundle: &Bundle,
    state: &State,
    placement: &Placement,
    now: Timestamp,
) -> PlacementNext {
    let level = result_level(model, bundle, state, &placement.subject, now);
    PlacementNext::Done(PlacementResult {
        level,
        start_level: start_level(level.as_ref(), bundle, &placement.subject),
        answered: answered(placement),
        source: PlacementSource::Diagnostic,
    })
}

fn beginner_exit(placement: &Placement, level: Option<&Level>) -> bool {
    placement.asked.len() >= BEGINNER_ITEMS
        && placement.correct <= BEGINNER_MAX_CORRECT
        && level.is_none_or(|l| l.point < BEGINNER_LEVEL)
}

fn measured(live: &[&KcEntry], state: &State) -> bool {
    live.iter().all(|e| {
        state
            .kcs
            .get(&e.kc.id)
            .is_some_and(|k| libm::sqrt(k.var) < STOP_SIGMA)
    })
}

fn score_slot(
    model: &KnowledgeModel,
    bundle: &Bundle,
    state: &State,
    item: &ItemRef,
    slot: u16,
    expected_time: u32,
    now: Timestamp,
) -> f64 {
    let Some(projection) = bundle.projection(item) else {
        return 0.0;
    };
    let Some(row) = projection.slots.get(usize::from(slot)) else {
        return 0.0;
    };
    let mut score = 0.0;
    for k in &row.kcs {
        let Some(entry) = bundle.content.kcs.iter().find(|e| e.kc.id == k.kc) else {
            continue;
        };
        let Some(info) = model.information(state, bundle, item, slot, &k.kc, now) else {
            continue;
        };
        let Some(belief) = model.belief(state, bundle, &k.kc, now) else {
            continue;
        };
        let v = belief.var + model.params.process_noise;
        let reduced = v - 1.0 / (1.0 / v + info);
        score += entry.kc.importance.get() * reduced;
    }
    score / f64::from(expected_time.max(1))
}

fn result_level(
    model: &KnowledgeModel,
    bundle: &Bundle,
    state: &State,
    subject: &SubjectId,
    now: Timestamp,
) -> Option<Level> {
    let domain = domain_for(bundle, subject)?;
    model.level(state, bundle, subject, &domain, now)
}

/// The level is needed only after the beginner threshold; computing it on
/// every short selection call is unnecessary work for the 100k benchmark.
fn level_for_next(
    model: &KnowledgeModel,
    bundle: &Bundle,
    state: &State,
    placement: &Placement,
    now: Timestamp,
) -> Option<Level> {
    if placement.asked.len() >= BEGINNER_ITEMS {
        result_level(model, bundle, state, &placement.subject, now)
    } else {
        None
    }
}

fn start_level(level: Option<&Level>, bundle: &Bundle, subject: &SubjectId) -> f64 {
    let Some(level) = level else {
        return 0.0;
    };
    let max = bundle
        .content
        .subjects
        .iter()
        .find(|s| &s.id == subject)
        .map_or(0.0, |s| {
            f64::from(
                u16::try_from(s.scale.levels.len())
                    .unwrap_or(u16::MAX)
                    .saturating_sub(1),
            )
        });
    level.point.clamp(0.0, max)
}

fn domain_for(bundle: &Bundle, subject: &SubjectId) -> Option<DomainId> {
    let mut counts: BTreeMap<&DomainId, usize> = BTreeMap::new();
    for e in &bundle.content.kcs {
        if e.status == KcStatus::Live && &e.kc.subject == subject {
            *counts.entry(&e.kc.domain).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(domain, _)| domain.clone())
}

fn live_kcs<'a>(bundle: &'a Bundle, subject: &SubjectId) -> Vec<&'a KcEntry> {
    bundle
        .content
        .kcs
        .iter()
        .filter(|e| e.status == KcStatus::Live && &e.kc.subject == subject)
        .filter(|e| {
            bundle
                .content
                .items_by_kc
                .get(&e.kc.id)
                .is_some_and(|items| !items.primary.is_empty())
        })
        .collect()
}

fn auto_checked(kind: ItemKindTag) -> bool {
    !matches!(
        kind,
        ItemKindTag::Writing | ItemKindTag::OwnSentence | ItemKindTag::Shadowing
    )
}

#[allow(clippy::cast_precision_loss)] // timestamps are within 2^53
fn recently_seen(state: &State, item: &ItemRef, unseen_days: f64, now: Timestamp) -> bool {
    state.seen.get(&item.id).is_some_and(|at| {
        let days = (now.0 - at.0).max(0) as f64 / 86_400_000.0;
        days < unseen_days
    })
}

fn answered(placement: &Placement) -> u16 {
    u16::try_from(placement.asked.len()).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn beginner_exit_needs_three_answers_and_few_correct() {
        let placement = Placement {
            subject: SubjectId::new("en").unwrap(),
            asked: Vec::new(),
            correct: 0,
        };
        assert!(!beginner_exit(&placement, None));
        let three = Placement {
            subject: SubjectId::new("en").unwrap(),
            asked: vec![
                PlacementItem {
                    item: ItemRef {
                        id: crate::ids::ItemId::new("a").unwrap(),
                        version: crate::ids::ItemVersion(1),
                        seed: None,
                    },
                    slot: 0,
                },
                PlacementItem {
                    item: ItemRef {
                        id: crate::ids::ItemId::new("b").unwrap(),
                        version: crate::ids::ItemVersion(1),
                        seed: None,
                    },
                    slot: 0,
                },
                PlacementItem {
                    item: ItemRef {
                        id: crate::ids::ItemId::new("c").unwrap(),
                        version: crate::ids::ItemVersion(1),
                        seed: None,
                    },
                    slot: 0,
                },
            ],
            correct: 1,
        };
        assert!(beginner_exit(&three, None));
    }
}
