//! Subject checks (`docs/10-subjects.md`, «Предмет», «Шкала уровней»;
//! `docs/02-layer1-content.md`, «Проверки при сборке контента»): every KC
//! names an existing subject, a level key of its scale and one of its
//! domains; level keys and domains are unique within a subject. Ids of KCs,
//! misconceptions and item families start with their subject's slug, and
//! `requires` never crosses subjects (`supports` may). The scale is not
//! empty and its step is positive; the spelling KC and slot KCs belong to
//! the subject.

use std::collections::{BTreeMap, BTreeSet};

use refl_core::content::EdgeKind;
use refl_core::ids::{KcId, SubjectId};
use refl_core::subject::Subject;

use crate::check::diag;
use crate::load::Content;
use crate::report::{Code, Diagnostic};

/// Subjects by id; the first definition wins when an id is duplicated.
pub fn by_id(c: &Content) -> BTreeMap<&SubjectId, &Subject> {
    let mut out = BTreeMap::new();
    for s in &c.subjects {
        out.entry(&s.value.id).or_insert(&s.value);
    }
    out
}

pub fn check(c: &Content, diags: &mut Vec<Diagnostic>) {
    for s in &c.subjects {
        let subject = format!("subject `{}`", s.value.id);
        let scale = &s.value.scale;
        if scale.levels.is_empty() || scale.step_logit.get() <= 0.0 {
            let message = format!(
                "the scale needs at least one level and step_logit > 0, has {} level(s) and {}",
                scale.levels.len(),
                scale.step_logit.get()
            );
            diags.push(diag(Code::InvalidScale, &s.file, subject.clone(), message));
        }
        let mut keys = BTreeSet::new();
        for level in &s.value.scale.levels {
            if !keys.insert(&level.key) {
                let message = format!("level key `{}` appears twice in the scale", level.key);
                diags.push(diag(Code::DuplicateId, &s.file, subject.clone(), message));
            }
        }
        let mut domains = BTreeSet::new();
        for d in &s.value.domains {
            if !domains.insert(d) {
                let message = format!("domain `{d}` appears twice");
                diags.push(diag(Code::DuplicateId, &s.file, subject.clone(), message));
            }
        }
    }
    let subjects = by_id(c);
    for k in &c.kcs {
        let kc = &k.value;
        let subject = format!("KC `{}`", kc.id);
        let Some(s) = subjects.get(&kc.subject) else {
            let message = format!("unknown subject `{}`", kc.subject);
            diags.push(diag(Code::UnknownReference, &k.file, subject, message));
            continue;
        };
        if s.scale.rank(&kc.level).is_none() {
            let message = format!("level `{}` is not in the scale of `{}`", kc.level, s.id);
            diags.push(diag(Code::UnknownLevel, &k.file, subject.clone(), message));
        }
        if !s.domains.contains(&kc.domain) {
            let message = format!("domain `{}` is not a domain of `{}`", kc.domain, s.id);
            diags.push(diag(Code::UnknownDomain, &k.file, subject, message));
        }
    }
}

/// The subject's spelling KC and every KC of an item's slots belong to the
/// subject: a typo updates the subject's own spelling KC, and an item is
/// assessed within one subject. Unknown subjects and KCs are reported
/// elsewhere.
pub fn kc_subjects(c: &Content, diags: &mut Vec<Diagnostic>) {
    let kc_subject: BTreeMap<&KcId, &SubjectId> = c
        .kcs
        .iter()
        .map(|k| (&k.value.id, &k.value.subject))
        .collect();
    for s in &c.subjects {
        let subject = &s.value;
        let who = format!("subject `{}`", subject.id);
        match kc_subject.get(&subject.spelling_kc) {
            None => {
                let message = format!("unknown spelling KC `{}`", subject.spelling_kc);
                diags.push(diag(Code::UnknownReference, &s.file, who, message));
            }
            Some(&of) if of != &subject.id => {
                let message = format!(
                    "spelling KC `{}` belongs to subject `{of}`",
                    subject.spelling_kc
                );
                diags.push(diag(Code::KcOutsideSubject, &s.file, who, message));
            }
            Some(_) => {}
        }
    }
    for it in &c.items {
        let item = &it.value;
        for (j, slot) in item.body.slots().iter().enumerate() {
            for sk in &slot.kcs {
                let Some(&of) = kc_subject.get(&sk.kc) else {
                    continue;
                };
                if of != &item.subject {
                    let message = format!(
                        "slots[{j}]: KC `{}` belongs to subject `{of}`, the item to `{}`",
                        sk.kc, item.subject
                    );
                    let who = format!("item `{}`", item.id);
                    diags.push(diag(Code::KcOutsideSubject, &it.file, who, message));
                }
            }
        }
    }
}

/// `id` is `<subject>.<rest>`.
fn has_prefix(id: &str, subject: &SubjectId) -> bool {
    id.strip_prefix(subject.as_str())
        .is_some_and(|rest| rest.starts_with('.'))
}

fn prefix_error(id: &dyn std::fmt::Display, subject: &SubjectId, whose: &str) -> String {
    format!("id `{id}` must start with `{subject}.`, the subject of {whose}")
}

/// Subject prefixes of KC, misconception and family ids; `requires` edges
/// within one subject. References that do not resolve are reported elsewhere.
pub fn prefixes(c: &Content, diags: &mut Vec<Diagnostic>) {
    let subjects = by_id(c);
    let kc_subject: BTreeMap<&KcId, &SubjectId> = c
        .kcs
        .iter()
        .map(|k| (&k.value.id, &k.value.subject))
        .collect();
    for k in &c.kcs {
        let kc = &k.value;
        if subjects.contains_key(&kc.subject) && !has_prefix(kc.id.as_str(), &kc.subject) {
            let message = prefix_error(&kc.id, &kc.subject, "the KC");
            diags.push(diag(
                Code::SubjectPrefix,
                &k.file,
                format!("KC `{}`", kc.id),
                message,
            ));
        }
    }
    for m in &c.misconceptions {
        let mis = &m.value;
        let of_kcs: BTreeSet<&SubjectId> = mis
            .affects
            .iter()
            .chain([&mis.remedy])
            .filter_map(|kc| kc_subject.get(kc).copied())
            .collect();
        for s in of_kcs {
            if !has_prefix(mis.id.as_str(), s) {
                let message = prefix_error(&mis.id, s, "the KCs it affects");
                let subject = format!("misconception `{}`", mis.id);
                diags.push(diag(Code::SubjectPrefix, &m.file, subject, message));
            }
        }
    }
    for it in &c.items {
        let item = &it.value;
        let subject = format!("item `{}`", item.id);
        if !subjects.contains_key(&item.subject) {
            let message = format!("unknown subject `{}`", item.subject);
            diags.push(diag(Code::UnknownReference, &it.file, subject, message));
        } else if !has_prefix(item.family.as_str(), &item.subject) {
            let message = prefix_error(&item.family, &item.subject, "the item");
            diags.push(diag(Code::SubjectPrefix, &it.file, subject, message));
        }
    }
    for e in &c.edges {
        let edge = &e.value;
        let (Some(from), Some(to)) = (kc_subject.get(&edge.from), kc_subject.get(&edge.to)) else {
            continue;
        };
        if edge.kind == EdgeKind::Requires && from != to {
            let message = format!(
                "`requires` crosses subjects (`{from}` → `{to}`): one subject must not block \
                 another; use `supports`"
            );
            let subject = format!("edge `{}` requires `{}`", edge.from, edge.to);
            diags.push(diag(
                Code::RequiresAcrossSubjects,
                &e.file,
                subject,
                message,
            ));
        }
    }
}
