//! `report --review`: the review queue and the tasks as a learner sees them
//! (`docs/02-layer1-content.md`, «Вычитка и показ», «Инструменты автора»).
//!
//! The queue lists every written text that is still draft, by language, as
//! `path:line — what it is`. A text exported from Nickel has no source
//! position, so the line is the nearest one: the first line of the file that
//! holds the longest run of words the text shares with the source. For a
//! text built from a template, that run is the literal part around the
//! substituted words.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use refl_core::content::LangReviews;
use refl_core::item::Item;
use refl_core::value::{Lang, Localized};

use crate::load::Content;

const LANGS: [Lang; 3] = [Lang::Uz, Lang::Ru, Lang::En];

fn name(lang: Lang) -> &'static str {
    match lang {
        Lang::Uz => "uz",
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

#[derive(Debug)]
pub struct ReviewFiles {
    /// `queue.txt`.
    pub queue: String,
    /// `tasks.md`.
    pub tasks: String,
}

/// The sources of the content files, read once.
struct Sources<'a> {
    root: &'a Path,
    files: BTreeMap<PathBuf, String>,
}

impl<'a> Sources<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            files: BTreeMap::new(),
        }
    }

    fn text(&mut self, file: &Path) -> &str {
        let root = self.root;
        self.files
            .entry(file.to_path_buf())
            .or_insert_with(|| std::fs::read_to_string(root.join(file)).unwrap_or_default())
    }

    /// Where an entry is: the file as the author opens it, and the line.
    fn place(&mut self, file: &Path, needles: &[&str]) -> (String, usize) {
        let line = locate(self.text(file), needles);
        (self.root.join(file).display().to_string(), line)
    }
}

/// The 1-based line of `source` nearest to the first needle that has a
/// match: the longest run (at least two words, 12 characters) of the needle's
/// words that appears on one line. Line 1 if nothing matches.
fn locate(source: &str, needles: &[&str]) -> usize {
    for needle in needles {
        let words: Vec<&str> = needle.split_whitespace().collect();
        for n in (2..=words.len().min(10)).rev() {
            for window in words.windows(n) {
                let run = window.join(" ");
                if run.chars().count() < 12 {
                    continue;
                }
                if let Some(i) = source.lines().position(|l| l.contains(&run)) {
                    return i + 1;
                }
            }
        }
    }
    1
}

/// The text cut to a short quotation for the queue.
fn quote(text: &str) -> String {
    let flat = text.replace('\n', " ");
    let mut short: String = flat.chars().take(60).collect();
    if flat.chars().count() > 60 {
        short.push('…');
    }
    format!("“{short}”")
}

fn push_entry(
    queue: &mut BTreeMap<Lang, Vec<(String, usize, String)>>,
    lang: Lang,
    place: (String, usize),
    what: String,
) {
    queue
        .entry(lang)
        .or_default()
        .push((place.0, place.1, what));
}

/// Whether the item has any `why` or `hint` text in `lang`.
fn has_task_text(item: &Item, lang: Lang) -> bool {
    item.body
        .slots()
        .iter()
        .any(|s| s.why.get(lang).is_some() || s.hint.as_ref().and_then(|h| h.get(lang)).is_some())
}

fn reviewed(reviews: &LangReviews, lang: Lang) -> bool {
    reviews.is_shown(lang)
}

/// Both files. Call it only on content that passed the build checks: the
/// queue holds what is left for a person.
///
/// # Panics
/// Never: writing to a `String` does not fail.
pub fn review_files(root: &Path, content: &Content) -> ReviewFiles {
    let mut sources = Sources::new(root);
    let mut queue: BTreeMap<Lang, Vec<(String, usize, String)>> = BTreeMap::new();

    for e in &content.explanations {
        let x = &e.value;
        if x.review.is_shown() {
            continue;
        }
        let place = sources.place(&e.file, &[&x.body]);
        let depth = format!("{:?}", x.depth).to_lowercase();
        let what = format!("explanation `{}` ({depth}): {}", x.id, quote(&x.body));
        push_entry(&mut queue, x.lang, place, what);
        if let Some(contrast) = &x.l1_contrast {
            let place = sources.place(&e.file, &[contrast]);
            let what = format!("explanation `{}`: L1 contrast {}", x.id, quote(contrast));
            push_entry(&mut queue, x.lang, place, what);
        }
    }
    for e in &content.instructions {
        for lang in LANGS {
            if let Some(text) = e
                .value
                .text
                .get(lang)
                .filter(|_| !reviewed(&e.value.review, lang))
            {
                let place = sources.place(&e.file, &[text]);
                let what = format!("instruction `{}`: {}", e.value.id, quote(text));
                push_entry(&mut queue, lang, place, what);
            }
        }
    }
    for e in &content.kcs {
        for lang in LANGS {
            if let Some(text) = e
                .value
                .title
                .get(lang)
                .filter(|_| !reviewed(&e.value.review, lang))
            {
                let place = sources.place(&e.file, &[text]);
                let what = format!("KC `{}`: title {}", e.value.id, quote(text));
                push_entry(&mut queue, lang, place, what);
            }
        }
    }
    for e in &content.misconceptions {
        for lang in LANGS {
            if let Some(text) = e
                .value
                .title
                .get(lang)
                .filter(|_| !reviewed(&e.value.review, lang))
            {
                let place = sources.place(&e.file, &[text]);
                let what = format!("misconception `{}`: title {}", e.value.id, quote(text));
                push_entry(&mut queue, lang, place, what);
            }
        }
    }
    for e in &content.items {
        let item = &e.value;
        for lang in LANGS {
            if reviewed(&item.review, lang) || !has_task_text(item, lang) {
                continue;
            }
            let slot = &item.body.slots()[0];
            let why = slot.why.get(lang).unwrap_or_default();
            let hint = slot
                .hint
                .as_ref()
                .and_then(|h| h.get(lang))
                .unwrap_or_default();
            let place = sources.place(&e.file, &[why, hint]);
            let what = format!(
                "task `{}` v{}: why and hint {}",
                item.id,
                item.version.0,
                quote(why)
            );
            push_entry(&mut queue, lang, place, what);
        }
    }

    let mut out = String::new();
    for lang in LANGS {
        let _ = writeln!(out, "== {}", name(lang));
        let mut entries = queue.remove(&lang).unwrap_or_default();
        entries.sort();
        entries.dedup();
        for (file, line, what) in &entries {
            let _ = writeln!(out, "{file}:{line} — {what}");
        }
        let _ = writeln!(out, "({} to review)\n", entries.len());
    }
    ReviewFiles {
        queue: out,
        tasks: tasks_file(content),
    }
}

fn localized(text: &Localized, lang: Lang) -> &str {
    text.get(lang).unwrap_or("—")
}

/// Every task, in id order, as a learner sees it in each language, with the
/// review status of its `why` and `hint` next to the language.
fn tasks_file(content: &Content) -> String {
    let mut items: Vec<&Item> = content.items.iter().map(|i| &i.value).collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = String::from(
        "# Tasks as a learner sees them\n\n\
         One block per task and language. The options are in authoring order, \
         the answer first (the app shuffles them). `[draft]` or `[reviewed by …]` \
         is the review of the `why` and `hint` in that language.\n",
    );
    for item in items {
        let _ = writeln!(
            out,
            "\n## {} v{} · {} · difficulty {} · {}",
            item.id,
            item.version.0,
            format!("{:?}", item.body.tag()).to_lowercase(),
            item.difficulty.get(),
            format!("{:?}", item.status).to_lowercase()
        );
        let _ = writeln!(out, "\nStimulus: {}", item.body.stimulus().text);
        for lang in LANGS {
            let status = match item.review.get(lang) {
                Some(r) if r.is_shown() => format!("reviewed by {}", r.by.as_deref().unwrap_or("")),
                _ => "draft".to_owned(),
            };
            let _ = writeln!(out, "\n### {} [{status}]", name(lang));
            let instruction = content
                .instructions
                .iter()
                .find(|i| i.value.id == item.instruction)
                .map_or("—", |i| localized(&i.value.text, lang));
            let _ = writeln!(out, "\nInstruction: {instruction}");
            for (n, slot) in item.body.slots().iter().enumerate() {
                if item.body.slots().len() > 1 {
                    let _ = writeln!(out, "\nGap {}:", n + 1);
                }
                if !slot.options.is_empty() {
                    let options: Vec<&str> = std::iter::once(slot.answer.as_str())
                        .chain(slot.options.iter().map(|o| o.text.as_str()))
                        .collect();
                    let _ = writeln!(out, "- Options: {}", options.join(" · "));
                }
                let _ = writeln!(out, "- Answer: {}", slot.answer);
                let _ = writeln!(out, "- Why: {}", localized(&slot.why, lang));
                if let Some(hint) = &slot.hint {
                    let _ = writeln!(out, "- Hint: {}", localized(hint, lang));
                }
            }
        }
    }
    out
}
