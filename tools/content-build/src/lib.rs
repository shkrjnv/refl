//! content-build: compiles `content/` (Nickel) into one deterministic JSON
//! bundle and runs every check from `docs/02-layer1-content.md`, «Проверки
//! при сборке контента». Errors fail the build; warnings and KCs that stay
//! draft only go to the report.

pub mod check;
pub mod graph;
pub mod ledger;
pub mod lexicon;
pub mod load;
pub mod lock;
pub mod report;
pub mod review;
pub mod status;
pub mod subjects;
pub mod templates;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use refl_core::answer::normalize::NORMALIZATION_VERSION;
use refl_core::bundle::{
    BUNDLE_FORMAT_VERSION, Bundle, BundleContent, ItemEntry, KcEntry, KcItems, KcStatus,
};
use refl_core::content::EdgeKind;
use refl_core::ids::{ContentVersion, ItemId, ItemRef, KcId, LexemeId, SubjectId};
use refl_core::item::{ItemStatus, KcRole};
use refl_core::lexicon::Lexicon;
use sha2::{Digest, Sha256};

use crate::report::Report;

#[derive(Debug, Clone)]
pub struct Options {
    /// Content root.
    pub content: PathBuf,
    /// Root that audio references resolve against.
    pub audio_root: PathBuf,
    /// The `nickel` executable.
    pub nickel: PathBuf,
    /// `share/dict` of the nixpkgs `scowl` package.
    pub scowl: PathBuf,
    /// The item versions lock file; a missing file is an empty lock.
    pub lock: PathBuf,
    /// The lock must not change: a build that would append an entry fails
    /// (`lock-outdated`). For CI, where nothing may write the file.
    pub locked: bool,
    /// The review ledger; a missing file is an empty ledger. `mark-reviewed`
    /// writes it.
    pub reviews: PathBuf,
}

impl Options {
    /// Audio under `<content>/audio`, the lock at `<content>/item-versions.lock`,
    /// `nickel` from `PATH`, SCOWL from `REFL_SCOWL_DICT` (the dev shell and
    /// the flake checks set it).
    pub fn new(content: impl Into<PathBuf>) -> Self {
        let content = content.into();
        Self {
            audio_root: content.join("audio"),
            lock: content.join(lock::FILE),
            locked: false,
            reviews: content.join(ledger::FILE),
            content,
            nickel: PathBuf::from("nickel"),
            scowl: std::env::var_os("REFL_SCOWL_DICT").map_or_else(PathBuf::new, PathBuf::from),
        }
    }
}

#[derive(Debug)]
pub struct Output {
    pub report: Report,
    /// `None` when the build has errors.
    pub bundle: Option<Bundle>,
    /// The lexicon the bundle references; `None` with the bundle.
    pub lexicon: Option<Lexicon>,
    /// The new text of the lock file when the build appended entries; the
    /// caller writes it to [`Options::lock`]. `None` when the lock is
    /// unchanged or the build has errors.
    pub lock_update: Option<String>,
}

impl Output {
    /// The result of a build with errors: the report alone.
    fn rejected(report: Report) -> Self {
        Self {
            report,
            bundle: None,
            lexicon: None,
            lock_update: None,
        }
    }
}

/// Loads, checks and compiles the content.
///
/// # Errors
/// The content root, the lock file or the SCOWL lists cannot be read. Content problems are reported in
/// [`Output::report`], not as errors.
///
/// # Panics
/// Never: bundle types always serialize.
pub fn build(opts: &Options) -> std::io::Result<Output> {
    build_with_content(opts).map(|(output, _)| output)
}

/// [`build`], and the loaded content with the file of every entity: the
/// review queue needs both (`report --review`).
///
/// # Errors
/// As [`build`].
///
/// # Panics
/// Never: bundle types always serialize.
/// Long because it coordinates loading, checks, the lock file and bundle assembly.
#[allow(clippy::too_many_lines)]
pub fn build_with_content(opts: &Options) -> std::io::Result<(Output, load::Content)> {
    let (mut content, mut diags) = load::load(&opts.content, &opts.nickel).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("cannot read {}: {e}", opts.content.display()),
        )
    })?;
    match ledger::read(&opts.reviews)? {
        Ok(entries) => {
            for e in ledger::apply(&mut content, &entries) {
                diags.push(check::diag(
                    report::Code::OrphanReview,
                    std::path::Path::new(ledger::FILE),
                    format!("review of `{}` in {:?}", e.record, e.lang),
                    "no such record: remove the line with the record it reviewed".to_owned(),
                ));
            }
        }
        Err(why) => diags.push(check::diag(
            report::Code::Schema,
            std::path::Path::new(ledger::FILE),
            format!("review ledger `{}`", opts.reviews.display()),
            why,
        )),
    }
    let lexicon = lexicon::load_scowl(&opts.scowl)?;
    let ids = check::duplicates(&content, &mut diags);
    check::references(&content, &ids, &mut diags);
    check_lexemes(&content, &ids, &mut diags);
    subjects::check(&content, &mut diags);
    subjects::prefixes(&content, &mut diags);
    subjects::kc_subjects(&content, &mut diags);
    check::misconception_feedback(&content, &mut diags);
    check::instruction_text(&content, &mut diags);
    check::reviewers(&content, &mut diags);
    check::reviewed_texts(&content, &mut diags);
    let requires = graph::Requires::new(&content, &ids);
    requires.check_acyclic(&mut diags);
    let compiled = templates::compile(&content.items, &mut diags);
    check::live_items(&content, &compiled, &mut diags);
    check::audio(&content, &opts.audio_root, &mut diags);
    let (lock, lock_update) = item_versions(opts, &content, &mut diags)?;

    let mut report = Report {
        diagnostics: diags,
        ..Report::default()
    };
    report.lexemes = content.lexemes.len();
    let status = status::compute(&content, &mut report);
    if report.has_errors() {
        return Ok((Output::rejected(report), content));
    }

    let requires_closure = requires.closure(&ids);
    let mut kcs = kc_entries(&content, &status);
    apply_layout(&content, &requires_closure, &status, &mut kcs);
    let (items, items_by_kc) = item_entries(&content, &status, &compiled);
    check::redundant_supports(&content, &requires_closure, &mut report.diagnostics);
    check::secondary_downstream(&content, &requires_closure, &mut report.diagnostics);
    check::family_instruction(&content, &mut report.diagnostics);
    check::implied_prerequisites(
        &content,
        &items,
        &items_by_kc,
        &requires_closure,
        &mut report.diagnostics,
    );
    if report.has_errors() {
        return Ok((Output::rejected(report), content));
    }

    let mut edges: Vec<_> = content.edges.iter().map(|e| e.value.clone()).collect();
    edges.sort();
    let body = BundleContent {
        normalization_version: NORMALIZATION_VERSION,
        lexicon: lexicon::reference(&lexicon),
        subjects: sorted(&content.subjects, |s| &s.id),
        kcs,
        lexemes: sorted(&content.lexemes, |l| &l.id),
        item_lexemes: item_lexemes(&content),
        edges,
        requires_closure,
        misconceptions: sorted(&content.misconceptions, |m| &m.id),
        explanations: sorted(&content.explanations, |e| &e.id),
        instructions: sorted(&content.instructions, |i| &i.id),
        lessons: sorted(&content.lessons, |l| &l.id),
        items,
        item_versions: lock.entries().to_vec(),
        items_by_kc,
        tokens: compiled.tokens,
    };
    let hash = Sha256::digest(serde_json::to_vec(&body).expect("bundle serializes"));
    let content_version = ContentVersion(hash.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    }));
    let output = Output {
        report,
        bundle: Some(Bundle {
            format_version: BUNDLE_FORMAT_VERSION,
            content_version,
            content: body,
        }),
        lexicon: Some(lexicon),
        lock_update,
    };
    Ok((output, content))
}

/// The lock after this build and, unless `opts.locked`, its new text when it
/// differs from the file. A lock that cannot be trusted is reported and
/// replaced by an empty one: the build fails on the report anyway.
fn item_versions(
    opts: &Options,
    content: &load::Content,
    diags: &mut Vec<report::Diagnostic>,
) -> std::io::Result<(lock::Lock, Option<String>)> {
    let file = std::path::Path::new(lock::FILE);
    let old = match lock::read(&opts.lock)? {
        Ok(lock) => lock,
        Err(why) => {
            diags.push(check::diag(
                report::Code::LockCorrupt,
                file,
                format!("lock file `{}`", opts.lock.display()),
                why,
            ));
            return Ok((lock::Lock::default(), None));
        }
    };
    let new = lock::reconcile(content, &old, diags);
    if new == old {
        return Ok((new, None));
    }
    if opts.locked {
        let added = new.entries().len() - old.entries().len();
        diags.push(check::diag(
            report::Code::LockOutdated,
            file,
            format!("lock file `{}`", opts.lock.display()),
            format!("{added} published item version(s) are not in the lock; run content-build without --locked and commit the file"),
        ));
        return Ok((new, None));
    }
    let text = new.render();
    Ok((new, Some(text)))
}

/// Items with their automata and session eligibility, sorted by id, and
/// the session-eligible items indexed by KC.
fn item_entries(
    content: &load::Content,
    status: &BTreeMap<KcId, KcStatus>,
    compiled: &templates::Compiled,
) -> (Vec<ItemEntry>, BTreeMap<KcId, KcItems>) {
    let mut items_by_kc: BTreeMap<_, KcItems> = BTreeMap::new();
    let mut items = Vec::with_capacity(content.items.len());
    for (i, it) in content.items.iter().enumerate() {
        let item = &it.value;
        let primary_live = item
            .body
            .slots()
            .iter()
            .flat_map(|s| &s.kcs)
            .filter(|k| k.role == KcRole::Primary)
            .all(|k| status.get(&k.kc) == Some(&KcStatus::Live));
        // Live, live primary KCs, and `why` and `hint` reviewed in ru and en.
        let session_eligible =
            item.status == ItemStatus::Live && primary_live && item.is_reviewed();
        if session_eligible {
            let r = ItemRef {
                id: item.id.clone(),
                version: item.version,
                seed: None,
            };
            for sk in item.body.slots().iter().flat_map(|s| &s.kcs) {
                let entry = items_by_kc.entry(sk.kc.clone()).or_default();
                match sk.role {
                    KcRole::Primary => entry.primary.push(r.clone()),
                    KcRole::Secondary => entry.secondary.push(r.clone()),
                }
            }
        }
        items.push(ItemEntry {
            item: item.clone(),
            session_eligible,
            slots: compiled.items[&i].clone(),
        });
    }
    items.sort_by(|a, b| a.item.id.cmp(&b.item.id));
    for refs in items_by_kc.values_mut() {
        for list in [&mut refs.primary, &mut refs.secondary] {
            list.sort();
            list.dedup();
        }
    }
    (items, items_by_kc)
}

/// KCs with computed status and level rank, sorted by id. Only called
/// without errors, so every subject and level resolves.
fn kc_entries(content: &load::Content, status: &BTreeMap<KcId, KcStatus>) -> Vec<KcEntry> {
    let by_subject = subjects::by_id(content);
    let mut kcs: Vec<KcEntry> = content
        .kcs
        .iter()
        .map(|k| KcEntry {
            kc: k.value.clone(),
            status: status[&k.value.id],
            level_rank: by_subject[&k.value.subject]
                .scale
                .rank(&k.value.level)
                .expect("subjects::check rejects unknown levels"),
            layout_x: 0,
            layout_y: 0,
        })
        .collect();
    kcs.sort_by(|a, b| a.kc.id.cmp(&b.kc.id));
    kcs
}

/// Values of `entries`, sorted by `key`.
fn sorted<T: Clone, K: Ord>(entries: &[load::Sourced<T>], key: impl Fn(&T) -> &K) -> Vec<T> {
    let mut v: Vec<T> = entries.iter().map(|e| e.value.clone()).collect();
    v.sort_by(|a, b| key(a).cmp(key(b)));
    v
}

/// Every lexical cross-check lives here so one pass reports all problems.
#[allow(clippy::too_many_lines)]
fn check_lexemes(content: &load::Content, ids: &check::Ids, diags: &mut Vec<report::Diagnostic>) {
    let by_subject = subjects::by_id(content);
    let mut lexeme_ids = std::collections::BTreeSet::new();
    let mut sense_ids = std::collections::BTreeSet::new();
    for entry in &content.lexemes {
        let lexeme = &entry.value;
        let subject = lexeme.id.as_str().split('.').next().unwrap_or("en");
        let Some(subject_id) = SubjectId::new(subject) else {
            diags.push(check::diag(
                report::Code::SubjectPrefix,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "lexeme id must start with a subject id".to_owned(),
            ));
            continue;
        };
        if !lexeme_ids.insert(lexeme.id.clone()) {
            diags.push(check::diag(
                report::Code::DuplicateId,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "lexeme id already defined".to_owned(),
            ));
        }
        if !lexeme.id.as_str().starts_with(&format!("{subject}.lex.")) {
            diags.push(check::diag(
                report::Code::SubjectPrefix,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "lexeme id must have the prefix `<subject>.lex.`".to_owned(),
            ));
        }
        if lexeme.lemma.trim().is_empty() {
            diags.push(check::diag(
                report::Code::Schema,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "lemma is empty".to_owned(),
            ));
        }
        if lexeme.source.trim().is_empty() || lexeme.license.trim().is_empty() {
            diags.push(check::diag(
                report::Code::Schema,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "source and license are required".to_owned(),
            ));
        }
        if lexeme.senses.is_empty() {
            diags.push(check::diag(
                report::Code::Schema,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "a lexeme needs at least one sense".to_owned(),
            ));
        }
        if !(1..=10_000).contains(&lexeme.frequency_rank) {
            diags.push(check::diag(
                report::Code::Schema,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                "frequency_rank must be 1..=10000".to_owned(),
            ));
        }
        if by_subject
            .get(&subject_id)
            .is_none_or(|s| s.scale.rank(&lexeme.level).is_none())
        {
            diags.push(check::diag(
                report::Code::UnknownLevel,
                &entry.file,
                format!("lexeme `{}`", lexeme.id),
                format!(
                    "unknown level `{}` for subject {subject}",
                    lexeme.level.as_str()
                ),
            ));
        }
        for kc in &lexeme.kc_ids {
            if !ids.kcs.contains(kc) {
                diags.push(check::diag(
                    report::Code::UnknownReference,
                    &entry.file,
                    format!("lexeme `{}`", lexeme.id),
                    format!("unknown linked KC `{kc}`"),
                ));
            }
        }
        for sense in &lexeme.senses {
            if !sense.id.as_str().starts_with(&format!("{subject}.lex.")) {
                diags.push(check::diag(
                    report::Code::SubjectPrefix,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "sense id must have the prefix `<subject>.lex.`".to_owned(),
                ));
            }
            if !sense.id.as_str().starts_with(&format!("{}.", lexeme.id)) {
                diags.push(check::diag(
                    report::Code::SubjectPrefix,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    format!("sense id must start with `{}.`", lexeme.id),
                ));
            }
            if !sense_ids.insert(sense.id.clone()) {
                diags.push(check::diag(
                    report::Code::DuplicateId,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "sense id already defined".to_owned(),
                ));
            }
            if sense.definition_en.trim().is_empty() {
                diags.push(check::diag(
                    report::Code::Schema,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "definition_en is empty".to_owned(),
                ));
            }
            if sense.definition_en.split_whitespace().count() > 25 {
                diags.push(check::diag(
                    report::Code::Schema,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "definition_en must be at most 25 words".to_owned(),
                ));
            }
            if !(1..=3).contains(&sense.examples.len()) {
                diags.push(check::diag(
                    report::Code::Schema,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "a sense needs 1..=3 examples".to_owned(),
                ));
            }
            if !(2..=5).contains(&sense.collocations.len()) {
                diags.push(check::diag(
                    report::Code::Schema,
                    &entry.file,
                    format!("sense `{}`", sense.id),
                    "a sense needs 2..=5 collocations".to_owned(),
                ));
            }
        }
    }
}

/// Word boundaries shared with `book-build`: ASCII letters, apostrophes and
/// hyphens; punctuation and digits separate words. Spans keep their case.
pub fn word_spans(text: &str) -> impl Iterator<Item = &str> + '_ {
    text.split(|c: char| !(c.is_ascii_alphabetic() || c == '\'' || c == '-'))
        .filter(|word| !word.is_empty())
}

/// [`word_spans`], lowercased for matching against the lexical bank.
pub fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    word_spans(text).map(str::to_ascii_lowercase)
}

/// Current item id → lexical senses in its stimulus and reference answers.
/// Ids are sense ids, because a sense is the unit of a lexical card. This is
/// intentionally a separate index, not an item projection: published
/// projections stay append-only.
fn item_lexemes(content: &load::Content) -> BTreeMap<ItemId, Vec<LexemeId>> {
    let mut by_form: BTreeMap<String, std::collections::BTreeSet<LexemeId>> = BTreeMap::new();
    for entry in &content.lexemes {
        let lexeme = &entry.value;
        let mut forms = vec![lexeme.lemma.to_ascii_lowercase()];
        forms.extend(lexeme.forms.iter().map(|form| form.to_ascii_lowercase()));
        for form in forms {
            by_form
                .entry(form)
                .or_default()
                .extend(lexeme.senses.iter().map(|sense| sense.id.clone()));
        }
    }
    let mut out = BTreeMap::new();
    for entry in &content.items {
        let item = &entry.value;
        let mut found = std::collections::BTreeSet::new();
        for text in std::iter::once(item.body.stimulus().text.as_str())
            .chain(item.body.slots().iter().map(|slot| slot.answer.as_str()))
        {
            for word in words(text) {
                if let Some(ids) = by_form.get(&word) {
                    found.extend(ids.iter().cloned());
                }
            }
        }
        if !found.is_empty() {
            out.insert(item.id.clone(), found.into_iter().collect());
        }
    }
    out
}

fn layout_layer<'a>(
    id: &'a KcId,
    parents: &BTreeMap<&'a KcId, Vec<&'a KcId>>,
    memo: &mut BTreeMap<&'a KcId, u16>,
) -> u16 {
    if let Some(value) = memo.get(id) {
        return *value;
    }
    let value = parents
        .get(id)
        .into_iter()
        .flatten()
        .map(|parent| layout_layer(parent, parents, memo).saturating_add(1))
        .max()
        .unwrap_or(0);
    memo.insert(id, value);
    value
}

fn apply_layout(
    content: &load::Content,
    _requires_closure: &BTreeMap<KcId, Vec<refl_core::bundle::Required>>,
    status: &BTreeMap<KcId, KcStatus>,
    kcs: &mut [KcEntry],
) {
    let live: BTreeSet<&KcId> = status
        .iter()
        .filter(|(_, s)| **s == KcStatus::Live)
        .map(|(id, _)| id)
        .collect();
    let mut parents: BTreeMap<&KcId, Vec<&KcId>> = BTreeMap::new();
    for edge in &content.edges {
        if edge.value.kind == EdgeKind::Requires
            && live.contains(&edge.value.from)
            && live.contains(&edge.value.to)
        {
            parents
                .entry(&edge.value.from)
                .or_default()
                .push(&edge.value.to);
        }
    }
    let mut memo = BTreeMap::new();
    let mut layers: BTreeMap<u16, Vec<&KcId>> = BTreeMap::new();
    for id in &live {
        layers
            .entry(layout_layer(id, &parents, &mut memo))
            .or_default()
            .push(*id);
    }
    let max_layer = layers.keys().next_back().copied().unwrap_or(0);
    let mut x: BTreeMap<&KcId, u16> = BTreeMap::new();
    for (level, ids) in &layers {
        let mut ids = ids.clone();
        ids.sort_by(|left, right| {
            let avg = |id: &KcId| {
                let values: Vec<u16> = parents
                    .get(id)
                    .into_iter()
                    .flatten()
                    .filter_map(|parent| x.get(parent).copied())
                    .collect();
                if values.is_empty() {
                    0.0
                } else {
                    f64::from(values.iter().map(|v| u32::from(*v)).sum::<u32>())
                        / f64::from(u32::try_from(values.len()).unwrap_or(1))
                }
            };
            avg(left)
                .total_cmp(&avg(right))
                .then_with(|| left.cmp(right))
        });
        let count = ids.len();
        for (index, id) in ids.into_iter().enumerate() {
            let position = if count <= 1 {
                500
            } else {
                u16::try_from(index.saturating_mul(1000) / (count - 1)).unwrap_or(1000)
            };
            x.insert(id, position);
            if let Some(entry) = kcs.iter_mut().find(|entry| &entry.kc.id == id) {
                entry.layout_x = position;
                entry.layout_y = if max_layer == 0 {
                    500
                } else {
                    u16::try_from(u32::from(*level).saturating_mul(1000) / u32::from(max_layer))
                        .unwrap_or(1000)
                };
            }
        }
    }
}

/// The bundle's bytes: compact JSON plus a trailing newline. Deterministic.
///
/// # Panics
/// Never: bundle types always serialize.
pub fn to_bytes(bundle: &Bundle) -> Vec<u8> {
    let mut out = serde_json::to_vec(bundle).expect("bundle serializes");
    out.push(b'\n');
    out
}
