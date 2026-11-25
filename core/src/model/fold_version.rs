//! The version of everything the fold reads from a bundle except the item
//! projections (`docs/02-layer1-content.md`, «Версия свёртки»). Snapshots key
//! on it: a bundle with the same `fold_version` gives the same state from the
//! same log, so nothing needs refolding when only texts, tasks or
//! instructions change. Tasks enter only through the family table
//! ([`Families`]).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::fluency::Families;
use crate::bundle::{Bundle, Required};
use crate::content::ExplanationTarget;
use crate::ids::{ExplanationId, FamilyId, InstructionId, KcId, MisconceptionId, SubjectId};

/// Bumped when the set of hashed inputs changes, so an old and a new
/// composition never collide. 2: the family table.
pub const FOLD_INPUTS_FORMAT: u32 = 2;

#[derive(Serialize)]
struct FamilyInput<'a> {
    id: &'a FamilyId,
    instruction: &'a InstructionId,
    beta: f64,
}

#[derive(Serialize)]
struct SubjectInput<'a> {
    id: &'a SubjectId,
    /// Logits between neighbouring levels: the starting μ.
    step_logit: f64,
    spelling_kc: &'a KcId,
}

#[derive(Serialize)]
struct KcInput<'a> {
    id: &'a KcId,
    subject: &'a SubjectId,
    level_rank: u16,
    importance: f64,
}

#[derive(Serialize)]
struct MisconceptionInput<'a> {
    id: &'a MisconceptionId,
    prior_uz: f64,
    prior_ru: f64,
}

#[derive(Serialize)]
struct ExplanationInput<'a> {
    id: &'a ExplanationId,
    kc: &'a KcId,
}

#[derive(Serialize)]
struct FoldInputs<'a> {
    format: u32,
    subjects: Vec<SubjectInput<'a>>,
    kcs: Vec<KcInput<'a>>,
    requires_closure: &'a BTreeMap<KcId, Vec<Required>>,
    misconceptions: Vec<MisconceptionInput<'a>>,
    explanation_targets: Vec<ExplanationInput<'a>>,
    families: Vec<FamilyInput<'a>>,
}

/// The fold version of `bundle`, hex SHA-256 of the canonical JSON of the
/// fold's inputs. Bundle collections are already sorted; the lists here keep
/// that order.
///
/// # Panics
/// Never: the inputs always serialize.
pub fn fold_version(bundle: &Bundle) -> String {
    let c = &bundle.content;
    let families = Families::of(bundle);
    let inputs = FoldInputs {
        format: FOLD_INPUTS_FORMAT,
        subjects: c
            .subjects
            .iter()
            .map(|s| SubjectInput {
                id: &s.id,
                step_logit: s.scale.step_logit.get(),
                spelling_kc: &s.spelling_kc,
            })
            .collect(),
        kcs: c
            .kcs
            .iter()
            .map(|e| KcInput {
                id: &e.kc.id,
                subject: &e.kc.subject,
                level_rank: e.level_rank,
                importance: e.kc.importance.get(),
            })
            .collect(),
        requires_closure: &c.requires_closure,
        misconceptions: c
            .misconceptions
            .iter()
            .map(|m| MisconceptionInput {
                id: &m.id,
                prior_uz: m.prior.uz.get(),
                prior_ru: m.prior.ru.get(),
            })
            .collect(),
        explanation_targets: c
            .explanations
            .iter()
            .filter_map(|e| match &e.target {
                ExplanationTarget::Kc(kc) => Some(ExplanationInput { id: &e.id, kc }),
                ExplanationTarget::Misconception(_) => None,
            })
            .collect(),
        families: families
            .iter()
            .map(|(id, f)| FamilyInput {
                id,
                instruction: &f.instruction,
                beta: f.beta,
            })
            .collect(),
    };
    let json = serde_json::to_vec(&inputs).expect("fold inputs serialize");
    Sha256::digest(json).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}
