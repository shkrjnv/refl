//! TypeScript bindings for the web client are generated from the Rust types
//! and committed under `web/src/generated/core/`. This test fails when they
//! are stale. Regenerate with:
//!
//!     UPDATE_TS_BINDINGS=1 cargo test -p refl-core --all-features --test ts_bindings

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use refl_core::Event;
use refl_core::content::{Edge, Explanation, Instruction, Kc, Misconception};
use refl_core::item::Item;
use refl_core::lesson::Lesson;
use refl_core::subject::Subject;
use ts_rs::{Config, TS};

fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/src/generated/core")
}

fn export_all(dir: &Path) {
    let cfg = Config::new().with_out_dir(dir);
    Event::export_all(&cfg).unwrap();
    Item::export_all(&cfg).unwrap();
    Kc::export_all(&cfg).unwrap();
    Edge::export_all(&cfg).unwrap();
    Misconception::export_all(&cfg).unwrap();
    Explanation::export_all(&cfg).unwrap();
    Instruction::export_all(&cfg).unwrap();
    Lesson::export_all(&cfg).unwrap();
    Subject::export_all(&cfg).unwrap();
}

fn read_tree(dir: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                out.insert(rel, fs::read_to_string(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// No integer in an event may exceed 2^53 - 1, the largest JavaScript holds
/// exactly (`docs/02-layer1-content.md`, «Событие и лог»). ts-rs writes
/// `bigint` for 64-bit integers, so `Event` and every type it reaches must
/// be free of it.
#[test]
fn event_types_have_no_bigint() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ts-event-types");
    let _ = fs::remove_dir_all(&dir);
    Event::export_all(&Config::new().with_out_dir(&dir)).unwrap();
    let types = read_tree(&dir);
    assert!(types.len() > 10, "Event exported too few types: {types:?}");
    let bigint: Vec<_> = types
        .iter()
        .filter(|(_, text)| text.contains("bigint"))
        .map(|(path, _)| path)
        .collect();
    assert!(
        bigint.is_empty(),
        "event types with bigint fields: {bigint:?}; use Seed or another 53-bit type"
    );
}

#[test]
fn ts_bindings_are_up_to_date() {
    let fresh = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ts-bindings");
    let _ = fs::remove_dir_all(&fresh);
    export_all(&fresh);
    let generated = read_tree(&fresh);
    assert!(!generated.is_empty(), "no bindings were generated");

    let committed = committed_dir();
    if std::env::var_os("UPDATE_TS_BINDINGS").is_some() {
        let _ = fs::remove_dir_all(&committed);
        for (rel, text) in &generated {
            let path = committed.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        return;
    }
    let on_disk = read_tree(&committed);
    let stale: Vec<_> = generated
        .keys()
        .chain(on_disk.keys())
        .filter(|k| generated.get(*k) != on_disk.get(*k))
        .collect();
    assert!(
        stale.is_empty(),
        "web/src/generated/core is stale ({stale:?}); run \
         UPDATE_TS_BINDINGS=1 cargo test -p refl-core --all-features --test ts_bindings"
    );
}
