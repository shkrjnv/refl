//! `book-import <input> [--out <dir>] [--id book.slug] [--title <title>]
//! [--author <author>] [--level A1] --license <license> [--source <source>]
//! [--force]`
//!
//! Writes `content/books/<slug>/source.md`. Exit status: 0 on success, 1 on
//! an import problem, 2 on usage errors.

use std::path::PathBuf;
use std::process::ExitCode;

use book_import::{Options, import};

const USAGE: &str = "usage: book-import <input.epub|input.pdf|input.txt|input.md> [--out <dir>] [--id book.slug] [--title <title>] [--author <author>] [--level <A1-C1>] --license <license> [--source <source>] [--force]";

fn usage_error(message: &str) -> ExitCode {
    eprintln!("book-import: {message}\n{USAGE}");
    ExitCode::from(2)
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args_os().skip(1);
    let mut input = None;
    let mut out = None;
    let (mut id, mut title, mut author, mut level) = (None, None, None, None);
    let (mut license, mut source, mut force) = (None, None, false);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.to_str() {
            Some("--out") => out = Some(value("--out")?),
            Some("--id") => id = Some(value("--id")?.to_string_lossy().into_owned()),
            Some("--title") => title = Some(value("--title")?.to_string_lossy().into_owned()),
            Some("--author") => author = Some(value("--author")?.to_string_lossy().into_owned()),
            Some("--level") => level = Some(value("--level")?.to_string_lossy().into_owned()),
            Some("--license") => {
                license = Some(value("--license")?.to_string_lossy().into_owned());
            }
            Some("--source") => source = Some(value("--source")?.to_string_lossy().into_owned()),
            Some("--force") => force = true,
            Some("-h" | "--help") => return Err(String::new()),
            Some(s) if s.starts_with('-') => return Err(format!("unknown option {s}")),
            _ if input.is_none() => input = Some(PathBuf::from(arg)),
            _ => return Err("more than one input file".to_owned()),
        }
    }
    let mut opts = Options::new(input.ok_or("missing input file")?);
    if let Some(value) = out {
        opts.out = value;
    }
    opts.id = id;
    opts.title = title;
    opts.author = author;
    if let Some(value) = level {
        opts.level = value;
    }
    opts.license = license.ok_or("--license is required")?;
    opts.source = source;
    opts.force = force;
    Ok(opts)
}

fn main() -> ExitCode {
    let opts = match parse() {
        Ok(opts) => opts,
        Err(message) if message.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => return usage_error(&message),
    };
    match import(&opts) {
        Ok(imported) => {
            for warning in &imported.warnings {
                eprintln!("book-import: repaired: {warning}");
            }
            println!(
                "{} imported: {} chapter(s) -> {}",
                imported.book_id,
                imported.chapters,
                imported.path.display()
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("book-import: {message}");
            ExitCode::from(1)
        }
    }
}
