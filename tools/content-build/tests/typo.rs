//! The typo rule against the real SCOWL lexicon (`docs/02-layer1-content.md`,
//! «Проверка ответов на ввод», «Словарь проверщика»), and its time budget:
//! one check under 5 ms. Needs `REFL_SCOWL_DICT` (the dev shell and
//! `nix flake check` set it).

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use content_build::lexicon::{load_scowl, reference};
use refl_core::answer::dfa::{Dfa, TokenTable};
use refl_core::answer::normalize::NormalizeOptions;
use refl_core::answer::template::{NormPattern, Pattern};
use refl_core::bundle::SlotAutomata;
use refl_core::event::{Response, Verdict};
use refl_core::ids::KcId;
use refl_core::item::{
    AnswerPattern, EnglishCheck, GapFillBody, ItemBody, KcRole, Slot, SlotKc, Stimulus,
};
use refl_core::kind::{CheckCtx, CheckOutcome, Compiled, ItemKind};
use refl_core::lexicon::Lexicon;
use refl_core::value::Localized;

fn lexicon() -> &'static Lexicon {
    static LEXICON: OnceLock<Lexicon> = OnceLock::new();
    LEXICON.get_or_init(|| {
        let dir = std::env::var_os("REFL_SCOWL_DICT").expect("REFL_SCOWL_DICT is set");
        load_scowl(&PathBuf::from(dir)).unwrap()
    })
}

/// A gap-fill item with one slot accepting `accepted`, compiled.
struct Compiled1 {
    body: ItemBody,
    slots: Vec<SlotAutomata>,
    tokens: TokenTable,
}

fn item(accepted: &str) -> Compiled1 {
    let check = EnglishCheck::default();
    let pattern = Pattern::parse(accepted)
        .unwrap()
        .normalize(NormalizeOptions::from(check))
        .unwrap();
    let tokens = TokenTable::new(NormPattern::tokens(&pattern));
    let slots = vec![SlotAutomata {
        accepted: Dfa::compile(std::slice::from_ref(&pattern), &tokens).unwrap(),
        expected_wrong: vec![],
    }];
    let slot = Slot {
        kcs: vec![SlotKc {
            kc: KcId::new("en.k.a").unwrap(),
            role: KcRole::Primary,
        }],
        answer: accepted.into(),
        accepted: vec![AnswerPattern(accepted.into())],
        options: vec![],
        expected_wrong: vec![],
        hint: None,
        why: Localized::default(),
    };
    let body = ItemBody::GapFill(GapFillBody {
        stimulus: Stimulus {
            text: "___".into(),
            audio: None,
            image: None,
        },
        slots: vec![slot],
        check,
    });
    Compiled1 {
        body,
        slots,
        tokens,
    }
}

fn verdict(accepted: &str, answer: &str) -> Verdict {
    let it = item(accepted);
    let ctx = CheckCtx {
        compiled: Compiled {
            slots: &it.slots,
            tokens: &it.tokens,
        },
        lexicon: lexicon(),
    };
    match it.body.check(0, &Response::Text(answer.into()), &ctx) {
        CheckOutcome::Checked { verdict, .. } => verdict,
        CheckOutcome::NotChecked => panic!("gap fill is checked"),
    }
}

#[test]
fn real_words_are_not_typos() {
    for (accepted, answer) in [
        ("than", "then"),
        ("taller than me", "taller then me"),
        ("from", "form"),
        ("its", "it's"),
        ("it's", "its"),
    ] {
        assert_eq!(
            verdict(accepted, answer),
            Verdict::Unrecognized,
            "{answer} for {accepted}"
        );
    }
}

#[test]
fn misspellings_are_typos() {
    for (accepted, answer) in [
        ("the", "teh"),
        ("receive", "recieve"),
        ("I will receive it", "I will recieve it"),
        (
            "(I have|I've) been to the museum",
            "I've been to teh musuem",
        ),
    ] {
        assert_eq!(
            verdict(accepted, answer),
            Verdict::Typo,
            "{answer} for {accepted}"
        );
    }
}

#[test]
fn lexicon_size_and_check_latency() {
    let lex = lexicon();
    let r = reference(lex);
    assert!(r.words > 100_000, "{} words", r.words);
    let it =
        item("(I have|I've) [already] (seen|watched) (this|that) (film|movie) (twice|two times)");
    let ctx = CheckCtx {
        compiled: Compiled {
            slots: &it.slots,
            tokens: &it.tokens,
        },
        lexicon: lex,
    };
    let answer = Response::Text("I've alredy watched thsi movei two tiems".into());
    let runs = 200;
    let start = Instant::now();
    for _ in 0..runs {
        assert_eq!(
            it.body.check(0, &answer, &ctx),
            CheckOutcome::Checked {
                verdict: Verdict::Typo,
                misconception: None
            }
        );
    }
    let per_check = start.elapsed() / runs;
    eprintln!(
        "lexicon {}: {} words, {} bytes; one typo check {per_check:?}",
        r.version, r.words, r.bytes
    );
    assert!(
        per_check.as_micros() < 5_000,
        "a check takes {per_check:?}, budget 5 ms"
    );
}
