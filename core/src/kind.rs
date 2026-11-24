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
    ChoiceBody, DictationBody, EnglishCheck, FindErrorBody, GapFillBody, ItemBody, LadderLevel,
    MatchingBody, OwnSentenceBody, SentenceAssemblyBody, ShadowingBody, Slot, TransformationBody,
    TranslationBody, TrueFalseNotGivenBody, WritingBody, each_body,
};
use crate::lexicon::Lexicon;

/// What a kind is and how it is checked.
pub trait ItemKind {
    /// Step of the ladder: recognition, guided, free, pronunciation.
    fn ladder(&self) -> LadderLevel;
    /// Whether a correct answer counts as production for mastery
    /// (`docs/03-layer2-learner-model.md`) and the KC live condition.
    fn is_production(&self) -> bool;
    /// Chance of a correct answer by guessing on `slot`: `1/n` for a choice
    /// among `n`, 0 for typed input (`docs/03-layer2-learner-model.md`, g).
    fn guessing(&self, slot: u16) -> f64;
    /// Whether the learner types the answer rather than picking it; the
    /// rapid-guess rule adds a 2-second bound for typed input.
    fn typed_input(&self) -> bool;
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

/// `1/n` among the slot's answer and its options.
fn choice_guessing(slots: &[Slot], slot: u16) -> f64 {
    slots.get(usize::from(slot)).map_or(0.0, |s| {
        let n = u32::try_from(s.options.len() + 1).unwrap_or(u32::MAX);
        1.0 / f64::from(n)
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

impl ItemKind for ChoiceBody {
    fn ladder(&self) -> LadderLevel {
        LadderLevel::Recognition
    }
    fn is_production(&self) -> bool {
        false
    }
    fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
        choice_quality(&self.slots, ctx)
    }
    fn guessing(&self, slot: u16) -> f64 {
        choice_guessing(&self.slots, slot)
    }
    fn typed_input(&self) -> bool {
        false
    }
    fn check(&self, slot: u16, response: &Response, _: &CheckCtx<'_>) -> CheckOutcome {
        choice_check(&self.slots, slot, response)
    }
}

impl ItemKind for FindErrorBody {
    fn ladder(&self) -> LadderLevel {
        LadderLevel::Recognition
    }
    fn is_production(&self) -> bool {
        false
    }
    fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
        choice_quality(&self.slots, ctx)
    }
    fn guessing(&self, slot: u16) -> f64 {
        choice_guessing(&self.slots, slot)
    }
    fn typed_input(&self) -> bool {
        false
    }
    fn check(&self, slot: u16, response: &Response, _: &CheckCtx<'_>) -> CheckOutcome {
        choice_check(&self.slots, slot, response)
    }
}

/// A kind whose rule, check and guessing rate are defined by the stage that
/// introduces it; until then it cannot be live, and its guessing rate is 0.
macro_rules! not_yet_checked {
    ($body:ident, $tag:literal, $ladder:ident, $production:literal, $typed:literal) => {
        impl ItemKind for $body {
            fn ladder(&self) -> LadderLevel {
                LadderLevel::$ladder
            }
            fn is_production(&self) -> bool {
                $production
            }
            fn guessing(&self, _: u16) -> f64 {
                0.0
            }
            fn typed_input(&self) -> bool {
                $typed
            }
            fn quality(&self, _: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
                no_rule($tag)
            }
            fn check(&self, _: u16, _: &Response, _: &CheckCtx<'_>) -> CheckOutcome {
                CheckOutcome::NotChecked
            }
        }
    };
}

not_yet_checked!(MatchingBody, "matching", Recognition, false, false);
not_yet_checked!(
    TrueFalseNotGivenBody,
    "true_false_not_given",
    Recognition,
    false,
    false
);
not_yet_checked!(OwnSentenceBody, "own_sentence", Free, true, true);
not_yet_checked!(WritingBody, "writing", Free, true, true);
not_yet_checked!(ShadowingBody, "shadowing", Pronunciation, false, false);

/// An English typed-input kind. Dictation and sentence assembly are not
/// production: the learner transcribes or orders given words rather than
/// producing the form.
macro_rules! english_input {
    ($body:ident, $production:literal) => {
        impl ItemKind for $body {
            fn ladder(&self) -> LadderLevel {
                LadderLevel::Guided
            }
            fn is_production(&self) -> bool {
                $production
            }
            fn guessing(&self, _: u16) -> f64 {
                0.0
            }
            fn typed_input(&self) -> bool {
                true
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

english_input!(GapFillBody, true);
english_input!(TransformationBody, true);
english_input!(TranslationBody, true);
english_input!(SentenceAssemblyBody, false);
english_input!(DictationBody, false);

impl ItemKind for ItemBody {
    fn ladder(&self) -> LadderLevel {
        each_body!(self, b => b.ladder())
    }
    fn is_production(&self) -> bool {
        each_body!(self, b => b.is_production())
    }
    fn guessing(&self, slot: u16) -> f64 {
        each_body!(self, b => b.guessing(slot))
    }
    fn typed_input(&self) -> bool {
        each_body!(self, b => b.typed_input())
    }
    fn quality(&self, ctx: &QualityCtx<'_>) -> Result<(), Vec<QualityIssue>> {
        each_body!(self, b => b.quality(ctx))
    }
    fn check(&self, slot: u16, response: &Response, ctx: &CheckCtx<'_>) -> CheckOutcome {
        each_body!(self, b => b.check(slot, response, ctx))
    }
}
