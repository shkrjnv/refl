//! `fold_version` (`docs/02-layer1-content.md`, «Версия свёртки»): a hash of
//! what the fold reads from a bundle except item projections. Editing a text
//! keeps it, and so does adding a task that leaves its family's row alone; a
//! prior, a `requires` edge, a rank, an importance, a subject parameter or a
//! family's instruction or βᶠ prior changes it.

use refl_core::bundle::{Bundle, Required};
use refl_core::content::{
    Example, LangReviews, Lexeme, LexemePos, LexemeSense, Refutation, Representation,
};
use refl_core::ids::{InstructionId, ItemId, ItemVersion, KcId, LexemeId};
use refl_core::item::ItemBody;
use refl_core::model::fold_version;
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use refl_core::value::{Finite, Localized, UnitInterval};
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

/// The bundle with a copy of its first item (a choice of family
/// `en.fam.k00.0`, whose slots give 8, 8, 12 and 12 s: βᶠ = ln 10) taking
/// `expected_time` seconds.
fn with_task(base: &Bundle, expected_time: u32) -> Bundle {
    let mut b = base.clone();
    let mut entry = b.content.items[0].clone();
    entry.item.id = ItemId::new("en.item.extra").unwrap();
    entry.item.expected_time = expected_time;
    b.content.item_versions.push(ItemVersionEntry {
        item: entry.item.id.clone(),
        version: entry.item.version,
        projection: ItemProjection::of(&entry.item),
    });
    b.content.items.push(entry);
    b
}

#[test]
fn adding_a_task_keeps_it_unless_its_family_row_changes() {
    let base = bundle();
    // 8, 8, 10, 12, 12: the median stays 10.
    assert_eq!(fold_version(&with_task(&base, 10)), fold_version(&base));
    // 8, 8, 8, 12, 12: βᶠ becomes ln 8.
    assert_ne!(fold_version(&with_task(&base, 8)), fold_version(&base));
}

#[test]
fn the_family_table_changes_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.items[0].item.expected_time += 20;
    b.content.items[2].item.expected_time += 20;
    assert_ne!(fold_version(&b), fold_version(&base));
    // The reference answer of a typed item is its motor time.
    let mut b = base.clone();
    for i in [1, 3] {
        let ItemBody::GapFill(gap) = &mut b.content.items[i].item.body else {
            panic!("odd items are gap fills");
        };
        gap.slots[0].answer = "the cat that sat on the mat".into();
    }
    assert_ne!(fold_version(&b), fold_version(&base));
    let mut b = base.clone();
    b.content.items[0].item.instruction = InstructionId::new("en.instr.other").unwrap();
    assert_ne!(fold_version(&b), fold_version(&base));
}

#[test]
fn editing_a_stimulus_keeps_it() {
    let base = bundle();
    let mut b = base.clone();
    let ItemBody::Choice(choice) = &mut b.content.items[0].item.body else {
        panic!("item 0 is a choice");
    };
    choice.stimulus.text.push_str(" Edited.");
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

/// The optional pedagogy fields are presentation/authoring data; they do not
/// enter the fold's inputs and must not force a refold.
#[test]
fn pedagogical_fields_keep_the_fold_version() {
    let base = bundle();
    let mut b = base.clone();
    b.content.misconceptions[0]
        .refutations
        .push(Refutation::Contrast {
            correct: "I have lost my keys.".into(),
            incorrect: "I lost my keys.".into(),
            difference: Localized {
                uz: None,
                ru: None,
                en: None,
            },
        });
    b.content.explanations[0].examples.push(Example {
        text: "I have a cat.".into(),
        audio: None,
        representation: Some(Representation::Pictorial),
    });
    b.content.items[0].item.representation = Some(Representation::Concrete);
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

fn sample_lexeme() -> Lexeme {
    Lexeme {
        id: LexemeId::new("en.lex.cat").unwrap(),
        lemma: "cat".into(),
        pos: LexemePos::Noun,
        frequency_rank: 100,
        level: refl_core::subject::LevelKey::new("A1").unwrap(),
        forms: vec!["cats".into()],
        senses: vec![LexemeSense {
            id: LexemeId::new("en.lex.cat.animal").unwrap(),
            definition_en: "a small animal".into(),
            gloss_uz: String::new(),
            gloss_ru: String::new(),
            examples: Vec::new(),
            collocations: Vec::new(),
            ipa_uk: String::new(),
            ipa_us: String::new(),
            audio_key: None,
            review: LangReviews::default(),
        }],
        kc_ids: Vec::new(),
        source: "test".into(),
        license: "original".into(),
        review: LangReviews::default(),
    }
}

#[test]
fn the_lexeme_bank_and_item_index_change_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.lexemes.push(sample_lexeme());
    assert_ne!(fold_version(&b), fold_version(&base));
    // The frequency rank is part of the placement prior, so it matters.
    let mut b = base.clone();
    let mut lexeme = sample_lexeme();
    lexeme.frequency_rank += 1;
    b.content.lexemes.push(lexeme);
    assert_ne!(fold_version(&b), fold_version(&base));
    // The item→sense index is what passive credit reads.
    let mut b = base.clone();
    b.content.item_lexemes.insert(
        b.content.items[0].item.id.clone(),
        vec![LexemeId::new("en.lex.cat.animal").unwrap()],
    );
    assert_ne!(fold_version(&b), fold_version(&base));
}

#[test]
fn graph_layout_does_not_change_it() {
    let base = bundle();
    let mut b = base.clone();
    b.content.kcs[0].layout_x = 1000;
    b.content.kcs[1].layout_y = 1000;
    assert_eq!(fold_version(&b), fold_version(&base));
}
