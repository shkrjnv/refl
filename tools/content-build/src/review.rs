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

use refl_core::content::{LangReviews, Review, text_hash};
use refl_core::item::Item;
use refl_core::value::{Lang, Localized};

use crate::load::{Content, Sourced};

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
    /// `hashes.txt`.
    pub hashes: String,
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

/// One line of the queue. The key — record id and field — is what names the
/// text; the place is only a hint, because matching a text to a line can hit
/// a duplicate.
type Queue = BTreeMap<Lang, Vec<(String, String, usize, String)>>;

fn push_entry(queue: &mut Queue, lang: Lang, key: String, place: (String, usize), what: String) {
    queue
        .entry(lang)
        .or_default()
        .push((key, place.0, place.1, what));
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
    let mut queue = Queue::new();

    for e in &content.explanations {
        let x = &e.value;
        if x.review.is_shown() {
            continue;
        }
        let place = sources.place(&e.file, &[&x.body]);
        let depth = format!("{:?}", x.depth).to_lowercase();
        let what = format!("explanation ({depth}): {}", quote(&x.body));
        push_entry(&mut queue, x.lang, format!("{}#body", x.id), place, what);
        if let Some(contrast) = &x.l1_contrast {
            let place = sources.place(&e.file, &[contrast]);
            let what = format!("explanation: L1 contrast {}", quote(contrast));
            let key = format!("{}#l1_contrast", x.id);
            push_entry(&mut queue, x.lang, key, place, what);
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
                let what = format!("instruction: {}", quote(text));
                let key = format!("{}#text", e.value.id);
                push_entry(&mut queue, lang, key, place, what);
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
                let what = format!("KC title {}", quote(text));
                let key = format!("{}#title", e.value.id);
                push_entry(&mut queue, lang, key, place, what);
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
                let what = format!("misconception title {}", quote(text));
                let key = format!("{}#title", e.value.id);
                push_entry(&mut queue, lang, key, place, what);
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
            let what = format!("task v{}: why and hint {}", item.version.0, quote(why));
            let key = format!("{}#why,hint", item.id);
            push_entry(&mut queue, lang, key, place, what);
        }
    }

    let mut out = String::new();
    for lang in LANGS {
        let _ = writeln!(out, "== {}", name(lang));
        let mut entries = queue.remove(&lang).unwrap_or_default();
        entries.sort();
        entries.dedup();
        for (key, file, line, what) in &entries {
            let _ = writeln!(out, "{key} — {what} (≈ {file}:{line})");
        }
        let _ = writeln!(out, "({} to review)\n", entries.len());
    }
    ReviewFiles {
        queue: out,
        tasks: tasks_file(content),
        hashes: hashes_file(content),
    }
}

/// A text a review record covers, with the hash of exactly that text and the
/// review records that must carry the hash.
#[derive(Debug)]
pub struct Covered<'a> {
    /// `explanation:<id>`, `instruction:<id>`, `kc:<id>`, `misconception:<id>`,
    /// `item:<id>`, or `group:<name>` for a shared task review.
    pub key: String,
    pub lang: Lang,
    pub file: PathBuf,
    pub hash: String,
    /// Every record whose `reviewed` status claims this text.
    pub reviews: Vec<&'a Review>,
}

/// Every review-covered text of the content, hashed in the fixed order of
/// `docs/02-layer1-content.md`, «Вычитка и показ». Records without text in
/// a language are left out.
pub fn covered(content: &Content) -> Vec<Covered<'_>> {
    fn push<'a>(
        out: &mut Vec<Covered<'a>>,
        key: String,
        lang: Lang,
        file: &Path,
        parts: &[&str],
        reviews: Vec<&'a Review>,
    ) {
        if parts.iter().all(|p| p.is_empty()) {
            return;
        }
        out.push(Covered {
            key,
            lang,
            file: file.to_path_buf(),
            hash: text_hash(parts),
            reviews,
        });
    }
    let mut out = Vec::new();
    for e in &content.explanations {
        let x = &e.value;
        push(
            &mut out,
            format!("explanation:{}", x.id),
            x.lang,
            &e.file,
            &x.covered_text(),
            vec![&x.review],
        );
    }
    for lang in LANGS {
        for e in &content.instructions {
            let reviews = e.value.review.get(lang).into_iter().collect();
            push(
                &mut out,
                format!("instruction:{}", e.value.id),
                lang,
                &e.file,
                &e.value.covered_text(lang),
                reviews,
            );
        }
        for e in &content.kcs {
            let reviews = e.value.review.get(lang).into_iter().collect();
            push(
                &mut out,
                format!("kc:{}", e.value.id),
                lang,
                &e.file,
                &e.value.covered_text(lang),
                reviews,
            );
        }
        for e in &content.misconceptions {
            let reviews = e.value.review.get(lang).into_iter().collect();
            push(
                &mut out,
                format!("misconception:{}", e.value.id),
                lang,
                &e.file,
                &e.value.covered_text(lang),
                reviews,
            );
        }
        // Items: alone, or together with the others of their group, in id order.
        let mut groups: BTreeMap<String, Vec<&Sourced<Item>>> = BTreeMap::new();
        for e in &content.items {
            let key = e
                .value
                .review_group
                .as_ref()
                .map_or_else(|| format!("item:{}", e.value.id), |g| format!("group:{g}"));
            groups.entry(key).or_default().push(e);
        }
        for (key, mut members) in groups {
            members.sort_by(|a, b| a.value.id.cmp(&b.value.id));
            let parts: Vec<&str> = members
                .iter()
                .flat_map(|m| m.value.covered_text(lang))
                .collect();
            let reviews = members
                .iter()
                .filter_map(|m| m.value.review.get(lang))
                .collect();
            push(&mut out, key, lang, &members[0].file, &parts, reviews);
        }
    }
    out
}

/// `hashes.txt`: the hash to set in a record when a person marks its text
/// reviewed — `key`, language, hash.
fn hashes_file(content: &Content) -> String {
    let mut out = String::from(
        "# record\tlang\thash: set `status = \"reviewed\"`, `by` and this `hash` in the record's review\n",
    );
    for c in covered(content) {
        let _ = writeln!(out, "{}\t{}\t{}", c.key, name(c.lang), c.hash);
    }
    out
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
