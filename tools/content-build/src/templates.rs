//! Answer templates: parse, normalize with each item's flags, compile into
//! automata over one token table, and check intersections exactly in
//! normalized space (`docs/02-layer1-content.md`, «Проверки при сборке
//! контента»).

use std::collections::BTreeMap;
use std::path::Path;

use refl_core::answer::dfa::{Dfa, TokenId, TokenTable, intersection_witness};
use refl_core::answer::normalize::{NormalizeOptions, normalize};
use refl_core::answer::template::{NormPattern, Pattern};
use refl_core::bundle::{ErrorAutomaton, SlotAutomata};
use refl_core::ids::MisconceptionId;
use refl_core::item::{AnswerPattern, Item, Slot};

use crate::load::Sourced;
use crate::report::{Code, Diagnostic, Severity};

/// Automata of every item whose templates are all valid, by item index.
#[derive(Debug)]
pub struct Compiled {
    pub tokens: TokenTable,
    pub items: BTreeMap<usize, Vec<SlotAutomata>>,
}

struct NormSlot {
    accepted: Vec<NormPattern>,
    wrong: Vec<(MisconceptionId, NormPattern)>,
}

fn diag(severity: Severity, code: Code, file: &Path, item: &Item, message: String) -> Diagnostic {
    Diagnostic {
        severity,
        code,
        file: file.to_path_buf(),
        subject: Some(format!("item `{}`", item.id)),
        message,
    }
}

fn parse_one(
    pattern: &AnswerPattern,
    at: &str,
    opts: NormalizeOptions,
    report: &mut dyn FnMut(Code, String),
) -> Option<NormPattern> {
    let src = &pattern.0;
    let parsed = Pattern::parse(src)
        .map_err(|e| report(Code::TemplateSyntax, format!("{at} `{src}`: {e}")))
        .ok()?;
    parsed
        .normalize(opts)
        .map_err(|e| report(Code::TemplateEmpty, format!("{at} `{src}`: {e}")))
        .ok()
}

/// Parses and normalizes one item's slots; `None` if any template is invalid.
fn normalize_item(it: &Sourced<Item>, diags: &mut Vec<Diagnostic>) -> Option<Vec<NormSlot>> {
    let item = &it.value;
    let opts = NormalizeOptions::of(&item.body);
    let mut report =
        |code, message| diags.push(diag(Severity::Error, code, &it.file, item, message));
    let mut ok = true;
    let mut slots = Vec::new();
    for (j, slot) in item.body.slots().iter().enumerate() {
        if slot.accepted.is_empty() {
            report(
                Code::NoAccepted,
                format!("slots[{j}] has no accepted patterns"),
            );
            ok = false;
        }
        let mut norm = NormSlot {
            accepted: Vec::new(),
            wrong: Vec::new(),
        };
        for (k, p) in slot.accepted.iter().enumerate() {
            let at = format!("slots[{j}].accepted[{k}]");
            match parse_one(p, &at, opts, &mut report) {
                Some(n) => norm.accepted.push(n),
                None => ok = false,
            }
        }
        for (k, w) in slot.expected_wrong.iter().enumerate() {
            let at = format!("slots[{j}].expected_wrong[{k}]");
            match parse_one(&w.pattern, &at, opts, &mut report) {
                Some(n) => norm.wrong.push((w.misconception.clone(), n)),
                None => ok = false,
            }
        }
        slots.push(norm);
    }
    ok.then_some(slots)
}

/// Compiles every item's templates and runs the intersection checks.
pub fn compile(items: &[Sourced<Item>], diags: &mut Vec<Diagnostic>) -> Compiled {
    let normalized: Vec<Option<Vec<NormSlot>>> =
        items.iter().map(|it| normalize_item(it, diags)).collect();
    let tokens = TokenTable::new(
        normalized
            .iter()
            .flatten()
            .flatten()
            .flat_map(|s| s.accepted.iter().chain(s.wrong.iter().map(|(_, p)| p)))
            .flat_map(NormPattern::tokens),
    );
    let mut compiled = BTreeMap::new();
    for (i, (it, norm)) in items.iter().zip(&normalized).enumerate() {
        let Some(norm) = norm else { continue };
        let mut cx = SlotCheck {
            item: &it.value,
            file: &it.file,
            opts: NormalizeOptions::of(&it.value.body),
            tokens: &tokens,
            diags,
        };
        let automata = it
            .value
            .body
            .slots()
            .iter()
            .zip(norm)
            .enumerate()
            .map(|(j, (slot, ns))| cx.slot(j, slot, ns))
            .collect();
        compiled.insert(i, automata);
    }
    Compiled {
        tokens,
        items: compiled,
    }
}

/// Checks of one item's slots against its compiled automata.
struct SlotCheck<'a> {
    item: &'a Item,
    file: &'a Path,
    opts: NormalizeOptions,
    tokens: &'a TokenTable,
    diags: &'a mut Vec<Diagnostic>,
}

impl SlotCheck<'_> {
    fn report(&mut self, severity: Severity, code: Code, message: String) {
        self.diags
            .push(diag(severity, code, self.file, self.item, message));
    }

    fn dfa(&self, patterns: &[NormPattern]) -> Dfa {
        Dfa::compile(patterns, self.tokens)
            .expect("the token table is built from every normalized pattern")
    }

    fn witness(&self, tokens: &[TokenId]) -> String {
        let words: Vec<&str> = tokens
            .iter()
            .map(|&t| self.tokens.text(t).unwrap_or("<other>"))
            .collect();
        format!("\"{}\"", words.join(" "))
    }

    fn slot(&mut self, j: usize, slot: &Slot, ns: &NormSlot) -> SlotAutomata {
        let accepted = self.dfa(&ns.accepted);
        let answer = self.tokens.ids(&normalize(&slot.answer, self.opts));
        if !accepted.accepts(&answer) {
            self.report(
                Severity::Error,
                Code::AnswerNotAccepted,
                format!(
                    "slots[{j}].answer \"{}\" is not accepted by `accepted`",
                    slot.answer
                ),
            );
        }
        let wrong: Vec<ErrorAutomaton> = ns
            .wrong
            .iter()
            .map(|(m, p)| ErrorAutomaton {
                misconception: m.clone(),
                dfa: self.dfa(std::slice::from_ref(p)),
            })
            .collect();
        for (k, w) in wrong.iter().enumerate() {
            if let Some(witness) = intersection_witness(&accepted, &w.dfa) {
                let msg = format!(
                    "slots[{j}]: `accepted` and expected_wrong[{k}] (`{}`) both match {}",
                    w.misconception,
                    self.witness(&witness)
                );
                self.report(Severity::Error, Code::AcceptedMatchesError, msg);
            }
        }
        for (k, a) in wrong.iter().enumerate() {
            for (l, b) in wrong.iter().enumerate().skip(k + 1) {
                if let Some(witness) = intersection_witness(&a.dfa, &b.dfa) {
                    let msg = format!(
                        "slots[{j}]: expected_wrong[{k}] (`{}`) and expected_wrong[{l}] (`{}`) \
                         both match {}; the first one wins",
                        a.misconception,
                        b.misconception,
                        self.witness(&witness)
                    );
                    self.report(Severity::Warning, Code::ErrorPatternsOverlap, msg);
                }
            }
        }
        SlotAutomata {
            accepted,
            expected_wrong: wrong,
        }
    }
}
