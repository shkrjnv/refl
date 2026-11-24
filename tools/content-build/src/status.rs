//! Computed KC status (`docs/02-layer1-content.md`, «Единица знания и граф»).
//! A KC is live only if it has a short explanation in ru and en and, among
//! live items where it is primary in at least one slot: ≥ 10 items, ≥ 3
//! families, ≥ 3 production items, and difficulties covering two adjacent
//! levels. Otherwise it stays draft and the report says what is missing.

use std::collections::{BTreeMap, BTreeSet};

use refl_core::bundle::KcStatus;
use refl_core::content::{ExplanationDepth, ExplanationTarget};
use refl_core::ids::KcId;
use refl_core::item::{ItemStatus, KcRole};
use refl_core::value::Lang;

use crate::load::Content;
use crate::report::{DraftKc, Report};

const MIN_ITEMS: usize = 10;
const MIN_FAMILIES: usize = 3;
const MIN_PRODUCTION: usize = 3;

pub fn compute(c: &Content, report: &mut Report) -> BTreeMap<KcId, KcStatus> {
    let mut first = BTreeMap::new();
    for k in &c.kcs {
        first.entry(&k.value.id).or_insert(&k.file);
    }
    let mut status = BTreeMap::new();
    for (&kc, &file) in &first {
        let mut missing = Vec::new();
        for lang in [Lang::Ru, Lang::En] {
            let has = c.explanations.iter().any(|e| {
                let e = &e.value;
                e.target == ExplanationTarget::Kc(kc.clone())
                    && e.depth == ExplanationDepth::Short
                    && e.lang == lang
            });
            if !has {
                let name = if lang == Lang::Ru { "ru" } else { "en" };
                missing.push(format!("no short explanation in {name}"));
            }
        }
        let items: Vec<_> = c
            .items
            .iter()
            .map(|i| &i.value)
            .filter(|i| i.status == ItemStatus::Live)
            .filter(|i| {
                i.slots.iter().any(|s| {
                    s.kcs
                        .iter()
                        .any(|k| &k.kc == kc && k.role == KcRole::Primary)
                })
            })
            .collect();
        let families: BTreeSet<_> = items.iter().map(|i| &i.family).collect();
        let production = items.iter().filter(|i| i.item_type.is_production()).count();
        let levels: BTreeSet<u8> = items.iter().map(|i| i.difficulty.get()).collect();
        let adjacent = levels.iter().any(|d| levels.contains(&(d + 1)));
        if items.len() < MIN_ITEMS {
            missing.push(format!("{} live item(s), needs {MIN_ITEMS}", items.len()));
        }
        if families.len() < MIN_FAMILIES {
            missing.push(format!(
                "{} family(ies), needs {MIN_FAMILIES}",
                families.len()
            ));
        }
        if production < MIN_PRODUCTION {
            missing.push(format!(
                "{production} production item(s), needs {MIN_PRODUCTION}"
            ));
        }
        if !adjacent {
            let levels: Vec<String> = levels.iter().map(u8::to_string).collect();
            missing.push(format!(
                "difficulty levels [{}], needs two adjacent",
                levels.join(", ")
            ));
        }
        if missing.is_empty() {
            report.live.push(kc.to_string());
            status.insert(kc.clone(), KcStatus::Live);
        } else {
            report.draft.push(DraftKc {
                kc: kc.to_string(),
                file: file.clone(),
                missing,
            });
            status.insert(kc.clone(), KcStatus::Draft);
        }
    }
    status
}
