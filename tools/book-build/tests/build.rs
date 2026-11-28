//! End to end: the real content and the starter reader compile to stable
//! bytes, their tokens link to the lexical bank, and a missing license record
//! stops the build (`docs/16-vocabulary-library-notes-graph.md`, §2).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use book_build::{Options, build, write};
use refl_core::book::Book;
use refl_core::ids::LexemeId;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn unique(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("refl-book-build-{}-{n}-{tag}", std::process::id()))
}

fn content_bundle(dir: &Path, tag: &str) -> PathBuf {
    let out = content_build::build(&content_build::Options::new(dir)).unwrap();
    assert!(!out.report.has_errors(), "{}", out.report);
    let bundle = out.bundle.expect("bundle");
    let path = unique(tag).with_extension("json");
    std::fs::write(&path, content_build::to_bytes(&bundle)).unwrap();
    path
}

fn options(bundle: &Path, tag: &str) -> Options {
    let root = repo();
    let mut opts = Options::new(root.join("content/books"));
    opts.bundle = bundle.to_path_buf();
    opts.third_party = root.join("THIRD_PARTY.md");
    opts.content_license = root.join("content/LICENSE");
    opts.out = unique(tag).with_extension("out");
    opts
}

#[test]
fn the_starter_reader_is_deterministic_and_linked() {
    let bundle_path = content_bundle(&repo().join("content"), "linked");
    let bundle: refl_core::bundle::Bundle =
        serde_json::from_slice(&std::fs::read(&bundle_path).unwrap()).unwrap();
    let senses: BTreeSet<LexemeId> = bundle
        .content
        .lexemes
        .iter()
        .flat_map(|lexeme| lexeme.senses.iter().map(|sense| sense.id.clone()))
        .collect();
    let opts = options(&bundle_path, "linked");
    let (first, diagnostics) = build(&opts).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(first.len(), 1, "the starter A1 reader");
    let compiled = &first[0];
    assert_eq!(compiled.book.id.as_str(), "book.first-day");
    assert_eq!(compiled.book.level.as_str(), "A1");
    assert_eq!(compiled.book.version.len(), 64);
    assert!(compiled.book.version.bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(compiled.book.chapters.len() >= 2);
    let mut linked = 0;
    for chapter in &compiled.book.chapters {
        assert!(chapter.id.starts_with("chapter.first-day."));
        for paragraph in &chapter.paragraphs {
            assert!(paragraph.id.starts_with("para.first-day."));
            for sentence in &paragraph.sentences {
                assert!(sentence.id.starts_with("sentence.first-day."));
                for token in &sentence.tokens {
                    assert!(token.id.starts_with("token.first-day."));
                    if let Some(lexeme) = &token.lexeme_id {
                        linked += 1;
                        assert!(senses.contains(lexeme), "{}", lexeme);
                    }
                    for lexeme in &token.sense_ids {
                        assert!(senses.contains(lexeme), "{}", lexeme);
                    }
                }
            }
        }
    }
    assert!(linked > 0, "the reader links words to the starter bank");
    // Rebuilding the same sources gives the same bytes and version.
    let (second, diagnostics) = build(&opts).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(second[0].bytes, compiled.bytes);
    assert_eq!(second[0].book.version, compiled.book.version);
    // The asset round-trips as a Book with the same version.
    let parsed: Book = serde_json::from_slice(&compiled.bytes).unwrap();
    assert_eq!(parsed, compiled.book);
    write(&opts, &first).unwrap();
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(opts.out.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["books"][0]["id"], serde_json::json!("book.first-day"));
    assert_eq!(
        index["books"][0]["version"],
        serde_json::json!(compiled.book.version)
    );
    let _ = std::fs::remove_file(&bundle_path);
}

#[test]
fn a_book_without_a_license_record_fails() {
    let bundle_path = content_bundle(&repo().join("content"), "license");
    let opts = options(&bundle_path, "license");
    let empty = unique("empty-third-party").with_extension("md");
    std::fs::write(&empty, "# no records\n").unwrap();
    let mut opts = opts;
    opts.third_party = empty.clone();
    let (compiled, diagnostics) = build(&opts).unwrap();
    assert!(compiled.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("THIRD_PARTY.md")),
        "{diagnostics:?}"
    );
    let _ = std::fs::remove_file(&empty);
    let _ = std::fs::remove_file(&bundle_path);
}
