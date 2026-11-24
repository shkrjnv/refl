//! Structural checks: unique ids, references, per-item rules for live items
//! and referenced audio files (`docs/02-layer1-content.md`, «Проверки при
//! сборке контента», «Задание: общая часть»).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use refl_core::content::{
    EdgeKind, ExplanationDepth, ExplanationTarget, LangReviews, Review, ReviewStatus,
};
use refl_core::ids::{InstructionId, KcId, MisconceptionId};
use refl_core::item::{ItemStatus, KcRole};
use refl_core::kind::{self, ItemKind, QualityCtx, QualityIssueKind};
use refl_core::lesson::LessonStep;
use refl_core::value::{Lang, Localized};

use crate::load::{Content, Sourced};
use crate::report::{Code, Diagnostic, Severity};
use crate::templates::Compiled;

/// The name an enum value has in content files, e.g. `gap_fill`.
fn wire_name(v: impl serde::Serialize) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}

pub(crate) fn diag(code: Code, file: &Path, subject: String, message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code,
        file: file.to_path_buf(),
        subject: Some(subject),
        message,
    }
}

/// Ids of every referenced kind of entity.
#[derive(Debug, Default)]
pub struct Ids {
    pub kcs: BTreeSet<KcId>,
    pub misconceptions: BTreeSet<MisconceptionId>,
    pub instructions: BTreeSet<InstructionId>,
}

fn unique<'a, T, K: Ord + Clone + std::fmt::Display + 'a>(
    kind: &str,
    entries: &'a [Sourced<T>],
    key: impl Fn(&'a T) -> &'a K,
    diags: &mut Vec<Diagnostic>,
) -> BTreeSet<K> {
    let mut seen: BTreeMap<&K, &Path> = BTreeMap::new();
    for e in entries {
        let id = key(&e.value);
        if let Some(first) = seen.get(id) {
            diags.push(diag(
                Code::DuplicateId,
                &e.file,
                format!("{kind} `{id}`"),
                format!("id already defined in {}", first.display()),
            ));
        } else {
            seen.insert(id, &e.file);
        }
    }
    seen.into_keys().cloned().collect()
}

/// Every id must be unique within its kind.
pub fn duplicates(c: &Content, diags: &mut Vec<Diagnostic>) -> Ids {
    unique("subject", &c.subjects, |s| &s.id, diags);
    let ids = Ids {
        kcs: unique("KC", &c.kcs, |k| &k.id, diags),
        misconceptions: unique("misconception", &c.misconceptions, |m| &m.id, diags),
        instructions: unique("instruction", &c.instructions, |i| &i.id, diags),
    };
    unique("explanation", &c.explanations, |e| &e.id, diags);
    unique("item", &c.items, |i| &i.id, diags);
    unique("lesson", &c.lessons, |l| &l.id, diags);
    ids
}

/// Every reference to a KC, misconception or instruction resolves.
pub fn references(c: &Content, ids: &Ids, diags: &mut Vec<Diagnostic>) {
    let mut unknown = |file: &Path, subject: &str, what: &str, id: &dyn std::fmt::Display| {
        diags.push(diag(
            Code::UnknownReference,
            file,
            subject.to_owned(),
            format!("unknown {what} `{id}`"),
        ));
    };
    for e in &c.edges {
        let edge = &e.value;
        let subject = format!(
            "edge `{}` {} `{}`",
            edge.from,
            wire_name(edge.kind),
            edge.to
        );
        for kc in [&edge.from, &edge.to] {
            if !ids.kcs.contains(kc) {
                unknown(&e.file, &subject, "KC", kc);
            }
        }
    }
    for m in &c.misconceptions {
        let subject = format!("misconception `{}`", m.value.id);
        for kc in m.value.affects.iter().chain([&m.value.remedy]) {
            if !ids.kcs.contains(kc) {
                unknown(&m.file, &subject, "KC", kc);
            }
        }
    }
    for e in &c.explanations {
        let subject = format!("explanation `{}`", e.value.id);
        match &e.value.target {
            ExplanationTarget::Kc(kc) if !ids.kcs.contains(kc) => {
                unknown(&e.file, &subject, "KC", kc);
            }
            ExplanationTarget::Misconception(m) if !ids.misconceptions.contains(m) => {
                unknown(&e.file, &subject, "misconception", m);
            }
            _ => {}
        }
    }
    for it in &c.items {
        let subject = format!("item `{}`", it.value.id);
        if !ids.instructions.contains(&it.value.instruction) {
            unknown(&it.file, &subject, "instruction", &it.value.instruction);
        }
        for slot in it.value.body.slots() {
            let confused = slot
                .options
                .iter()
                .filter_map(|o| o.confuses_with.as_ref())
                .chain(
                    slot.expected_wrong
                        .iter()
                        .filter_map(|w| w.confuses_with.as_ref()),
                );
            for kc in slot.kcs.iter().map(|sk| &sk.kc).chain(confused) {
                if !ids.kcs.contains(kc) {
                    unknown(&it.file, &subject, "KC", kc);
                }
            }
            let mis = slot
                .options
                .iter()
                .filter_map(|o| o.misconception.as_ref())
                .chain(slot.expected_wrong.iter().map(|w| &w.misconception));
            for m in mis {
                if !ids.misconceptions.contains(m) {
                    unknown(&it.file, &subject, "misconception", m);
                }
            }
        }
    }
    for l in &c.lessons {
        let subject = format!("lesson `{}`", l.value.id);
        let mut kcs: Vec<&KcId> = l.value.kcs.iter().collect();
        for step in &l.value.steps {
            match step {
                LessonStep::Pretest { query }
                | LessonStep::Practice { query }
                | LessonStep::Production { query }
                | LessonStep::Posttest { query } => kcs.extend(&query.kcs),
                LessonStep::Explanation { kc, .. } => kcs.push(kc),
                LessonStep::WorkedExamples { .. } | LessonStep::Summary => {}
            }
        }
        for kc in kcs {
            if !ids.kcs.contains(kc) {
                unknown(&l.file, &subject, "KC", kc);
            }
        }
    }
}

/// The `contrasts` edges with no live item where one end is a primary KC of
/// a slot and an option or error pattern is marked `confuses_with` the other
/// end, so that confusing the pair cannot be measured
/// (`docs/03-layer2-learner-model.md`, «Расширенная диагностика»). Edges are
/// symmetric in meaning, so either direction counts. Not an error: a blocker
/// of both KCs (`status`).
pub fn uncrossed_contrasts(c: &Content) -> Vec<(&KcId, &KcId)> {
    let mut crossed: BTreeSet<(&KcId, &KcId)> = BTreeSet::new();
    let live_items = c
        .items
        .iter()
        .map(|i| &i.value)
        .filter(|i| i.status == ItemStatus::Live);
    for slot in live_items.flat_map(|i| i.body.slots()) {
        let confused = slot
            .options
            .iter()
            .filter_map(|o| o.confuses_with.as_ref())
            .chain(
                slot.expected_wrong
                    .iter()
                    .filter_map(|w| w.confuses_with.as_ref()),
            );
        for other in confused {
            for k in slot.kcs.iter().filter(|k| k.role == KcRole::Primary) {
                crossed.insert((&k.kc, other));
            }
        }
    }
    c.edges
        .iter()
        .map(|e| &e.value)
        .filter(|e| e.kind == EdgeKind::Contrasts)
        .filter(|e| !crossed.contains(&(&e.from, &e.to)) && !crossed.contains(&(&e.to, &e.from)))
        .map(|e| (&e.from, &e.to))
        .collect()
}

/// Live items: every slot has a primary KC, and the item keeps its kind's
/// quality rule ([`ItemKind::quality`]); the rule's checks in normalized
/// space use the compiled templates.
pub fn live_items(c: &Content, compiled: &Compiled, diags: &mut Vec<Diagnostic>) {
    for (i, it) in c.items.iter().enumerate() {
        let item = &it.value;
        if item.status != ItemStatus::Live {
            continue;
        }
        let subject = format!("item `{}`", item.id);
        let mut push = |code, message: String| {
            diags.push(diag(code, &it.file, subject.clone(), message));
        };
        let slots = item.body.slots();
        if slots.is_empty() {
            push(Code::NoPrimaryKc, "live item has no slots".to_owned());
        }
        for (j, slot) in slots.iter().enumerate() {
            if !slot.kcs.iter().any(|k| k.role == KcRole::Primary) {
                push(Code::NoPrimaryKc, format!("slots[{j}] has no primary KC"));
            }
        }
        let ctx = QualityCtx {
            compiled: compiled.items.get(&i).map(|slots| kind::Compiled {
                slots,
                tokens: &compiled.tokens,
            }),
        };
        for issue in item.body.quality(&ctx).err().unwrap_or_default() {
            let code = match issue.kind {
                QualityIssueKind::NoRule => Code::NoQualityRule,
                QualityIssueKind::TooFewDistractors => Code::TooFewDistractors,
                QualityIssueKind::DistractorAccepted => Code::DistractorAccepted,
                QualityIssueKind::DuplicateDistractor => Code::DuplicateDistractor,
                QualityIssueKind::TooFewErrorPatterns => Code::TooFewErrorPatterns,
            };
            push(code, issue.message);
        }
    }
}

/// Referenced audio files exist under `audio_root`.
pub fn audio(c: &Content, audio_root: &Path, diags: &mut Vec<Diagnostic>) {
    let mut check = |file: &Path, subject: String, path: &str| {
        if !audio_root.join(path).is_file() {
            diags.push(diag(
                Code::MissingAudio,
                file,
                subject,
                format!(
                    "audio file `{path}` not found under {}",
                    audio_root.display()
                ),
            ));
        }
    };
    for it in &c.items {
        if let Some(a) = &it.value.body.stimulus().audio {
            check(&it.file, format!("item `{}`", it.value.id), &a.0);
        }
    }
    for e in &c.explanations {
        for ex in &e.value.examples {
            if let Some(a) = &ex.audio {
                check(&e.file, format!("explanation `{}`", e.value.id), &a.0);
            }
        }
    }
}

fn missing_ru_en(text: &Localized) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if text.ru.is_none() {
        missing.push("ru");
    }
    if text.en.is_none() {
        missing.push("en");
    }
    missing
}

/// A misconception a live item can diagnose is live; its micro-explanation is
/// found by key (target, depth short) and must exist in ru and en.
pub fn misconception_feedback(c: &Content, diags: &mut Vec<Diagnostic>) {
    let mut diagnosed_by: BTreeMap<&MisconceptionId, &str> = BTreeMap::new();
    for it in c
        .items
        .iter()
        .filter(|i| i.value.status == ItemStatus::Live)
    {
        for slot in it.value.body.slots() {
            let mis = slot
                .options
                .iter()
                .filter_map(|o| o.misconception.as_ref())
                .chain(slot.expected_wrong.iter().map(|w| &w.misconception));
            for m in mis {
                diagnosed_by.entry(m).or_insert(it.value.id.as_str());
            }
        }
    }
    for m in &c.misconceptions {
        let Some(item) = diagnosed_by.get(&m.value.id) else {
            continue;
        };
        for (lang, name) in [(Lang::Ru, "ru"), (Lang::En, "en")] {
            let found = c.explanations.iter().any(|e| {
                let e = &e.value;
                e.target == ExplanationTarget::Misconception(m.value.id.clone())
                    && e.depth == ExplanationDepth::Short
                    && e.lang == lang
            });
            if !found {
                diags.push(diag(
                    Code::MissingFeedback,
                    &m.file,
                    format!("misconception `{}`", m.value.id),
                    format!(
                        "diagnosed by live item `{item}` but has no short explanation in {name}"
                    ),
                ));
            }
        }
    }
}

/// Instructions used by live items have text in ru and en.
pub fn instruction_text(c: &Content, diags: &mut Vec<Diagnostic>) {
    let mut used_by: BTreeMap<&InstructionId, &str> = BTreeMap::new();
    for it in c
        .items
        .iter()
        .filter(|i| i.value.status == ItemStatus::Live)
    {
        used_by
            .entry(&it.value.instruction)
            .or_insert(it.value.id.as_str());
    }
    for ins in &c.instructions {
        let Some(item) = used_by.get(&ins.value.id) else {
            continue;
        };
        let missing = missing_ru_en(&ins.value.text);
        if !missing.is_empty() {
            diags.push(diag(
                Code::InstructionText,
                &ins.file,
                format!("instruction `{}`", ins.value.id),
                format!(
                    "used by live item `{item}` but has no text in {}",
                    missing.join(", ")
                ),
            ));
        }
    }
}

/// A review is bound to the text it covers: a `reviewed` record must carry
/// the hash of exactly that text, else the text changed after the review
/// (docs/02, «Вычитка и показ»). One error per record and language.
pub fn reviewed_texts(c: &Content, diags: &mut Vec<Diagnostic>) {
    for cov in crate::review::covered(c) {
        let wrong = cov
            .reviews
            .iter()
            // A review without a name is already an error of its own.
            .filter(|r| r.status == ReviewStatus::Reviewed && r.by.is_some())
            .find(|r| r.hash.as_deref() != Some(cov.hash.as_str()));
        let Some(r) = wrong else { continue };
        let lang = format!("{:?}", cov.lang).to_lowercase();
        let message = match &r.hash {
            None => format!(
                "language {lang} is `reviewed` without a `hash`; the text now hashes to {}",
                cov.hash
            ),
            Some(old) => format!(
                "language {lang}: the text changed since it was reviewed (recorded hash {old}, now {})",
                cov.hash
            ),
        };
        diags.push(diag(
            Code::ReviewedTextChanged,
            &cov.file,
            format!("review of `{}`", cov.key),
            message,
        ));
    }
}

/// `reviewed` comes with the name of the person who reviewed
/// (docs/02, «Вычитка и показ»).
pub fn reviewers(c: &Content, diags: &mut Vec<Diagnostic>) {
    let nameless = |r: &Review| r.status == ReviewStatus::Reviewed && r.by.is_none();
    for e in &c.explanations {
        if nameless(&e.value.review) {
            diags.push(diag(
                Code::ReviewedWithoutReviewer,
                &e.file,
                format!("explanation `{}`", e.value.id),
                "review is `reviewed` but `by` is missing".to_owned(),
            ));
        }
    }
    let mut per_language = |file: &Path, what: String, reviews: &LangReviews| {
        for lang in [Lang::Uz, Lang::Ru, Lang::En] {
            if reviews.get(lang).is_some_and(nameless) {
                diags.push(diag(
                    Code::ReviewedWithoutReviewer,
                    file,
                    what.clone(),
                    format!("review of {lang:?} is `reviewed` but `by` is missing"),
                ));
            }
        }
    };
    for ins in &c.instructions {
        per_language(
            &ins.file,
            format!("instruction `{}`", ins.value.id),
            &ins.value.review,
        );
    }
    for k in &c.kcs {
        per_language(&k.file, format!("KC `{}`", k.value.id), &k.value.review);
    }
    for m in &c.misconceptions {
        per_language(
            &m.file,
            format!("misconception `{}`", m.value.id),
            &m.value.review,
        );
    }
    for i in &c.items {
        per_language(&i.file, format!("item `{}`", i.value.id), &i.value.review);
    }
}
