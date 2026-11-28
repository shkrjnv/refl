//! `book-build <books-dir> [--out <dir>] [--bundle <bundle.json>]
//! [--third-party <THIRD_PARTY.md>] [--content-license <content/LICENSE>]
//! [--nickel <path>]`
//!
//! Compiles every `content/books/<book-id>/book.ncl` into a deterministic
//! JSON asset plus `index.json`. Exit status: 0 on success, 1 if any book has
//! an error, 2 on usage or I/O problems.

use std::path::PathBuf;
use std::process::ExitCode;

use book_build::{Options, build, write};

const USAGE: &str = "usage: book-build <books-dir> [--out <dir>] [--bundle <bundle.json>] [--third-party <THIRD_PARTY.md>] [--content-license <content/LICENSE>] [--nickel <path>]";

fn usage_error(message: &str) -> ExitCode {
    eprintln!("book-build: {message}\n{USAGE}");
    ExitCode::from(2)
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args_os().skip(1);
    let (mut books, mut out, mut bundle) = (None, None, None);
    let (mut third_party, mut content_license, mut nickel) = (None, None, None);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.to_str() {
            Some("--out") => out = Some(value("--out")?),
            Some("--bundle") => bundle = Some(value("--bundle")?),
            Some("--third-party") => third_party = Some(value("--third-party")?),
            Some("--content-license") => content_license = Some(value("--content-license")?),
            Some("--nickel") => nickel = Some(value("--nickel")?),
            Some("-h" | "--help") => return Err(String::new()),
            Some(s) if s.starts_with('-') => return Err(format!("unknown option {s}")),
            _ if books.is_none() => books = Some(PathBuf::from(arg)),
            _ => return Err("more than one books directory".to_owned()),
        }
    }
    let mut opts = Options::new(books.ok_or("missing books directory")?);
    if let Some(value) = out {
        opts.out = value;
    }
    if let Some(value) = bundle {
        opts.bundle = value;
    }
    if let Some(value) = third_party {
        opts.third_party = value;
    }
    if let Some(value) = content_license {
        opts.content_license = value;
    }
    if let Some(value) = nickel {
        opts.nickel = value;
    }
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
    let (compiled, diagnostics) = match build(&opts) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("book-build: {error}");
            return ExitCode::from(2);
        }
    };
    for diagnostic in &diagnostics {
        eprintln!("book-build: {diagnostic}");
    }
    if !diagnostics.is_empty() {
        return ExitCode::from(1);
    }
    if let Err(error) = write(&opts, &compiled) {
        eprintln!("book-build: cannot write {}: {error}", opts.out.display());
        return ExitCode::from(2);
    }
    for item in &compiled {
        println!(
            "{} {} {} bytes -> {}",
            item.book.id,
            item.book.version,
            item.bytes.len(),
            opts.out.join(format!("{}.json", item.book.id)).display()
        );
    }
    println!(
        "{} book(s), index written to {}",
        compiled.len(),
        opts.out.join("index.json").display()
    );
    ExitCode::SUCCESS
}
