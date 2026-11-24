//! Item kinds (`docs/10-subjects.md`, «Вид задания: трейт и данные»): every
//! `ItemBody` variant keeps the behaviour the old `ItemType` matches had, and
//! `check` follows docs/02 («Проверка ответов на ввод») and the canonical
//! option index.

use refl_core::answer::dfa::{Dfa, TokenTable};
use refl_core::answer::normalize::NormalizeOptions;
use refl_core::answer::template::{NormPattern, Pattern};
use refl_core::bundle::{ErrorAutomaton, SlotAutomata};
use refl_core::event::{Response, Verdict};
use refl_core::ids::{
    FamilyId, InstructionId, ItemId, ItemVersion, KcId, MisconceptionId, SubjectId,
};
use refl_core::item::{
    AnswerPattern, AuthorDifficulty, ChoiceBody, ChoiceOption, DictationBody, EnglishCheck,
    ExpectedWrong, FindErrorBody, GapFillBody, Item, ItemBody, ItemKindTag, ItemStatus, KcRole,
    LadderLevel, MatchingBody, OwnSentenceBody, SentenceAssemblyBody, ShadowingBody, Slot, SlotKc,
    Stimulus, TransformationBody, TranslationBody, TrueFalseNotGivenBody, WritingBody,
};
use refl_core::kind::{CheckCtx, CheckOutcome, Compiled, ItemKind, QualityCtx, QualityIssueKind};
use refl_core::lexicon::Lexicon;
use refl_core::projection::ItemProjection;
use refl_core::value::Localized;

fn mis(s: &str) -> MisconceptionId {
    MisconceptionId::new(s).unwrap()
}

/// A slot answered "a cat", with two diagnosed distractors, one noise option
/// and two error patterns.
fn slot() -> Slot {
    Slot {
        kcs: vec![SlotKc {
            kc: KcId::new("en.k.a").unwrap(),
            role: KcRole::Primary,
        }],
        answer: "a cat".into(),
        accepted: vec![AnswerPattern("(a|one) cat".into())],
        options: vec![
            ChoiceOption {
                text: "the cat".into(),
                misconception: Some(mis("en.m.the")),
                confuses_with: None,
            },
            ChoiceOption {
                text: "cat".into(),
                misconception: Some(mis("en.m.omission")),
                confuses_with: None,
            },
            ChoiceOption {
                text: "an cat".into(),
                misconception: None,
                confuses_with: None,
            },
        ],
        expected_wrong: vec![
            ExpectedWrong {
                pattern: AnswerPattern("cat".into()),
                misconception: mis("en.m.omission"),
                confuses_with: None,
            },
            ExpectedWrong {
                pattern: AnswerPattern("the cat".into()),
                misconception: mis("en.m.the"),
                confuses_with: None,
            },
        ],
        hint: None,
        why: Localized::default(),
    }
}

fn stimulus() -> Stimulus {
    Stimulus {
        text: "I have ___.".into(),
        audio: None,
        image: None,
    }
}

/// One body of every kind, with `slots` and `check`.
fn every_kind(slots: &[Slot], check: EnglishCheck) -> Vec<ItemBody> {
    let s = || slots.to_vec();
    vec![
        ItemBody::Choice(ChoiceBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::FindError(FindErrorBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::Matching(MatchingBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::TrueFalseNotGiven(TrueFalseNotGivenBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::GapFill(GapFillBody {
            stimulus: stimulus(),
            slots: s(),
            check,
        }),
        ItemBody::Transformation(TransformationBody {
            stimulus: stimulus(),
            slots: s(),
            check,
        }),
        ItemBody::SentenceAssembly(SentenceAssemblyBody {
            stimulus: stimulus(),
            slots: s(),
            check,
        }),
        ItemBody::Translation(TranslationBody {
            stimulus: stimulus(),
            slots: s(),
            check,
        }),
        ItemBody::Dictation(DictationBody {
            stimulus: stimulus(),
            slots: s(),
            check,
        }),
        ItemBody::OwnSentence(OwnSentenceBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::Writing(WritingBody {
            stimulus: stimulus(),
            slots: s(),
        }),
        ItemBody::Shadowing(ShadowingBody {
            stimulus: stimulus(),
            slots: s(),
        }),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OldRule {
    Choice,
    TypedInput,
    None,
}

/// `ItemType::ladder_level`, `is_production` and `quality_rule` as they were
/// before the kinds moved onto `ItemKind`.
fn old_behaviour(tag: ItemKindTag) -> (LadderLevel, bool, OldRule) {
    use ItemKindTag as T;
    use LadderLevel as L;
    match tag {
        T::Choice | T::FindError => (L::Recognition, false, OldRule::Choice),
        T::Matching | T::TrueFalseNotGiven => (L::Recognition, false, OldRule::None),
        T::GapFill | T::Transformation | T::Translation => (L::Guided, true, OldRule::TypedInput),
        T::SentenceAssembly | T::Dictation => (L::Guided, false, OldRule::TypedInput),
        T::OwnSentence | T::Writing => (L::Free, true, OldRule::None),
        T::Shadowing => (L::Pronunciation, false, OldRule::None),
    }
}

/// The rule a kind applies, read from what it reports on a slot that breaks
/// both counting rules (no options, no error patterns).
fn observed_rule(body: &ItemBody) -> OldRule {
    let issues = body
        .quality(&QualityCtx { compiled: None })
        .expect_err("an empty slot breaks every rule");
    let kinds: Vec<_> = issues.iter().map(|i| i.kind).collect();
    match kinds.as_slice() {
        [QualityIssueKind::TooFewDistractors] => OldRule::Choice,
        [QualityIssueKind::TooFewErrorPatterns] => OldRule::TypedInput,
        [QualityIssueKind::NoRule] => OldRule::None,
        other => panic!("unexpected issues {other:?}"),
    }
}

#[test]
fn every_kind_keeps_its_old_ladder_production_and_rule() {
    let bare = Slot {
        options: vec![],
        expected_wrong: vec![],
        ..slot()
    };
    let bodies = every_kind(&[bare], EnglishCheck::default());
    assert_eq!(bodies.len(), 12);
    for body in &bodies {
        let (ladder, production, rule) = old_behaviour(body.tag());
        assert_eq!(body.ladder(), ladder, "{:?}", body.tag());
        assert_eq!(body.is_production(), production, "{:?}", body.tag());
        assert_eq!(observed_rule(body), rule, "{:?}", body.tag());
    }
}

/// Guessing rate g and typed input, the inputs of the learner model
/// (`docs/03-layer2-learner-model.md`): `1/n` for a choice among the answer
/// and its options, 0 for typed input and for kinds that cannot be live yet.
#[test]
fn every_kind_states_its_guessing_rate_and_input() {
    let options = slot().options.len();
    for body in every_kind(&[slot()], EnglishCheck::default()) {
        let tag = body.tag();
        let (guessing, typed) = match tag {
            ItemKindTag::Choice | ItemKindTag::FindError => {
                (1.0 / f64::from(u32::try_from(options + 1).unwrap()), false)
            }
            ItemKindTag::GapFill
            | ItemKindTag::Transformation
            | ItemKindTag::SentenceAssembly
            | ItemKindTag::Translation
            | ItemKindTag::Dictation
            | ItemKindTag::OwnSentence
            | ItemKindTag::Writing => (0.0, true),
            ItemKindTag::Matching | ItemKindTag::TrueFalseNotGiven | ItemKindTag::Shadowing => {
                (0.0, false)
            }
        };
        assert!(
            (body.guessing(0) - guessing).abs() < f64::EPSILON,
            "{tag:?}"
        );
        assert_eq!(body.typed_input(), typed, "{tag:?}");
        // No such slot: no guessing.
        assert!(body.guessing(9).abs() < f64::EPSILON, "{tag:?}");
    }
}

/// A stored projection answers the model's questions exactly as the item
/// does, so the fold may read the projection of any version.
#[test]
fn projection_answers_like_the_item() {
    for body in every_kind(&[slot()], EnglishCheck::default()) {
        let tag = body.tag();
        let item = Item {
            id: ItemId::new("en.item.x").unwrap(),
            version: ItemVersion(1),
            subject: SubjectId::new("en").unwrap(),
            family: FamilyId::new("en.fam.x").unwrap(),
            instruction: InstructionId::new("en.instr.x").unwrap(),
            difficulty: AuthorDifficulty::new(3).unwrap(),
            expected_time: 10,
            topic: None,
            status: ItemStatus::Live,
            source: None,
            license: None,
            breaks_calibration: false,
            review: refl_core::content::LangReviews::default(),
            review_group: None,
            body: body.clone(),
        };
        let p = ItemProjection::of(&item);
        assert_eq!(p.kind, tag);
        assert_eq!(p.ladder(), body.ladder(), "{tag:?}");
        assert_eq!(p.is_production(), body.is_production(), "{tag:?}");
        assert_eq!(p.typed_input(), body.typed_input(), "{tag:?}");
        for slot in [0, 9] {
            assert!(
                (p.guessing(slot) - body.guessing(slot)).abs() < f64::EPSILON,
                "{tag:?} slot {slot}"
            );
        }
        let m = &p.slots[0].misconceptions;
        assert!(m.windows(2).all(|w| w[0] < w[1]), "ascending, no repeats");
        assert!(m.len() >= 2, "{m:?}");
        assert_eq!(p.slots[0].kcs, item.body.slots()[0].kcs);
    }
}

#[test]
fn tag_is_the_serialized_type() {
    for body in every_kind(&[slot()], EnglishCheck::default()) {
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["type"], serde_json::to_value(body.tag()).unwrap());
    }
}

/// Compiles a slot's templates the way content-build does.
fn compile(slot: &Slot, opts: NormalizeOptions) -> (Vec<SlotAutomata>, TokenTable) {
    let norm = |p: &AnswerPattern| -> NormPattern {
        Pattern::parse(&p.0).unwrap().normalize(opts).unwrap()
    };
    let accepted: Vec<NormPattern> = slot.accepted.iter().map(norm).collect();
    let wrong: Vec<(MisconceptionId, NormPattern)> = slot
        .expected_wrong
        .iter()
        .map(|w| (w.misconception.clone(), norm(&w.pattern)))
        .collect();
    let tokens = TokenTable::new(
        accepted
            .iter()
            .chain(wrong.iter().map(|(_, p)| p))
            .flat_map(NormPattern::tokens),
    );
    let automata = SlotAutomata {
        accepted: Dfa::compile(&accepted, &tokens).unwrap(),
        expected_wrong: wrong
            .iter()
            .map(|(m, p)| ErrorAutomaton {
                misconception: m.clone(),
                dfa: Dfa::compile(std::slice::from_ref(p), &tokens).unwrap(),
            })
            .collect(),
    };
    (vec![automata], tokens)
}

fn checked(verdict: Verdict, misconception: Option<&str>) -> CheckOutcome {
    CheckOutcome::Checked {
        verdict,
        misconception: misconception.map(mis),
    }
}

fn text(s: &str) -> Response {
    Response::Text(s.into())
}

#[test]
fn typed_input_runs_accepted_then_expected_errors_then_unrecognized() {
    let check = EnglishCheck::default();
    let (slots, tokens) = compile(&slot(), NormalizeOptions::from(check));
    let lex = lexicon();
    let ctx = CheckCtx {
        lexicon: &lex,
        compiled: Compiled {
            slots: &slots,
            tokens: &tokens,
        },
    };
    for body in every_kind(&[slot()], check) {
        if body.english_check().is_none() {
            continue;
        }
        let run = |slot: u16, r: &Response| body.check(slot, r, &ctx);
        assert_eq!(run(0, &text("A  cat")), checked(Verdict::Correct, None));
        assert_eq!(run(0, &text("one cat.")), checked(Verdict::Correct, None));
        assert_eq!(
            run(0, &text("cat")),
            checked(Verdict::Wrong, Some("en.m.omission"))
        );
        assert_eq!(
            run(0, &text("The cat")),
            checked(Verdict::Wrong, Some("en.m.the"))
        );
        assert_eq!(run(0, &text("a dog")), checked(Verdict::Unrecognized, None));
        // A choice response or a slot the item does not have is not recognized.
        assert_eq!(
            run(0, &Response::Option(0)),
            checked(Verdict::Unrecognized, None)
        );
        assert_eq!(run(1, &text("a cat")), checked(Verdict::Unrecognized, None));
    }
}

#[test]
fn typed_input_honours_its_english_check() {
    let check = EnglishCheck {
        case_sensitive: true,
        ..EnglishCheck::default()
    };
    let (slots, tokens) = compile(&slot(), NormalizeOptions::from(check));
    let lex = lexicon();
    let ctx = CheckCtx {
        lexicon: &lex,
        compiled: Compiled {
            slots: &slots,
            tokens: &tokens,
        },
    };
    let body = ItemBody::GapFill(GapFillBody {
        stimulus: stimulus(),
        slots: vec![slot()],
        check,
    });
    assert_eq!(
        body.check(0, &text("a cat"), &ctx),
        checked(Verdict::Correct, None)
    );
    assert_eq!(
        body.check(0, &text("A cat"), &ctx),
        checked(Verdict::Unrecognized, None)
    );
}

#[test]
fn choice_compares_the_canonical_option_index() {
    let (slots, tokens) = compile(&slot(), NormalizeOptions::default());
    let lex = lexicon();
    let ctx = CheckCtx {
        lexicon: &lex,
        compiled: Compiled {
            slots: &slots,
            tokens: &tokens,
        },
    };
    for body in &every_kind(&[slot()], EnglishCheck::default())[..2] {
        let run = |r: &Response| body.check(0, r, &ctx);
        // 0 is the answer; 1.. are the options in authored order.
        assert_eq!(run(&Response::Option(0)), checked(Verdict::Correct, None));
        assert_eq!(
            run(&Response::Option(1)),
            checked(Verdict::Wrong, Some("en.m.the"))
        );
        assert_eq!(
            run(&Response::Option(2)),
            checked(Verdict::Wrong, Some("en.m.omission"))
        );
        assert_eq!(run(&Response::Option(3)), checked(Verdict::Wrong, None));
        assert_eq!(
            run(&Response::Option(4)),
            checked(Verdict::Unrecognized, None)
        );
        assert_eq!(run(&text("a cat")), checked(Verdict::Unrecognized, None));
        assert_eq!(
            body.check(1, &Response::Option(0), &ctx),
            checked(Verdict::Unrecognized, None)
        );
    }
}

#[test]
fn kinds_without_a_check_say_so() {
    let (slots, tokens) = compile(&slot(), NormalizeOptions::default());
    let lex = lexicon();
    let ctx = CheckCtx {
        lexicon: &lex,
        compiled: Compiled {
            slots: &slots,
            tokens: &tokens,
        },
    };
    for body in every_kind(&[slot()], EnglishCheck::default()) {
        let not_checked = matches!(
            body.tag(),
            ItemKindTag::Matching
                | ItemKindTag::TrueFalseNotGiven
                | ItemKindTag::OwnSentence
                | ItemKindTag::Writing
                | ItemKindTag::Shadowing
        );
        let outcome = body.check(0, &text("a cat"), &ctx);
        assert_eq!(
            outcome == CheckOutcome::NotChecked,
            not_checked,
            "{:?}",
            body.tag()
        );
    }
}

#[test]
fn choice_rule_checks_distractors_in_normalized_space() {
    let mut s = slot();
    // Accepted after normalization, and a repeat of options[0].
    s.options.push(ChoiceOption {
        text: "One  cat".into(),
        misconception: None,
        confuses_with: None,
    });
    s.options.push(ChoiceOption {
        text: "The cat.".into(),
        misconception: None,
        confuses_with: None,
    });
    let (slots, tokens) = compile(&s, NormalizeOptions::default());
    let compiled = Some(Compiled {
        slots: &slots,
        tokens: &tokens,
    });
    let body = ItemBody::Choice(ChoiceBody {
        stimulus: stimulus(),
        slots: vec![s],
    });
    let kinds: Vec<_> = body
        .quality(&QualityCtx { compiled })
        .unwrap_err()
        .iter()
        .map(|i| i.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            QualityIssueKind::DistractorAccepted,
            QualityIssueKind::DuplicateDistractor
        ]
    );
    // The well-formed slot passes.
    let (slots, tokens) = compile(&slot(), NormalizeOptions::default());
    let ok = ItemBody::Choice(ChoiceBody {
        stimulus: stimulus(),
        slots: vec![slot()],
    });
    let ctx = QualityCtx {
        compiled: Some(Compiled {
            slots: &slots,
            tokens: &tokens,
        }),
    };
    assert_eq!(ok.quality(&ctx), Ok(()));
}

/// A small lexicon: every word of the slot plus neighbours that are real
/// words one edit away.
fn lexicon() -> Lexicon {
    Lexicon::from_words(["a", "an", "cat", "cot", "one", "the", "then", "than"].map(String::from))
}

#[test]
fn typed_input_classifies_typos_against_accepted_answers() {
    let check = EnglishCheck::default();
    let (slots, tokens) = compile(&slot(), NormalizeOptions::from(check));
    let lex = lexicon();
    let ctx = CheckCtx {
        lexicon: &lex,
        compiled: Compiled {
            slots: &slots,
            tokens: &tokens,
        },
    };
    let body = ItemBody::GapFill(GapFillBody {
        stimulus: stimulus(),
        slots: vec![slot()],
        check,
    });
    let run = |s: &str| body.check(0, &text(s), &ctx);
    let typo = checked(Verdict::Typo, None);
    let unrecognized = checked(Verdict::Unrecognized, None);
    assert_eq!(run("a catt"), typo);
    assert_eq!(run("one ct."), typo);
    // Every differing word is a typo: both words here.
    assert_eq!(run("oen catt"), typo);
    // A real word one edit away is not a typo, nor is a lexicon word in
    // place of the article.
    assert_eq!(run("a cot"), unrecognized);
    assert_eq!(run("an cat"), unrecognized);
    // Digits never make a typo; a different word count is not one either.
    assert_eq!(run("a ca7"), unrecognized);
    assert_eq!(run("a catt catt"), unrecognized);
    assert_eq!(run("catt"), unrecognized);
    // Three edits are too many.
    assert_eq!(run("a cattle"), unrecognized);
    // Expected errors still win over alignment.
    assert_eq!(run("the cat"), checked(Verdict::Wrong, Some("en.m.the")));
}
