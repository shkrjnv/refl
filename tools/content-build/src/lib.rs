//! content-build: compiles `content/` (Nickel) into one deterministic JSON
//! bundle and runs every check from `docs/02-layer1-content.md`, «Проверки
//! при сборке контента». Errors fail the build; warnings and KCs that stay
//! draft only go to the report.

pub mod check;
pub mod graph;
pub mod load;
pub mod report;
pub mod status;
pub mod subjects;
pub mod templates;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use refl_core::answer::normalize::NORMALIZATION_VERSION;
use refl_core::bundle::{
    BUNDLE_FORMAT_VERSION, Bundle, BundleContent, ItemEntry, KcEntry, KcItems, KcStatus,
};
use refl_core::ids::{ContentVersion, ItemRef, KcId};
use refl_core::item::{ItemStatus, KcRole};
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
}

impl Options {
    /// Audio under `<content>/audio`, `nickel` from `PATH`.
    pub fn new(content: impl Into<PathBuf>) -> Self {
        let content = content.into();
        Self {
            audio_root: content.join("audio"),
            content,
            nickel: PathBuf::from("nickel"),
        }
    }
}

#[derive(Debug)]
pub struct Output {
    pub report: Report,
    /// `None` when the build has errors.
    pub bundle: Option<Bundle>,
}

/// Loads, checks and compiles the content.
///
/// # Errors
/// The content root cannot be read. Content problems are reported in
/// [`Output::report`], not as errors.
///
/// # Panics
/// Never: bundle types always serialize.
pub fn build(opts: &Options) -> std::io::Result<Output> {
    let (content, mut diags) = load::load(&opts.content, &opts.nickel)?;
    let ids = check::duplicates(&content, &mut diags);
    check::references(&content, &ids, &mut diags);
    subjects::check(&content, &mut diags);
    subjects::prefixes(&content, &mut diags);
    subjects::kc_subjects(&content, &mut diags);
    check::misconception_feedback(&content, &mut diags);
    check::instruction_text(&content, &mut diags);
    let requires = graph::Requires::new(&content, &ids);
    requires.check_acyclic(&mut diags);
    let compiled = templates::compile(&content.items, &mut diags);
    check::live_items(&content, &compiled, &mut diags);
    check::audio(&content, &opts.audio_root, &mut diags);

    let mut report = Report {
        diagnostics: diags,
        ..Report::default()
    };
    let status = status::compute(&content, &mut report);
    check::contrasts(&content, &status, &mut report.diagnostics);
    if report.has_errors() {
        return Ok(Output {
            report,
            bundle: None,
        });
    }

    let kcs = kc_entries(&content, &status);

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
        let session_eligible = item.status == ItemStatus::Live && primary_live;
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

    let mut edges: Vec<_> = content.edges.iter().map(|e| e.value.clone()).collect();
    edges.sort();
    let body = BundleContent {
        normalization_version: NORMALIZATION_VERSION,
        subjects: sorted(&content.subjects, |s| &s.id),
        kcs,
        edges,
        requires_closure: requires.closure(&ids),
        misconceptions: sorted(&content.misconceptions, |m| &m.id),
        explanations: sorted(&content.explanations, |e| &e.id),
        instructions: sorted(&content.instructions, |i| &i.id),
        lessons: sorted(&content.lessons, |l| &l.id),
        items,
        items_by_kc,
        tokens: compiled.tokens,
    };
    let hash = Sha256::digest(serde_json::to_vec(&body).expect("bundle serializes"));
    let content_version = ContentVersion(hash.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    }));
    Ok(Output {
        report,
        bundle: Some(Bundle {
            format_version: BUNDLE_FORMAT_VERSION,
            content_version,
            content: body,
        }),
    })
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

/// The bundle's bytes: compact JSON plus a trailing newline. Deterministic.
///
/// # Panics
/// Never: bundle types always serialize.
pub fn to_bytes(bundle: &Bundle) -> Vec<u8> {
    let mut out = serde_json::to_vec(bundle).expect("bundle serializes");
    out.push(b'\n');
    out
}
