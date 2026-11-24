//! content-build end to end: the example content builds and is
//! byte-deterministic, and every check catches its own defect in a negative
//! fixture (`tests/fixtures/<code>/`, built on `tests/fixtures/lib/base.ncl`).
//! Needs `nickel` on PATH (the dev shell and `nix flake check` provide it).

use std::path::{Path, PathBuf};

use content_build::report::{Code, Severity};
use content_build::{Options, Output, build, to_bytes};
use refl_core::bundle::KcStatus;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(dir: &Path) -> Output {
    build(&Options::new(dir)).unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn clean_fixture_has_no_diagnostics() {
    let out = run(&fixtures().join("clean"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    assert!(out.bundle.is_some());
}

#[test]
fn each_check_catches_its_own_defect() {
    let mut failures = Vec::new();
    for code in Code::ALL {
        let dir = fixtures().join(code.as_str());
        let out = run(&dir);
        let codes: Vec<Code> = out.report.diagnostics.iter().map(|d| d.code).collect();
        if codes.is_empty() || codes.iter().any(|&c| c != code) {
            failures.push(format!(
                "{}: expected only {code:?}, got\n{}",
                dir.display(),
                out.report
            ));
            continue;
        }
        let d = &out.report.diagnostics[0];
        if d.file.as_os_str().is_empty() || d.message.is_empty() {
            failures.push(format!(
                "{}: diagnostic lacks file or message: {d}",
                dir.display()
            ));
        }
        let fails = d.severity == Severity::Error;
        if fails == out.bundle.is_some() {
            failures.push(format!(
                "{}: severity {:?} but bundle present = {}",
                dir.display(),
                d.severity,
                out.bundle.is_some()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn unknown_reference_covers_confusions() {
    let out = run(&fixtures().join("unknown-reference"));
    let report = out.report.to_string();
    for kc in ["k.missing", "k.missing-gap", "k.missing-choice"] {
        assert!(
            report.contains(&format!("unknown KC `{kc}`")),
            "{kc}:\n{report}"
        );
    }
}

#[test]
fn witnesses_are_shortest_examples() {
    let out = run(&fixtures().join("accepted-matches-error"));
    let msg = &out.report.diagnostics[0].message;
    assert!(msg.contains("both match \"a cat\""), "{msg}");
    let out = run(&fixtures().join("error-patterns-overlap"));
    let msg = &out.report.diagnostics[0].message;
    assert!(
        msg.contains("\"cat\"") && msg.contains("first one wins"),
        "{msg}"
    );
}

#[test]
fn example_content_builds_with_computed_status() {
    let out = run(&repo().join("content"));
    assert!(!out.report.has_errors(), "{}", out.report);
    let bundle = out.bundle.expect("bundle");
    let status = |id: &str| {
        bundle
            .content
            .kcs
            .iter()
            .find(|k| k.kc.id.as_str() == id)
            .map(|k| k.status)
    };
    assert_eq!(status("gram.article.first-mention"), Some(KcStatus::Live));
    assert_eq!(status("gram.article.second-mention"), Some(KcStatus::Draft));
    assert_eq!(status("gram.noun.countability"), Some(KcStatus::Draft));
    // Draft items are not eligible; neither are items whose primary KC is draft.
    for entry in &bundle.content.items {
        let primary_draft = entry.item.id.as_str().starts_with("art.sm.");
        assert_eq!(entry.session_eligible, !primary_draft, "{}", entry.item.id);
    }
    assert_eq!(
        bundle.content.requires_closure
            [&refl_core::ids::KcId::new("gram.article.second-mention").unwrap()]
            .iter()
            .map(refl_core::ids::KcId::as_str)
            .collect::<Vec<_>>(),
        ["gram.article.first-mention", "gram.noun.countability"]
    );
}

#[test]
fn bundle_is_byte_identical_across_builds() {
    let a = run(&repo().join("content")).bundle.expect("bundle");
    let b = run(&repo().join("content")).bundle.expect("bundle");
    assert_eq!(to_bytes(&a), to_bytes(&b));
    let parsed: refl_core::bundle::Bundle = serde_json::from_slice(&to_bytes(&a)).unwrap();
    assert_eq!(to_bytes(&parsed), to_bytes(&a), "bundle round-trips");
}
