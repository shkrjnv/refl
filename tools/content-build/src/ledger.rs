//! The review ledger `content/reviews.lock`: who reviewed which text, and the
//! hash of the text they read (`docs/02-layer1-content.md`, «Вычитка и
//! показ»). `content-build mark-reviewed` is the only writer. Records built
//! by Nickel functions cannot hold a hash of their own in the source, so the
//! reviewed state lives here, one JSON line per record and language, sorted,
//! and is merged into the records when the content is loaded; a text edited
//! after its entry was written no longer matches the hash and is the build
//! error `reviewed-text-changed`.

use std::path::Path;

use refl_core::content::{LangReviews, Review, ReviewStatus};
use refl_core::value::Lang;
use serde::{Deserialize, Serialize};

use crate::load::Content;

/// File name, next to `item-versions.lock`.
pub const FILE: &str = "reviews.lock";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entry {
    /// `explanation:<id>`, `instruction:<id>`, `kc:<id>`, `misconception:<id>`,
    /// `item:<id>` or `group:<name>` (see [`crate::review::Covered`]).
    pub record: String,
    pub lang: Lang,
    pub by: String,
    pub hash: String,
}

impl Entry {
    fn review(&self) -> Review {
        Review {
            status: ReviewStatus::Reviewed,
            by: Some(self.by.clone()),
            hash: Some(self.hash.clone()),
        }
    }
}

/// The entries of the ledger at `path`; a missing file is an empty ledger.
///
/// # Errors
/// I/O errors; `Ok(Err(why))` for a file that cannot be parsed.
pub fn read(path: &Path) -> std::io::Result<Result<Vec<Entry>, String>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Ok(Vec::new())),
        Err(e) => return Err(e),
    };
    let mut entries = Vec::new();
    for (n, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        match serde_json::from_str::<Entry>(line) {
            Ok(e) => entries.push(e),
            Err(e) => return Ok(Err(format!("line {}: {e}", n + 1))),
        }
    }
    Ok(Ok(entries))
}

/// The file text: entries sorted by (record, language), one per line.
///
/// # Panics
/// Never: entries always serialize.
pub fn render(entries: &[Entry]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort();
    sorted.dedup_by(|a, b| a.record == b.record && a.lang == b.lang);
    let mut out = String::new();
    for e in &sorted {
        out.push_str(&serde_json::to_string(e).expect("entry serializes"));
        out.push('\n');
    }
    out
}

fn set(reviews: &mut LangReviews, lang: Lang, review: Review) {
    match lang {
        Lang::Uz => reviews.uz = Some(review),
        Lang::Ru => reviews.ru = Some(review),
        Lang::En => reviews.en = Some(review),
    }
}

/// Merges the ledger into the records: the review of the entry's language
/// becomes `reviewed` by the named person with the recorded hash. Returns
/// the entries that name no existing record (or review group): orphans,
/// the build error `orphan-review` (docs/02, «Проверки при сборке»).
pub fn apply<'e>(content: &mut Content, entries: &'e [Entry]) -> Vec<&'e Entry> {
    let mut orphans = Vec::new();
    for e in entries {
        let Some((kind, id)) = e.record.split_once(':') else {
            orphans.push(e);
            continue;
        };
        let mut found = false;
        match kind {
            "explanation" => {
                for x in content
                    .explanations
                    .iter_mut()
                    .filter(|x| x.value.id.as_str() == id && x.value.lang == e.lang)
                {
                    x.value.review = e.review();
                    found = true;
                }
            }
            "instruction" => {
                for x in content
                    .instructions
                    .iter_mut()
                    .filter(|x| x.value.id.as_str() == id)
                {
                    set(&mut x.value.review, e.lang, e.review());
                    found = true;
                }
            }
            "kc" => {
                for x in content.kcs.iter_mut().filter(|x| x.value.id.as_str() == id) {
                    set(&mut x.value.review, e.lang, e.review());
                    found = true;
                }
            }
            "misconception" => {
                for x in content
                    .misconceptions
                    .iter_mut()
                    .filter(|x| x.value.id.as_str() == id)
                {
                    set(&mut x.value.review, e.lang, e.review());
                    found = true;
                }
            }
            "item" => {
                for x in content
                    .items
                    .iter_mut()
                    .filter(|x| x.value.id.as_str() == id)
                {
                    set(&mut x.value.review, e.lang, e.review());
                    found = true;
                }
            }
            "group" => {
                for x in content
                    .items
                    .iter_mut()
                    .filter(|x| x.value.review_group.as_deref() == Some(id))
                {
                    set(&mut x.value.review, e.lang, e.review());
                    found = true;
                }
            }
            _ => {}
        }
        if !found {
            orphans.push(e);
        }
    }
    orphans
}

/// Which records `mark-reviewed` marks.
#[derive(Debug, Clone)]
pub enum Selection {
    /// Every record with text in the language.
    All,
    /// Records by id (`expl.x.short.ru`, `en.gram.noun.countability`) or by
    /// full key (`kc:en.gram.noun.countability`), or a `review_group` name.
    Records(Vec<String>),
}

/// Marks the selected records reviewed in `lang` by `by`: computes the hash
/// of each record's current text and writes it to the ledger at `ledger`
/// (replacing an older entry of the same record and language). Returns the
/// keys marked.
///
/// # Errors
/// The content or the ledger cannot be read or written, a selector matches
/// no record with text in the language, or `by` is empty.
pub fn mark_reviewed(
    content: &Content,
    ledger: &Path,
    by: &str,
    lang: Lang,
    selection: &Selection,
) -> Result<Vec<String>, String> {
    if by.trim().is_empty() {
        return Err("--by needs the name of the person who read the texts".to_owned());
    }
    let covered: Vec<_> = crate::review::covered(content)
        .into_iter()
        .filter(|c| c.lang == lang)
        .collect();
    let chosen: Vec<&crate::review::Covered<'_>> = match selection {
        Selection::All => covered.iter().collect(),
        Selection::Records(selectors) => {
            let mut chosen = Vec::new();
            for s in selectors {
                let before = chosen.len();
                chosen.extend(covered.iter().filter(|c| {
                    c.key == *s || c.key.split_once(':').is_some_and(|(_, id)| id == s)
                }));
                if chosen.len() == before {
                    return Err(format!("`{s}` is no record with text in {lang:?}"));
                }
            }
            chosen
        }
    };
    let mut entries = read(ledger)
        .map_err(|e| format!("cannot read {}: {e}", ledger.display()))?
        .map_err(|why| format!("{}: {why}", ledger.display()))?;
    entries.retain(|e| !(e.lang == lang && chosen.iter().any(|c| c.key == e.record)));
    let mut keys = Vec::new();
    for c in chosen {
        entries.push(Entry {
            record: c.key.clone(),
            lang,
            by: by.to_owned(),
            hash: c.hash.clone(),
        });
        keys.push(c.key.clone());
    }
    std::fs::write(ledger, render(&entries))
        .map_err(|e| format!("cannot write {}: {e}", ledger.display()))?;
    keys.sort();
    keys.dedup();
    Ok(keys)
}
