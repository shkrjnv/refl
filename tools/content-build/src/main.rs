//! `content-build <content-dir> [--out <bundle.json>] [--locked] [--audio-root <dir>] [--nickel <path>] [--scowl <dir>]`
//!
//! With `--out`, the lexicon the bundle references is written next to it as
//! `lexicon-<version>.txt`. SCOWL defaults to `$REFL_SCOWL_DICT`.
//!
//! New published item versions are appended to `<content-dir>/item-versions.lock`;
//! commit the file. With `--locked` the file is never written, and a build that
//! would append to it fails (CI).
//!
//! Prints the report. Exit status: 0 on success, 1 if the content has errors,
//! 2 on usage or I/O problems.
//!
//! `content-build report [--missing] [--review] [--out <dir>] <content-dir>`
//! prints what is missing before live, and writes the review queue and the
//! tasks as a learner sees them to `<dir>` (default `target/review`).

use std::path::PathBuf;
use std::process::ExitCode;

use content_build::{Options, build, build_with_content, to_bytes};

const USAGE: &str = "usage: content-build <content-dir> [--out <bundle.json>] [--locked] [--audio-root <dir>] [--nickel <path>] [--scowl <dir>]\n       content-build report [--missing] [--review] [--out <dir>] <content-dir>\n       content-build mark-reviewed --by <name> --lang <ru|en|uz> [--content <dir>] (--all | <record>...)";

fn parse_args() -> Result<(Options, Option<PathBuf>), String> {
    let mut args = std::env::args_os().skip(1);
    let mut content = None;
    let mut out = None;
    let mut audio_root = None;
    let mut nickel = None;
    let mut scowl = None;
    let mut locked = false;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.to_str() {
            Some("--out") => out = Some(value("--out")?),
            Some("--audio-root") => audio_root = Some(value("--audio-root")?),
            Some("--nickel") => nickel = Some(value("--nickel")?),
            Some("--scowl") => scowl = Some(value("--scowl")?),
            Some("--locked") => locked = true,
            Some("-h" | "--help") => return Err(String::new()),
            Some(s) if s.starts_with('-') => return Err(format!("unknown option {s}")),
            _ if content.is_none() => content = Some(PathBuf::from(arg)),
            _ => return Err("more than one content directory".to_owned()),
        }
    }
    let mut opts = Options::new(content.ok_or("missing content directory")?);
    opts.locked = locked;
    if let Some(a) = audio_root {
        opts.audio_root = a;
    }
    if let Some(n) = nickel {
        opts.nickel = n;
    }
    if let Some(s) = scowl {
        opts.scowl = s;
    }
    Ok((opts, out))
}

/// `content-build report [--missing] [--review] [--out <dir>] <content-dir>`.
///
/// `--missing` prints what keeps each draft KC from live, most valuable
/// first. `--review` writes `queue.txt` and `tasks.md` to `--out` (default
/// `target/review`): only after the build checks pass. Neither writes the
/// lock file.
fn report_command(args: impl Iterator<Item = std::ffi::OsString>) -> ExitCode {
    let (mut missing, mut review) = (false, false);
    let (mut content, mut out) = (None, PathBuf::from("target/review"));
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--missing") => missing = true,
            Some("--review") => review = true,
            Some("--out") => match args.next() {
                Some(dir) => out = PathBuf::from(dir),
                None => return usage_error("--out needs a value"),
            },
            Some(s) if s.starts_with('-') => return usage_error(&format!("unknown option {s}")),
            _ if content.is_none() => content = Some(PathBuf::from(arg)),
            _ => return usage_error("more than one content directory"),
        }
    }
    let Some(content) = content else {
        return usage_error("missing content directory");
    };
    if !missing && !review {
        return usage_error("report needs --missing or --review");
    }
    let opts = Options::new(&content);
    let (output, loaded) = match build_with_content(&opts) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("content-build: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(bundle) = output.bundle else {
        eprintln!("{}", output.report);
        eprintln!("content-build: fix the errors above first; the report covers checked content");
        return ExitCode::from(1);
    };
    if missing {
        print!("{}", missing_report(&output.report, &bundle));
    }
    if review {
        let files = content_build::review::review_files(&content, &loaded);
        let written = std::fs::create_dir_all(&out)
            .and_then(|()| std::fs::write(out.join("queue.txt"), &files.queue))
            .and_then(|()| std::fs::write(out.join("tasks.md"), &files.tasks))
            .and_then(|()| std::fs::write(out.join("hashes.txt"), &files.hashes));
        if let Err(e) = written {
            eprintln!("content-build: cannot write {}: {e}", out.display());
            return ExitCode::from(2);
        }
        println!(
            "review queue written to {}",
            out.join("queue.txt").display()
        );
        println!("tasks written to {}", out.join("tasks.md").display());
        println!("hashes written to {}", out.join("hashes.txt").display());
    }
    ExitCode::SUCCESS
}

/// Draft KCs with what is missing, by priority: importance × (1 + the
/// number of KCs that require it), the order in which writing pays off first.
fn missing_report(
    report: &content_build::report::Report,
    bundle: &refl_core::bundle::Bundle,
) -> String {
    use std::fmt::Write as _;
    let priority = |kc: &str| -> f64 {
        let Some(entry) = bundle.content.kcs.iter().find(|k| k.kc.id.as_str() == kc) else {
            return 0.0;
        };
        let waiting = bundle
            .content
            .requires_closure
            .values()
            .filter(|required| required.iter().any(|r| r.kc.as_str() == kc))
            .count();
        #[allow(clippy::cast_precision_loss)] // a handful of KCs
        let waiting = waiting as f64;
        entry.kc.importance.get() * (1.0 + waiting)
    };
    let mut draft: Vec<_> = report.draft.iter().collect();
    draft.sort_by(|a, b| {
        priority(&b.kc)
            .total_cmp(&priority(&a.kc))
            .then(a.kc.cmp(&b.kc))
    });
    let mut out = String::new();
    for d in draft {
        let _ = writeln!(out, "{} (priority {:.2})", d.kc, priority(&d.kc));
        for m in &d.missing {
            let _ = writeln!(out, "  - {m}");
        }
    }
    out
}

/// `content-build mark-reviewed --by <name> --lang <ru|en|uz> [--content <dir>]
/// (--all | <record> …)`: the only writer of `reviews.lock`. Computes the hash
/// of each selected record's current text and records that `<name>` read it.
fn mark_reviewed_command(args: impl Iterator<Item = std::ffi::OsString>) -> ExitCode {
    use content_build::ledger::{Selection, mark_reviewed};
    use refl_core::value::Lang;

    let (mut by, mut lang, mut all) = (None, None, false);
    let mut content = PathBuf::from("content");
    let mut records = Vec::new();
    let mut args = args;
    while let Some(arg) = args.next() {
        let Some(arg) = arg.to_str().map(str::to_owned) else {
            return usage_error("arguments must be UTF-8");
        };
        match arg.as_str() {
            "--all" => all = true,
            "--by" | "--lang" | "--content" => {
                let Some(value) = args.next().and_then(|v| v.into_string().ok()) else {
                    return usage_error(&format!("{arg} needs a value"));
                };
                match arg.as_str() {
                    "--by" => by = Some(value),
                    "--content" => content = PathBuf::from(value),
                    _ => {
                        lang = match value.as_str() {
                            "uz" => Some(Lang::Uz),
                            "ru" => Some(Lang::Ru),
                            "en" => Some(Lang::En),
                            _ => return usage_error("--lang is uz, ru or en"),
                        };
                    }
                }
            }
            s if s.starts_with('-') => return usage_error(&format!("unknown option {s}")),
            _ => records.push(arg),
        }
    }
    let (Some(by), Some(lang)) = (by, lang) else {
        return usage_error("mark-reviewed needs --by and --lang");
    };
    if all != records.is_empty() {
        return usage_error("name the records to mark, or --all");
    }
    let opts = Options::new(&content);
    let (loaded, diags) = match content_build::load::load(&opts.content, &opts.nickel) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("content-build: cannot read {}: {e}", content.display());
            return ExitCode::from(2);
        }
    };
    if let Some(d) = diags.first() {
        eprintln!("content-build: the content does not load, so no text can be hashed:\n{d}");
        return ExitCode::from(1);
    }
    let selection = if all {
        Selection::All
    } else {
        Selection::Records(records)
    };
    match mark_reviewed(&loaded, &opts.reviews, &by, lang, &selection) {
        Ok(keys) => {
            for k in &keys {
                println!("reviewed by {by} ({lang:?}): {k}");
            }
            println!(
                "{} record(s) written to {}: commit the file",
                keys.len(),
                opts.reviews.display()
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("content-build: {why}");
            ExitCode::from(1)
        }
    }
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("content-build: {msg}");
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut argv = std::env::args_os().skip(1).peekable();
    if argv.peek().is_some_and(|a| a == "report") {
        argv.next();
        return report_command(argv);
    }
    if argv.peek().is_some_and(|a| a == "mark-reviewed") {
        argv.next();
        return mark_reviewed_command(argv);
    }
    let (opts, out) = match parse_args() {
        Ok(v) => v,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("content-build: {msg}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let output = match build(&opts) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("content-build: {e}");
            return ExitCode::from(2);
        }
    };
    println!("{}", output.report);
    let (Some(bundle), Some(lexicon)) = (output.bundle, output.lexicon) else {
        return ExitCode::from(1);
    };
    if let Some(text) = output.lock_update {
        if let Err(e) = std::fs::write(&opts.lock, text) {
            eprintln!("content-build: cannot write {}: {e}", opts.lock.display());
            return ExitCode::from(2);
        }
        println!(
            "lock written to {}: commit it with the content",
            opts.lock.display()
        );
    }
    let lex = &bundle.content.lexicon;
    println!("content version {}", bundle.content_version.0);
    println!("fold version {}", refl_core::model::fold_version(&bundle));
    println!(
        "lexicon {}: {} words, {} bytes",
        lex.version, lex.words, lex.bytes
    );
    if let Some(path) = out {
        if let Err(e) = std::fs::write(&path, to_bytes(&bundle)) {
            eprintln!("content-build: cannot write {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("bundle written to {}", path.display());
        let lexicon_path = path.with_file_name(content_build::lexicon::file_name(lex));
        if let Err(e) = std::fs::write(&lexicon_path, lexicon.as_str()) {
            eprintln!(
                "content-build: cannot write {}: {e}",
                lexicon_path.display()
            );
            return ExitCode::from(2);
        }
        println!("lexicon written to {}", lexicon_path.display());
    }
    ExitCode::SUCCESS
}
