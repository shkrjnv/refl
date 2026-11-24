//! Behaviour of item kinds (`docs/10-subjects.md`, «Вид задания: трейт и
//! данные»): each [`ItemBody`] variant implements [`ItemKind`] on its own
//! body type, and [`ItemBody`] dispatches. A new kind is a new variant, a new
//! implementation and a client component; the model, planner and log do not
//! change.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::answer::dfa::TokenTable;
use crate::answer::normalize::{NormalizeOptions, normalize};
use crate::answer::typo::is_typo;
use crate::bundle::SlotAutomata;
use crate::event::{Response, Verdict};
use crate::ids::MisconceptionId;
use crate::item::{
    ChoiceBody, DictationBody, EnglishCheck, FindErrorBody, GapFillBody, ItemBody, ItemKindTag,
    LadderLevel, MatchingBody, OwnSentenceBody, SentenceAssemblyBody, ShadowingBody, Slot,
    TransformationBody, TranslationBody, TrueFalseNotGivenBody, WritingBody, each_body,
};
use crate::lexicon::Lexicon;
use crate::projection::guessing_rate;

/// What a kind is and how it is checked.
pub trait ItemKind {
    /// The kind's tag. Ladder step, production and typed input are properties
    /// of the kind alone and come from the tag, so a stored projection
    /// (`docs/02-layer1-content.md`, «Идентификаторы и версии») answers them
    /// the same way the item does.
    fn tag(&self) -> ItemKindTag;
    /// Number of answers to pick among on `slot`, the correct one included;
    /// 0 for typed input and for kinds without a pick.
    fn choices(&self, slot: u16) -> u16;
    /// Step of the ladder: recognition, guided, free, pronunciation.
    fn ladder(&self) -> LadderLevel {
        self.tag().ladder()
    }
    /// Whether a correct answer counts as production for mastery
    /// (`docs/03-layer2-learner-model.md`) and the KC live condition.
    fn is_production(&self) -> bool {
        self.tag().is_production()
    }
    /// Chance of a correct answer by guessing on `slot`: `1/n` for a choice
    /// among `n`, 0 for typed input (`docs/03-layer2-learner-model.md`, g).
    fn guessing(&self, slot: u16) -> f64 {
        guessing_rate(self.choices(slot))
    }
    /// Whether the learner types the answer rather than picking it; the
    /// rapid-guess rule adds a 2-second bound for typed input.
    fn typed_input(&self) -> bool {
        self.tag().typed_input()
    }
    /// The quality rule of a live item, checked by content-build.
    ///
    /// # Errors
    /// Every rule the item breaks.
    fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>>;
    /// Deterministic check of one slot's response.
    fn check(&self, slot: u16, response: &Response, ctx: &CheckCtx<'_>) -> CheckOutcome;
}

/// Compiled answers of one item, as content-build produces them.
#[derive(Debug, Clone, Copy)]
pub struct Compiled<'a> {
    /// One per slot, in slot order.
    pub slots: &'a [SlotAutomata],
    /// Token alphabet of the automata.
    pub tokens: &'a TokenTable,
}

#[derive(Debug, Clone, Copy)]
pub struct QualityCtx<'a> {
    /// `None` when the item's answer templates did not compile; the checks
    /// in normalized space are then skipped (the templates are reported).
    pub compiled: Option<Compiled<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckCtx<'a> {
    pub compiled: Compiled<'a>,
    /// The bundle's lexicon, for the typo rule.
    pub lexicon: &'a Lexicon,
}

/// Why a live item breaks its kind's quality rule
/// (`docs/02-layer1-content.md`, «Задание: общая часть»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityIssueKind {
    /// The kind has no rule yet, so it cannot be live.
    NoRule,
    /// Fewer than 2 distractors with a misconception.
    TooFewDistractors,
    /// A distractor is accepted by the slot's `accepted`.
    DistractorAccepted,
    /// Two distractors are equal after normalization.
    DuplicateDistractor,
    /// Fewer than 2 expected-error patterns.
    TooFewErrorPatterns,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityIssue {
    pub kind: QualityIssueKind,
    pub message: String,
}

/// Result of checking one slot (`docs/10-subjects.md`, «Вид задания»).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    Checked {
        verdict: Verdict,
        misconception: Option<MisconceptionId>,
    },
    /// The kind has no automatic check yet.
    NotChecked,
}

impl CheckOutcome {
    fn verdict(verdict: Verdict, misconception: Option<MisconceptionId>) -> Self {
        Self::Checked {
            verdict,
            misconception,
        }
    }

    fn unrecognized() -> Self {
        Self::verdict(Verdict::Unrecognized, None)
    }
}

fn issue(kind: QualityIssueKind, message: String) -> QualityIssue {
    QualityIssue { kind, message }
}

fn finish(issues: Vec<QualityIssue>) -> Result<(), Vec<QualityIssue>> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Choice rule, per slot: ≥ 2 diagnosed distractors; in normalized space with
/// default settings, no distractor is accepted and none repeats.
fn choice_quality(slots: &[Slot], ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
    let mut issues = Vec::new();
    for (j, slot) in slots.iter().enumerate() {
        let n = slot
            .options
            .iter()
            .filter(|o| o.misconception.is_some())
            .count();
        if n < 2 {
            let message = format!("slots[{j}] has {n} distractor(s) with a misconception, needs 2");
            issues.push(issue(QualityIssueKind::TooFewDistractors, message));
        }
        let Some(compiled) = ctx.compiled else {
            continue;
        };
        let accepted = &compiled.slots[j].accepted;
        // Keyed by normalized text, not ids: unknown tokens all map to OTHER.
        let mut seen: BTreeMap<Vec<String>, usize> = BTreeMap::new();
        for (k, opt) in slot.options.iter().enumerate() {
            let norm = normalize(&opt.text, NormalizeOptions::default());
            if accepted.accepts(&compiled.tokens.ids(&norm)) {
                let message = format!(
                    "slots[{j}].options[{k}] \"{}\" is accepted by `accepted`",
                    opt.text
                );
                issues.push(issue(QualityIssueKind::DistractorAccepted, message));
            }
            if let Some(&first) = seen.get(&norm) {
                let message = format!(
                    "slots[{j}].options[{k}] \"{}\" repeats options[{first}] after normalization",
                    opt.text
                );
                issues.push(issue(QualityIssueKind::DuplicateDistractor, message));
            } else {
                seen.insert(norm, k);
            }
        }
    }
    finish(issues)
}

/// Typed-input rule, per slot: ≥ 2 expected-error patterns.
fn input_quality(slots: &[Slot]) -> Result<(), Vec<QualityIssue>> {
    let issues = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.expected_wrong.len() < 2)
        .map(|(j, slot)| {
            let n = slot.expected_wrong.len();
            let message = format!("slots[{j}] has {n} expected_wrong pattern(s), needs 2");
            issue(QualityIssueKind::TooFewErrorPatterns, message)
        })
        .collect();
    finish(issues)
}

/// The slot's answer and its options.
fn slot_choices(slots: &[Slot], slot: u16) -> u16 {
    slots.get(usize::from(slot)).map_or(0, |s| {
        u16::try_from(s.options.len() + 1).unwrap_or(u16::MAX)
    })
}

fn no_rule(tag: &str) -> Result<(), Vec<QualityIssue>> {
    let message = format!("type `{tag}` has no quality rule yet and cannot be live");
    Err(vec![issue(QualityIssueKind::NoRule, message)])
}

/// Compares the canonical option index: 0 is the slot's answer, `1..=n` are
/// `options` in authored order (`docs/02-layer1-content.md`, «Задание: общая
/// часть»). Any other response is unrecognized.
fn choice_check(slots: &[Slot], slot: u16, response: &Response) -> CheckOutcome {
    let (Some(s), Response::Option(i)) = (slots.get(usize::from(slot)), response) else {
        return CheckOutcome::unrecognized();
    };
    match usize::from(*i).checked_sub(1) {
        None => CheckOutcome::verdict(Verdict::Correct, None),
        Some(k) => match s.options.get(k) {
            Some(opt) => CheckOutcome::verdict(Verdict::Wrong, opt.misconception.clone()),
            None => CheckOutcome::unrecognized(),
        },
    }
}

/// The typed-input pipeline of `docs/02-layer1-content.md`, «Проверка ответов
/// на ввод»: normalize with the item's settings, accepted answers, expected
/// errors (first match wins), alignment for typos, otherwise unrecognized.
/// Other step-4 differences (form, missing word, order) stay unrecognized
/// until docs/02 open decision 10.
fn input_check(
    check: EnglishCheck,
    slot: u16,
    response: &Response,
    ctx: &CheckCtx<'_>,
) -> CheckOutcome {
    let compiled = ctx.compiled;
    let (Some(automata), Response::Text(text)) = (compiled.slots.get(usize::from(slot)), response)
    else {
        return CheckOutcome::unrecognized();
    };
    let words = normalize(text, NormalizeOptions::from(check));
    let tokens = compiled.tokens.ids(&words);
    if automata.accepted.accepts(&tokens) {
        return CheckOutcome::verdict(Verdict::Correct, None);
    }
    match automata
        .expected_wrong
        .iter()
        .find(|w| w.dfa.accepts(&tokens))
    {
        Some(w) => CheckOutcome::verdict(Verdict::Wrong, Some(w.misconception.clone())),
        None if is_typo(&words, &automata.accepted, compiled.tokens, ctx.lexicon) => {
            CheckOutcome::verdict(Verdict::Typo, None)
        }
        None => CheckOutcome::unrecognized(),
    }
}

/// A kind that picks among options.
macro_rules! choice_kind {
    ($body:ident, $tag:ident) => {
        impl ItemKind for $body {
            fn tag(&self) -> ItemKindTag {
                ItemKindTag::$tag
            }
            fn choices(&self, slot: u16) -> u16 {
                slot_choices(&self.slots, slot)
            }
            fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
                choice_quality(&self.slots, ctx)
            }
            fn check(&self, slot: u16, response: &Response, _: &CheckCtx<'_>) -> CheckOutcome {
                choice_check(&self.slots, slot, response)
            }
        }
    };
}

choice_kind!(ChoiceBody, Choice);
choice_kind!(FindErrorBody, FindError);

/// A kind whose rule, check and guessing rate are defined by the stage that
/// introduces it; until then it cannot be live, and its guessing rate is 0.
macro_rules! not_yet_checked {
    ($body:ident, $tag:ident, $name:literal) => {
        impl ItemKind for $body {
            fn tag(&self) -> ItemKindTag {
                ItemKindTag::$tag
            }
            fn choices(&self, _: u16) -> u16 {
                0
            }
            fn quality(&self, _: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
                no_rule($name)
            }
            fn check(&self, _: u16, _: &Response, _: &CheckCtx<'_>) -> CheckOutcome {
                CheckOutcome::NotChecked
            }
        }
    };
}

not_yet_checked!(MatchingBody, Matching, "matching");
not_yet_checked!(
    TrueFalseNotGivenBody,
    TrueFalseNotGiven,
    "true_false_not_given"
);
not_yet_checked!(OwnSentenceBody, OwnSentence, "own_sentence");
not_yet_checked!(WritingBody, Writing, "writing");
not_yet_checked!(ShadowingBody, Shadowing, "shadowing");

/// An English typed-input kind.
macro_rules! english_input {
    ($body:ident, $tag:ident) => {
        impl ItemKind for $body {
            fn tag(&self) -> ItemKindTag {
                ItemKindTag::$tag
            }
            fn choices(&self, _: u16) -> u16 {
                0
            }
            fn quality(&self, _: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
                input_quality(&self.slots)
            }
            fn check(&self, slot: u16, response: &Response, ctx: &CheckCtx<'_>) -> CheckOutcome {
                input_check(self.check, slot, response, ctx)
            }
        }
    };
}

english_input!(GapFillBody, GapFill);
english_input!(TransformationBody, Transformation);
english_input!(TranslationBody, Translation);
english_input!(SentenceAssemblyBody, SentenceAssembly);
english_input!(DictationBody, Dictation);

impl ItemKind for ItemBody {
    fn tag(&self) -> ItemKindTag {
        ItemBody::tag(self)
    }
    fn choices(&self, slot: u16) -> u16 {
        each_body!(self, b => b.choices(slot))
    }
    fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
        each_body!(self, b => b.quality(ctx))
    }
    fn check(&self, slot: u16, response: &Response, ctx: &CheckCtx<'_>) -> CheckOutcome {
        each_body!(self, b => b.check(slot, response, ctx))
    }
}
