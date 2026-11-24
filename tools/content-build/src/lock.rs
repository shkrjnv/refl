//! `content/item-versions.lock`: the model projection of every published
//! item version, append-only (`docs/02-layer1-content.md`, «Идентификаторы и
//! версии»).
//!
//! The file is one JSON entry per line, sorted by (item, version), followed by
//! `sha256 <hex>` over the entry lines. The digest makes a hand edit visible:
//! an entry removed or changed by hand fails the build instead of silently
//! coming back or going missing.

use std::fmt::Write as _;
use std::path::Path;

use refl_core::ids::{ItemId, ItemVersion};
use refl_core::item::ItemStatus;
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use sha2::{Digest, Sha256};

use crate::check::diag;
use crate::load::Content;
use crate::report::{Code, Diagnostic};

/// Name of the lock file inside the content root.
pub const FILE: &str = "item-versions.lock";

const DIGEST_PREFIX: &str = "sha256 ";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lock {
    /// Sorted by (item, version), without repeats.
    entries: Vec<ItemVersionEntry>,
}

fn digest(lines: &str) -> String {
    Sha256::digest(lines.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

impl Lock {
    pub fn entries(&self) -> &[ItemVersionEntry] {
        &self.entries
    }

    fn find(&self, item: &ItemId, version: ItemVersion) -> Option<&ItemProjection> {
        self.entries
            .binary_search_by(|e| (&e.item, e.version).cmp(&(item, version)))
            .ok()
            .map(|i| &self.entries[i].projection)
    }

    /// Reads the file's text.
    ///
    /// # Errors
    /// Why the file cannot be trusted: unreadable lines, entries out of
    /// order or repeated, a missing or wrong digest.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (body, last) = match text.trim_end_matches('\n').rsplit_once('\n') {
            Some((body, last)) => (format!("{body}\n"), last),
            None => (String::new(), text.trim_end_matches('\n')),
        };
        let Some(expected) = last.strip_prefix(DIGEST_PREFIX) else {
            return Err("the last line is not `sha256 <hex>`".to_owned());
        };
        if digest(&body) != expected {
            return Err(
                "the digest does not match the entries: an entry was removed or changed by hand"
                    .to_owned(),
            );
        }
        let mut entries: Vec<ItemVersionEntry> = Vec::new();
        for (n, line) in body.lines().enumerate() {
            let entry: ItemVersionEntry =
                serde_json::from_str(line).map_err(|e| format!("line {}: {e}", n + 1))?;
            if let Some(prev) = entries.last()
                && (&prev.item, prev.version) >= (&entry.item, entry.version)
            {
                return Err(format!(
                    "line {}: entries are not sorted by (item, version) or repeat",
                    n + 1
                ));
            }
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    /// The file's text.
    ///
    /// # Panics
    /// Never: entries always serialize.
    pub fn render(&self) -> String {
        let mut body = String::new();
        for e in &self.entries {
            body.push_str(&serde_json::to_string(e).expect("entry serializes"));
            body.push('\n');
        }
        let digest = digest(&body);
        format!("{body}{DIGEST_PREFIX}{digest}\n")
    }
}

/// Reads the lock file; a missing file is an empty lock.
///
/// # Errors
/// The file exists but cannot be read.
pub fn read(path: &Path) -> std::io::Result<Result<Lock, String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Lock::parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Ok(Lock::default())),
        Err(e) => Err(e),
    }
}

/// What differs between a locked projection and a fresh one.
fn differences(locked: &ItemProjection, fresh: &ItemProjection) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = |name: &str, same: bool| {
        if !same {
            out.push(name.to_owned());
        }
    };
    field("subject", locked.subject == fresh.subject);
    field("family", locked.family == fresh.family);
    field("kind", locked.kind == fresh.kind);
    field("difficulty", locked.difficulty == fresh.difficulty);
    field("expected_time", locked.expected_time == fresh.expected_time);
    if locked.slots.len() != fresh.slots.len() {
        out.push(format!(
            "slot count {} -> {}",
            locked.slots.len(),
            fresh.slots.len()
        ));
        return out;
    }
    for (i, (a, b)) in locked.slots.iter().zip(&fresh.slots).enumerate() {
        if a.kcs != b.kcs {
            out.push(format!("slots[{i}].kcs"));
        }
        if a.choices != b.choices {
            out.push(format!("slots[{i}].choices"));
        }
        if a.misconceptions != b.misconceptions {
            out.push(format!("slots[{i}].misconceptions"));
        }
    }
    out
}

/// The lock after this build: `old` plus an entry for every published item
/// version it lacks. An item whose version has an entry must still match it;
/// otherwise `projection-changed`. Items removed from the content keep their
/// entries.
pub fn reconcile(content: &Content, old: &Lock, diags: &mut Vec<Diagnostic>) -> Lock {
    let mut entries = old.entries.clone();
    for it in &content.items {
        let item = &it.value;
        let fresh = ItemProjection::of(item);
        match old.find(&item.id, item.version) {
            Some(locked) if *locked == fresh => {}
            Some(locked) => diags.push(diag(
                Code::ProjectionChanged,
                &it.file,
                format!("item `{}`", item.id),
                format!(
                    "the model projection of version {} changed since it was published ({}); \
                     bump the version",
                    item.version.0,
                    differences(locked, &fresh).join(", ")
                ),
            )),
            None if matches!(item.status, ItemStatus::Live | ItemStatus::Retired) => {
                entries.push(ItemVersionEntry {
                    item: item.id.clone(),
                    version: item.version,
                    projection: fresh,
                });
            }
            None => {}
        }
    }
    entries.sort_by(|a, b| (&a.item, a.version).cmp(&(&b.item, b.version)));
    entries.dedup_by(|b, a| (&a.item, a.version) == (&b.item, b.version));
    Lock { entries }
}
