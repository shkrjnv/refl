//! `fold_version` (`docs/02-layer1-content.md`, «Версия свёртки»): a hash of
//! what the fold reads from a bundle except item projections. Adding a task
//! or editing a text keeps it; a prior, a `requires` edge, a rank, an
//! importance or a subject parameter changes it.

use refl_core::bundle::{Bundle, Required};
use refl_core::ids::{ItemId, ItemVersion, KcId};
use refl_core::model::fold_version;
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use refl_core::value::{Finite, UnitInterval};
use refl_sim::{BundleSpec, kc_id, synthetic_bundle};

fn bundle() -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 6,
        items_per_kc: 4,
    })
}

#[test]
fn it_is_deterministic_and_a_sha256() {
    let b = bundle();
    let v = fold_version(&b);
    assert_eq!(v, fold_version(&bundle()));
    assert_eq!(v.len(), 64);
    assert!(v.bytes().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn adding_a_task_keeps_it() {
    let base = bundle();
    let mut b = base.clone();
    let mut entry = b.content.items[0].clone();
    entry.item.id = ItemId::new("en.item.extra").unwrap();
    b.content.item_versions.push(ItemVersionEntry {
        item: entry.item.id.clone(),
        version: entry.item.version,
        projection: ItemProjection::of(&entry.item),
    });
    b.content.items.push(entry);
    assert_eq!(fold_version(&b), fold_version(&base));
}

#[test]
fn a_new_version_of_a_task_keeps_it() {
    let base = bundle();
    let mut b = base.clone();
    let old = b.content.item_versions[0].clone();
    b.content.item_versions.push(ItemVersionEntry {
        version: ItemVersion(old.version.0 + 1),
        ..old
    });
    assert_eq!(fold_version(&b), fold_version(&base));
}

#[test]
fn editing_a_text_keeps_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.explanations[0].body.push_str(" Edited.");
    b.content.kcs[1].kc.rule.push_str(" Edited.");
    b.content.misconceptions[0].title.en = Some("Edited".into());
    b.content.instructions.clear();
    assert_eq!(fold_version(&b), fold_version(&base));
}

#[test]
fn a_prior_changes_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.misconceptions[0].prior.uz = UnitInterval::new(0.99).unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
    let mut b = base.clone();
    b.content.misconceptions[0].prior.ru = UnitInterval::new(0.99).unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
}

#[test]
fn a_requires_edge_changes_it() {
    let base = bundle();
    // A new `requires` edge shows up in the closure: k00 now requires k05.
    let mut b = base.clone();
    b.content
        .requires_closure
        .get_mut(&kc_id(0))
        .unwrap()
        .push(Required {
            kc: kc_id(5),
            distance: 1,
        });
    assert_ne!(fold_version(&b), fold_version(&base));
    // The distance matters, not only the set.
    let mut b = base.clone();
    let list = b.content.requires_closure.get_mut(&kc_id(2)).unwrap();
    list[0].distance += 1;
    assert_ne!(fold_version(&b), fold_version(&base));
}

#[test]
fn ranks_importance_and_subject_parameters_change_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.kcs[1].level_rank += 1;
    assert_ne!(fold_version(&b), fold_version(&base));
    let mut b = base.clone();
    b.content.kcs[1].kc.importance = UnitInterval::new(0.01).unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
    let mut b = base.clone();
    b.content.subjects[0].scale.step_logit = Finite::new(1.3).unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
    let mut b = base.clone();
    b.content.subjects[0].spelling_kc = KcId::new("en.gram.k00").unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
}

#[test]
fn adding_a_kc_or_an_explanation_target_changes_it() {
    let base = bundle();
    let mut b = base.clone();
    let mut kc = b.content.kcs[1].clone();
    kc.kc.id = KcId::new("en.gram.extra").unwrap();
    b.content.kcs.push(kc);
    assert_ne!(fold_version(&b), fold_version(&base));
    // The explanation viewed event marks its KC as explained.
    let mut b = base.clone();
    let mut e = b.content.explanations[0].clone();
    e.id = refl_core::ids::ExplanationId::new("expl.extra").unwrap();
    b.content.explanations.push(e);
    assert_ne!(fold_version(&b), fold_version(&base));
}
