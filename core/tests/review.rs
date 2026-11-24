//! The review gate (`docs/02-layer1-content.md`, «Вычитка и показ»): the
//! language chain interface → ru → en skips drafts, so a draft is never shown.

use refl_core::content::{
    Example, Explanation, ExplanationDepth, ExplanationTarget, Instruction, LangReviews, Review,
    ReviewStatus, explanation_for, language_chain,
};
use refl_core::ids::{ExplanationId, InstructionId, KcId};
use refl_core::value::{Lang, Localized};

fn review(status: ReviewStatus, by: Option<&str>) -> Review {
    Review {
        status,
        by: by.map(Into::into),
    }
}

fn reviewed() -> Review {
    review(ReviewStatus::Reviewed, Some("Hara"))
}

fn target() -> ExplanationTarget {
    ExplanationTarget::Kc(KcId::new("en.gram.article.first-mention").unwrap())
}

fn explanation(lang: Lang, review: Review) -> Explanation {
    Explanation {
        id: ExplanationId::new(format!("expl.x.{lang:?}").to_lowercase()).unwrap(),
        target: target(),
        depth: ExplanationDepth::Short,
        lang,
        body: format!("{lang:?}"),
        l1_contrast: None,
        examples: Vec::<Example>::new(),
        review,
    }
}

fn shown(all: &[Explanation], interface: Lang) -> Option<Lang> {
    explanation_for(all, &target(), ExplanationDepth::Short, interface).map(|e| e.lang)
}

#[test]
fn the_chain_is_interface_then_ru_then_en() {
    assert_eq!(language_chain(Lang::Uz), [Lang::Uz, Lang::Ru, Lang::En]);
    assert_eq!(language_chain(Lang::Ru), [Lang::Ru, Lang::En]);
    assert_eq!(language_chain(Lang::En), [Lang::En, Lang::Ru]);
}

#[test]
fn a_draft_is_skipped_down_the_chain() {
    let all = [
        explanation(Lang::Uz, review(ReviewStatus::Draft, None)),
        explanation(Lang::Ru, reviewed()),
        explanation(Lang::En, reviewed()),
    ];
    assert_eq!(shown(&all, Lang::Uz), Some(Lang::Ru));
    assert_eq!(shown(&all, Lang::Ru), Some(Lang::Ru));
    assert_eq!(shown(&all, Lang::En), Some(Lang::En));
}

#[test]
fn nothing_is_shown_when_everything_is_draft() {
    let all = [
        explanation(Lang::Ru, review(ReviewStatus::Draft, None)),
        explanation(Lang::En, review(ReviewStatus::Draft, Some("Hara"))),
    ];
    assert_eq!(shown(&all, Lang::Ru), None);
    assert_eq!(shown(&all, Lang::Uz), None);
}

#[test]
fn reviewed_without_a_reviewer_is_not_shown() {
    let all = [explanation(Lang::En, review(ReviewStatus::Reviewed, None))];
    assert_eq!(shown(&all, Lang::En), None);
}

#[test]
fn an_instruction_is_shown_per_language() {
    let instruction = Instruction {
        id: InstructionId::new("instr.x").unwrap(),
        text: Localized {
            uz: Some("uz".into()),
            ru: Some("ru".into()),
            en: Some("en".into()),
        },
        review: LangReviews {
            uz: None,
            ru: Some(reviewed()),
            en: Some(review(ReviewStatus::Draft, None)),
        },
    };
    assert_eq!(instruction.text_for(Lang::Uz), Some((Lang::Ru, "ru")));
    assert_eq!(instruction.text_for(Lang::Ru), Some((Lang::Ru, "ru")));
    // en interface: en is draft, so the chain goes on to ru.
    assert_eq!(instruction.text_for(Lang::En), Some((Lang::Ru, "ru")));
    let unreviewed = Instruction {
        review: LangReviews::default(),
        ..instruction
    };
    assert_eq!(unreviewed.text_for(Lang::Ru), None);
}

#[test]
fn a_kc_title_follows_the_chain_and_skips_drafts() {
    use refl_core::content::{Kc, localized_for};
    use refl_core::ids::{DomainId, SubjectId};
    use refl_core::subject::LevelKey;
    use refl_core::value::UnitInterval;

    let title = Localized {
        uz: Some("uz".into()),
        ru: Some("ru".into()),
        en: Some("en".into()),
    };
    let kc = Kc {
        id: KcId::new("en.gram.x").unwrap(),
        subject: SubjectId::new("en").unwrap(),
        domain: DomainId::new("grammar").unwrap(),
        title: title.clone(),
        review: LangReviews {
            uz: Some(review(ReviewStatus::Draft, None)),
            ru: None,
            en: Some(reviewed()),
        },
        level: LevelKey::new("A1").unwrap(),
        importance: UnitInterval::new(0.5).unwrap(),
        rule: String::new(),
        tags: Vec::new(),
    };
    // uz is draft and ru has no record, so every interface falls to en.
    for lang in [Lang::Uz, Lang::Ru, Lang::En] {
        assert_eq!(kc.title_for(lang), Some((Lang::En, "en")), "{lang:?}");
    }
    // Nothing reviewed, nothing shown.
    let none = LangReviews::default();
    assert_eq!(localized_for(&title, &none, Lang::Ru), None);
    // A language without text is skipped even if its record is reviewed.
    let only_ru = Localized {
        ru: Some("ru".into()),
        ..Localized::default()
    };
    let both = LangReviews {
        ru: Some(reviewed()),
        en: Some(reviewed()),
        ..LangReviews::default()
    };
    assert_eq!(
        localized_for(&only_ru, &both, Lang::En),
        Some((Lang::Ru, "ru"))
    );
}
