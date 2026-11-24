//! `content-build <content-dir> [--out <bundle.json>] [--audio-root <dir>] [--nickel <path>] [--scowl <dir>]`
//!
//! With `--out`, the lexicon the bundle references is written next to it as
//! `lexicon-<version>.txt`. SCOWL defaults to `$REFL_SCOWL_DICT`.
//!
//! Prints the report. Exit status: 0 on success, 1 if the content has errors,
//! 2 on usage or I/O problems.

use std::path::PathBuf;
use std::process::ExitCode;

use content_build::{Options, build, to_bytes};

const USAGE: &str = "usage: content-build <content-dir> [--out <bundle.json>] [--audio-root <dir>] [--nickel <path>] [--scowl <dir>]";

fn parse_args() -> Result<(Options, Option<PathBuf>), String> {
    let mut args = std::env::args_os().skip(1);
    let mut content = None;
    let mut out = None;
    let mut audio_root = None;
    let mut nickel = None;
    let mut scowl = None;
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
            Some("-h" | "--help") => return Err(String::new()),
            Some(s) if s.starts_with('-') => return Err(format!("unknown option {s}")),
            _ if content.is_none() => content = Some(PathBuf::from(arg)),
            _ => return Err("more than one content directory".to_owned()),
        }
    }
    let mut opts = Options::new(content.ok_or("missing content directory")?);
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

fn main() -> ExitCode {
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
    let lex = &bundle.content.lexicon;
    println!("content version {}", bundle.content_version.0);
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
