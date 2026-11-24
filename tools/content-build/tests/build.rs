//! content-build end to end: the example content builds and is
//! byte-deterministic, and every check catches its own defect in a negative
//! fixture (`tests/fixtures/<code>/`, built on `tests/fixtures/lib/base.ncl`).
//! Needs `nickel` on PATH (the dev shell and `nix flake check` provide it).

use std::path::{Path, PathBuf};

use content_build::lock::Lock;
use content_build::report::{Code, Severity};
use content_build::{Options, Output, build, to_bytes};
use refl_core::bundle::KcStatus;
use refl_core::ids::{ItemId, ItemRef, ItemVersion};
use refl_core::item::ItemStatus;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Options for a fixture. The fixtures' reviewed records are written by hand
/// without the hash of their text, so the hash check is off unless the
/// fixture opts in with a `.check-hashes` file.
fn options(dir: &Path) -> Options {
    let mut opts = Options::new(dir);
    opts.check_text_hashes = dir.join(".check-hashes").exists();
    opts
}

fn run(dir: &Path) -> Output {
    build(&options(dir)).unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
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
        // Only `lock-outdated` is about `--locked`; the other fixtures have
        // no lock or a matching one.
        let mut opts = options(&dir);
        opts.locked = code == Code::LockOutdated;
        let out = build(&opts).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
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
    assert!(report.contains("unknown subject `xx`"), "{report}");
    for kc in ["en.k.missing", "en.k.missing-gap", "en.k.missing-choice"] {
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
fn contrast_crossed_by_a_confusion_passes() {
    let out = run(&fixtures().join("contrast-with-confusion"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    let bundle = out.bundle.expect("bundle");
    let status = |id: &str| {
        bundle
            .content
            .kcs
            .iter()
            .find(|k| k.kc.id.as_str() == id)
            .map(|k| k.status)
    };
    assert_eq!(status("en.k.a"), Some(KcStatus::Live));
    assert_eq!(status("en.k.b"), Some(KcStatus::Live));
    assert_eq!(status("en.k.c"), Some(KcStatus::Draft));
}

/// A KC with everything written but not reviewed is draft, and the report
/// names each missing review (docs/02, «Вычитка и показ»).
#[test]
fn unreviewed_texts_keep_a_kc_draft() {
    let out = run(&fixtures().join("unreviewed"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    assert_eq!(out.report.live, Vec::<String>::new());
    let missing = &out
        .report
        .draft
        .iter()
        .find(|d| d.kc == "en.k.a")
        .unwrap()
        .missing;
    for expected in [
        "short explanation in ru not reviewed",
        "instruction `i.gap` not reviewed in en",
        "micro-explanation of `en.m.one` in en not reviewed",
    ] {
        assert!(
            missing.iter().any(|m| m == expected),
            "{expected}: {missing:?}"
        );
    }
    // Nothing else blocks it: no item count or family complaint.
    assert_eq!(missing.len(), 3, "{missing:?}");
    let bundle = out.bundle.expect("bundle");
    assert!(bundle.content.items.iter().all(|e| !e.session_eligible));
}

#[test]
fn draft_unreviewed_items_and_items_on_draft_kcs_are_not_eligible() {
    let out = run(&fixtures().join("eligibility"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    let bundle = out.bundle.expect("bundle");
    for entry in &bundle.content.items {
        let id = entry.item.id.as_str();
        let expected = !matches!(
            id,
            "it.draft" | "it.on-draft-kc" | "it.half-reviewed" | "it.unreviewed"
        );
        assert_eq!(entry.session_eligible, expected, "{id}");
    }
}

/// A `contrasts` edge nobody measures blocks both ends (docs/02), whatever
/// else they have; the build still passes.
#[test]
fn an_uncrossed_contrast_blocks_both_kcs() {
    let out = run(&fixtures().join("contrast-blocker"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    assert_eq!(out.report.live, Vec::<String>::new());
    for (kc, other) in [("en.k.a", "en.k.b"), ("en.k.b", "en.k.a")] {
        let missing = &out
            .report
            .draft
            .iter()
            .find(|d| d.kc == kc)
            .unwrap()
            .missing;
        assert_eq!(
            missing,
            &[format!(
                "contrast with `{other}` has no task marked `confuses_with`"
            )]
        );
    }
}

/// The live conditions count tasks a learner can be given: ten live tasks of
/// which one is unreviewed are nine.
#[test]
fn live_counts_only_reviewed_tasks() {
    let out = run(&fixtures().join("unreviewed-task"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    let missing = &out
        .report
        .draft
        .iter()
        .find(|d| d.kc == "en.k.a")
        .unwrap()
        .missing;
    assert_eq!(
        missing,
        &["9 reviewed live task(s) of 10 live, needs 10".to_owned()]
    );
    // The same ten, all reviewed, make the KC live (the clean fixture does).
    let clean = run(&fixtures().join("contrast-with-confusion"));
    assert!(clean.report.live.contains(&"en.k.a".to_owned()));
}

#[test]
fn an_unreviewed_title_keeps_a_kc_draft() {
    let out = run(&fixtures().join("title-unreviewed"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    let missing = &out
        .report
        .draft
        .iter()
        .find(|d| d.kc == "en.k.a")
        .unwrap()
        .missing;
    assert_eq!(missing, &["title in en not reviewed".to_owned()]);
}

/// The review queue (`report --review`): by language, `path:line — what`,
/// and every task as a learner sees it.
#[test]
fn the_review_queue_lists_drafts_by_language_with_a_line() {
    let root = fixtures().join("eligibility");
    let (out, content) = content_build::build_with_content(&options(&root)).unwrap();
    assert!(out.bundle.is_some());
    let files = content_build::review::review_files(&root, &content);
    let sections: Vec<&str> = files
        .queue
        .lines()
        .filter(|l| l.starts_with("== "))
        .collect();
    assert_eq!(sections, ["== uz", "== ru", "== en"]);
    // The unreviewed task is listed in ru and en (it has texts there), once
    // per language, as `path:line — …` with a real line of the file.
    let ru = files
        .queue
        .split("== ru")
        .nth(1)
        .unwrap()
        .split("== en")
        .next()
        .unwrap();
    let entry = ru
        .lines()
        .find(|l| l.starts_with("it.unreviewed#why,hint — "))
        .unwrap_or_else(|| panic!("{ru}"));
    // The key comes first; the place is a hint at the end.
    let (key, rest) = entry.split_once(" — ").unwrap();
    assert_eq!(key, "it.unreviewed#why,hint");
    assert!(rest.starts_with("task v1: why and hint"), "{rest}");
    let place = rest
        .rsplit_once("(≈ ")
        .unwrap()
        .1
        .strip_suffix(')')
        .unwrap();
    let (path, line) = place.rsplit_once(':').unwrap();
    assert!(path.ends_with("content.ncl"), "{path}");
    assert!(line.parse::<usize>().unwrap() >= 1);
    // A reviewed task is not in the queue.
    assert!(!ru.contains("it.gap#"), "{ru}");
    // The tasks file has a block per language with the review status.
    assert!(files.tasks.contains("## it.unreviewed v1 · gapfill"));
    assert!(files.tasks.contains("### ru [draft]"));
    assert!(files.tasks.contains("### ru [reviewed by test]"));
    assert!(files.tasks.contains("- Answer: a cat"));
}

/// The line is found from the text, also when the text came from a template.
#[test]
fn the_queue_finds_the_nearest_line_of_templated_text() {
    let root = repo().join("content");
    let (_, content) = content_build::build_with_content(&options(&root)).unwrap();
    let files = content_build::review::review_files(&root, &content);
    // The Articles tasks are one shared review: one hash per language.
    for lang in ["uz", "ru", "en"] {
        let line = files
            .hashes
            .lines()
            .find(|l| l.starts_with(&format!("group:articles\t{lang}\t")))
            .unwrap_or_else(|| panic!("{}", files.hashes));
        assert_eq!(line.rsplit('\t').next().unwrap().len(), 64);
    }
    // `*apple* здесь упоминается впервые…` is built from a template in items.ncl.
    let line = files
        .queue
        .lines()
        .find(|l| l.starts_with("art.fm.choice.dog#why,hint — ") && l.contains("items.ncl"))
        .unwrap();
    let source = std::fs::read_to_string(root.join("articles/items.ncl")).unwrap();
    let place = line
        .rsplit_once("(≈ ")
        .unwrap()
        .1
        .strip_suffix(')')
        .unwrap();
    let n: usize = place.rsplit(':').next().unwrap().parse().unwrap();
    assert!(
        source.lines().nth(n - 1).unwrap().contains("впервые")
            || source.lines().nth(n - 1).unwrap().contains("birinchi")
            || source.lines().nth(n - 1).unwrap().contains("first time"),
        "line {n}: {}",
        source.lines().nth(n - 1).unwrap()
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
    // The Articles unit of the prototype (docs/15, «Состав темы») is complete
    // but draft until a person has reviewed its texts (docs/02, «Вычитка и
    // показ»): the only thing that keeps a KC from live is review.
    for kc in [
        "en.gram.noun.countability",
        "en.gram.article.a-an-sound",
        "en.gram.article.first-mention",
        "en.gram.article.second-mention",
        "en.gram.article.zero-general",
    ] {
        assert_eq!(status(kc), Some(KcStatus::Draft), "{kc}");
        let missing = &out
            .report
            .draft
            .iter()
            .find(|d| d.kc == kc)
            .unwrap()
            .missing;
        // Everything missing is about review. The bank counts only reviewed
        // live tasks, so with none reviewed it is reported as such.
        assert!(
            missing.iter().all(|m| m.contains("reviewed")),
            "{kc}: {missing:?}"
        );
        assert!(
            missing
                .iter()
                .any(|m| m.starts_with("0 reviewed live task(s) of ") && m.ends_with(", needs 10")),
            "{kc}: {missing:?}"
        );
    }
    // Items on draft KCs are not given in sessions.
    for entry in &bundle.content.items {
        assert!(!entry.session_eligible, "{}", entry.item.id);
    }
    assert_eq!(
        bundle.content.requires_closure
            [&refl_core::ids::KcId::new("en.gram.article.second-mention").unwrap()]
            .iter()
            .map(|r| (r.kc.as_str(), r.distance))
            .collect::<Vec<_>>(),
        [
            ("en.gram.article.first-mention", 1),
            ("en.gram.noun.countability", 2)
        ]
    );
}

/// The committed lock holds every published item version of `content/`; CI
/// builds with `--locked`.
#[test]
fn committed_lock_is_up_to_date() {
    let mut opts = Options::new(repo().join("content"));
    opts.locked = true;
    let out = build(&opts).expect("build");
    assert!(!out.report.has_errors(), "{}", out.report);
    assert_eq!(out.lock_update, None);
    let bundle = out.bundle.expect("bundle");
    let published: Vec<_> = bundle
        .content
        .items
        .iter()
        .filter(|e| matches!(e.item.status, ItemStatus::Live | ItemStatus::Retired))
        .collect();
    // Every published item has its projection; superseded versions add more.
    for e in &published {
        assert!(
            bundle
                .content
                .item_versions
                .iter()
                .any(|v| v.item == e.item.id && v.version == e.item.version),
            "{}",
            e.item.id
        );
    }
    assert!(bundle.content.item_versions.len() >= published.len());
}

/// Bumping a version together with a projection change passes; the bundle
/// then holds the new version's body and both versions' projections.
#[test]
fn bumped_version_keeps_the_old_projection_in_the_bundle() {
    let out = run(&fixtures().join("version-bump"));
    assert!(out.report.diagnostics.is_empty(), "{}", out.report);
    assert_eq!(
        out.lock_update.as_deref().map(|t| t.lines().count()),
        Some(4)
    );
    let bundle = out.bundle.expect("bundle");
    let versions: Vec<_> = bundle
        .content
        .item_versions
        .iter()
        .map(|e| (e.item.as_str(), e.version.0, e.projection.difficulty.get()))
        .collect();
    assert_eq!(
        versions,
        [("it.choice", 1, 1), ("it.gap", 1, 1), ("it.gap", 2, 2)]
    );
    let bodies: Vec<_> = bundle
        .content
        .items
        .iter()
        .map(|e| (e.item.id.as_str(), e.item.version.0))
        .collect();
    assert_eq!(
        bodies,
        [("it.choice", 1), ("it.gap", 2)],
        "bodies: current only"
    );
    let old = ItemRef {
        id: ItemId::new("it.gap").unwrap(),
        version: ItemVersion(1),
        seed: None,
    };
    assert_eq!(bundle.projection(&old).unwrap().difficulty.get(), 1);
}

#[test]
fn projection_change_names_what_changed() {
    let out = run(&fixtures().join("projection-changed"));
    let msg = &out.report.diagnostics[0].message;
    assert!(msg.contains("difficulty") && msg.contains("bump"), "{msg}");
    assert_eq!(out.lock_update, None, "an erroneous build writes nothing");
}

/// The lock only grows: entries survive a rebuild, an unchanged lock is not
/// rewritten, and an entry lost from either end is caught by the digest.
#[test]
fn lock_is_append_only_and_tamper_evident() {
    let clean = run(&fixtures().join("clean"));
    let text = clean.lock_update.expect("a new lock");
    assert_eq!(text.lines().count(), 3, "two entries and the digest");
    let lock = Lock::parse(&text).expect("a written lock parses");
    assert_eq!(lock.render(), text);

    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lock-append-only");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("item-versions.lock");
    let mut opts = options(&fixtures().join("clean"));
    opts.lock = path.clone();
    std::fs::write(&path, &text).unwrap();
    let again = build(&opts).expect("build");
    assert!(again.report.diagnostics.is_empty(), "{}", again.report);
    assert_eq!(again.lock_update, None, "nothing new, nothing rewritten");

    let lines: Vec<&str> = text.lines().collect();
    for (what, tampered) in [
        ("first entry", [lines[1], lines[2]].join("\n")),
        ("last entry", [lines[0], lines[2]].join("\n")),
        ("all entries", format!("{}\n", lines[2])),
    ] {
        std::fs::write(&path, format!("{tampered}\n")).unwrap();
        let out = build(&opts).expect("build");
        let codes: Vec<Code> = out.report.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [Code::LockCorrupt], "{what} removed");
        assert!(out.bundle.is_none(), "{what} removed");
    }
}

#[test]
fn bundle_is_byte_identical_across_builds() {
    let a = run(&repo().join("content")).bundle.expect("bundle");
    let b = run(&repo().join("content")).bundle.expect("bundle");
    assert_eq!(to_bytes(&a), to_bytes(&b));
    let parsed: refl_core::bundle::Bundle = serde_json::from_slice(&to_bytes(&a)).unwrap();
    assert_eq!(to_bytes(&parsed), to_bytes(&a), "bundle round-trips");
}

#[test]
fn subject_prefix_covers_kcs_misconceptions_and_families() {
    let out = run(&fixtures().join("subject-prefix"));
    let subjects: Vec<&str> = out
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::SubjectPrefix)
        .filter_map(|d| d.subject.as_deref())
        .collect();
    assert_eq!(
        subjects,
        ["KC `k.z`", "misconception `m.z`", "item `it.z`"],
        "{}",
        out.report
    );
}

#[test]
fn only_requires_may_not_cross_subjects() {
    let out = run(&fixtures().join("requires-across-subjects"));
    let d = &out.report.diagnostics;
    assert_eq!(d.len(), 1, "{}", out.report);
    assert_eq!(
        d[0].subject.as_deref(),
        Some("edge `en.k.a` requires `math.k.x`")
    );
}

#[test]
fn kcs_stay_within_their_subject() {
    let out = run(&fixtures().join("kc-outside-subject"));
    let subjects: Vec<&str> = out
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::KcOutsideSubject)
        .filter_map(|d| d.subject.as_deref())
        .collect();
    assert_eq!(
        subjects,
        ["subject `math`", "item `it.gap`"],
        "{}",
        out.report
    );
}

#[test]
fn items_carry_only_the_fields_of_their_kind() {
    let typed = run(&fixtures().join("kind-fields-typed"));
    assert!(typed.report.diagnostics.is_empty(), "{}", typed.report);
    let choice = run(&fixtures().join("kind-fields"));
    let d = &choice.report.diagnostics;
    assert_eq!(d.len(), 1, "{}", choice.report);
    assert_eq!(d[0].code, Code::Nickel);
    assert!(
        d[0].message.contains("extra field `case_sensitive`"),
        "{}",
        d[0].message
    );
}

/// A review is bound to its text (docs/02, «Вычитка и показ»): editing a
/// reviewed text is an error until the new hash is set.
mod review_hash {
    use std::path::PathBuf;

    use content_build::report::Code;
    use content_build::{Options, build, build_with_content};

    use super::fixtures;

    /// A tiny content root whose only reviewable record is one en
    /// explanation, with the given body and review.
    struct Root(PathBuf);

    impl Root {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("refl-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, body: &str, review: &str) {
            let base = fixtures().join("lib/base.ncl");
            let content = format!(
                r#"let b = import "{}" in
let undone = fun r => std.record.update "review" {{}} r in
{{
  subjects = b.subjects,
  kcs = std.array.map undone b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = std.array.map undone b.instructions,
  explanations = [
    {{
      id = "e.k.a.en",
      target = {{ kind = "kc", id = "en.k.a" }},
      depth = "short",
      lang = "en",
      body = "{body}",
      review = {review},
    }},
  ],
}}
"#,
                base.display()
            );
            std::fs::write(self.0.join("content.ncl"), content).unwrap();
        }

        fn codes(&self) -> Vec<(Code, String)> {
            let out = build(&Options::new(&self.0)).unwrap();
            out.report
                .diagnostics
                .iter()
                .map(|d| (d.code, d.message.clone()))
                .collect()
        }

        /// What the explanation's hash is now (the check is off to read it).
        fn hash(&self) -> String {
            let mut opts = Options::new(&self.0);
            opts.check_text_hashes = false;
            let (_, content) = build_with_content(&opts).unwrap();
            content_build::review::covered(&content)
                .into_iter()
                .find(|c| c.key == "explanation:e.k.a.en")
                .unwrap()
                .hash
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn reviewed(hash: &str) -> String {
        format!(r#"{{ status = "reviewed", by = "Hara", hash = "{hash}" }}"#)
    }

    #[test]
    fn editing_a_reviewed_text_is_an_error_and_marking_again_passes() {
        let root = Root::new("hash");
        // Reviewed by a person but with no hash: the text is not pinned.
        root.write("A cat.", r#"{ status = "reviewed", by = "Hara" }"#);
        let codes = root.codes();
        assert_eq!(codes.len(), 1, "{codes:?}");
        assert_eq!(codes[0].0, Code::ReviewedTextChanged);
        assert!(codes[0].1.contains("without a `hash`"), "{}", codes[0].1);

        // Marked: the hash of the text as it stands.
        let marked = root.hash();
        root.write("A cat.", &reviewed(&marked));
        assert_eq!(root.codes(), []);

        // Edited after the review: an error naming the record and language.
        root.write("A cat!", &reviewed(&marked));
        let codes = root.codes();
        assert_eq!(codes.len(), 1, "{codes:?}");
        assert_eq!(codes[0].0, Code::ReviewedTextChanged);
        assert!(codes[0].1.contains("language en"), "{}", codes[0].1);
        assert!(codes[0].1.contains(&marked), "{}", codes[0].1);

        // Re-read and marked again with the new hash: passes.
        let again = root.hash();
        assert_ne!(again, marked);
        root.write("A cat!", &reviewed(&again));
        assert_eq!(root.codes(), []);
    }

    #[test]
    fn a_draft_needs_no_hash_and_may_change_freely() {
        let root = Root::new("draft");
        root.write("One.", r#"{ status = "draft" }"#);
        assert_eq!(root.codes(), []);
        root.write("Two.", r#"{ status = "draft" }"#);
        assert_eq!(root.codes(), []);
    }
}
