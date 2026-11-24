//! Computed KC status (`docs/02-layer1-content.md`, «Единица знания и граф»).
//! A KC is live only if it has a reviewed short explanation in ru and en, its
//! live items' instructions and diagnosed misconceptions are reviewed in ru
//! and en (docs/02, «Вычитка и показ») and, among
//! live items where it is primary in at least one slot: ≥ 10 items, ≥ 3
//! families, ≥ 3 production items, and difficulties covering two adjacent
//! levels. Otherwise it stays draft and the report says what is missing.

use std::collections::{BTreeMap, BTreeSet};

use refl_core::bundle::KcStatus;
use refl_core::content::{ExplanationDepth, ExplanationTarget};
use refl_core::ids::KcId;
use refl_core::item::{Item, ItemStatus, KcRole};
use refl_core::kind::ItemKind;
use refl_core::value::Lang;

use crate::load::Content;
use crate::report::{DraftKc, Report};

const MIN_ITEMS: usize = 10;
const MIN_FAMILIES: usize = 3;
const MIN_PRODUCTION: usize = 3;

fn lang_name(lang: Lang) -> &'static str {
    match lang {
        Lang::Uz => "uz",
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

/// Why a short explanation of `target` in `lang` cannot be shown: missing, or
/// present but not reviewed by a named person.
fn explanation_gap(
    c: &Content,
    target: &ExplanationTarget,
    lang: Lang,
    what: &str,
) -> Option<String> {
    let found: Vec<_> = c
        .explanations
        .iter()
        .map(|e| &e.value)
        .filter(|e| &e.target == target && e.depth == ExplanationDepth::Short && e.lang == lang)
        .collect();
    let name = lang_name(lang);
    if found.is_empty() {
        Some(format!("no {what} in {name}"))
    } else if found.iter().all(|e| !e.review.is_shown()) {
        Some(format!("{what} in {name} not reviewed"))
    } else {
        None
    }
}

/// Instructions and micro-explanations of a KC's live items that are not
/// reviewed in ru and en: a draft is never shown (docs/02, «Вычитка и показ»).
fn review_gaps(c: &Content, kc: &KcId, items: &[&Item]) -> Vec<String> {
    let mut gaps = Vec::new();
    let instructions: BTreeSet<_> = items.iter().map(|i| &i.instruction).collect();
    for id in instructions {
        let Some(ins) = c
            .instructions
            .iter()
            .map(|i| &i.value)
            .find(|i| &i.id == id)
        else {
            continue;
        };
        for lang in [Lang::Ru, Lang::En] {
            if ins.text.get(lang).is_some() && !ins.review.is_shown(lang) {
                gaps.push(format!(
                    "instruction `{id}` not reviewed in {}",
                    lang_name(lang)
                ));
            }
        }
    }
    let diagnosed: BTreeSet<_> = items
        .iter()
        .flat_map(|i| i.body.slots())
        .filter(|s| {
            s.kcs
                .iter()
                .any(|k| &k.kc == kc && k.role == KcRole::Primary)
        })
        .flat_map(|s| {
            s.options
                .iter()
                .filter_map(|o| o.misconception.as_ref())
                .chain(s.expected_wrong.iter().map(|w| &w.misconception))
        })
        .collect();
    for m in diagnosed {
        for lang in [Lang::Ru, Lang::En] {
            gaps.extend(explanation_gap(
                c,
                &ExplanationTarget::Misconception(m.clone()),
                lang,
                &format!("micro-explanation of `{m}`"),
            ));
        }
    }
    gaps
}

pub fn compute(c: &Content, report: &mut Report) -> BTreeMap<KcId, KcStatus> {
    let mut first = BTreeMap::new();
    for k in &c.kcs {
        first.entry(&k.value.id).or_insert(&k.file);
    }
    let mut status = BTreeMap::new();
    for (&kc, &file) in &first {
        let mut missing = Vec::new();
        for lang in [Lang::Ru, Lang::En] {
            missing.extend(explanation_gap(
                c,
                &ExplanationTarget::Kc(kc.clone()),
                lang,
                "short explanation",
            ));
        }
        let items: Vec<_> = c
            .items
            .iter()
            .map(|i| &i.value)
            .filter(|i| i.status == ItemStatus::Live)
            .filter(|i| {
                i.body.slots().iter().any(|s| {
                    s.kcs
                        .iter()
                        .any(|k| &k.kc == kc && k.role == KcRole::Primary)
                })
            })
            .collect();
        missing.extend(review_gaps(c, kc, &items));
        let families: BTreeSet<_> = items.iter().map(|i| &i.family).collect();
        let production = items.iter().filter(|i| i.body.is_production()).count();
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
