//! Subject checks (`docs/10-subjects.md`, «Предмет», «Шкала уровней»;
//! `docs/02-layer1-content.md`, «Проверки при сборке контента»): every KC
//! names an existing subject, a level key of its scale and one of its
//! domains; level keys and domains are unique within a subject.

use std::collections::{BTreeMap, BTreeSet};

use refl_core::ids::SubjectId;
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
