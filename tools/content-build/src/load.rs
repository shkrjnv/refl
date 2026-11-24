//! Loading content: every `.ncl` file under the content root (except those in
//! `lib/` directories, which hold imports such as contracts and helpers) is
//! exported with `nickel export` and parsed into refl-core types. The Rust
//! types are the source of truth; Nickel contracts only help authors.

use std::path::{Path, PathBuf};
use std::process::Command;

use refl_core::content::{Edge, Explanation, Instruction, Kc, Misconception};
use refl_core::item::Item;
use refl_core::lesson::Lesson;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::report::{Code, Diagnostic, Severity};

/// A value together with the file it came from (relative to the content root).
#[derive(Debug, Clone)]
pub struct Sourced<T> {
    pub file: PathBuf,
    pub value: T,
}

/// All content, in file order, then in order within each file.
#[derive(Debug, Default)]
pub struct Content {
    pub kcs: Vec<Sourced<Kc>>,
    pub edges: Vec<Sourced<Edge>>,
    pub misconceptions: Vec<Sourced<Misconception>>,
    pub explanations: Vec<Sourced<Explanation>>,
    pub instructions: Vec<Sourced<Instruction>>,
    pub items: Vec<Sourced<Item>>,
    pub lessons: Vec<Sourced<Lesson>>,
}

const SECTIONS: [&str; 7] = [
    "kcs",
    "edges",
    "misconceptions",
    "explanations",
    "instructions",
    "items",
    "lessons",
];

fn error(code: Code, file: &Path, subject: Option<String>, message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code,
        file: file.to_path_buf(),
        subject,
        message,
    }
}

/// `.ncl` files under `root`, sorted, skipping `lib/` directories.
///
/// # Errors
/// The directory cannot be read.
pub fn content_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "lib") {
                    walk(&path, out)?;
                }
            } else if path.extension().is_some_and(|e| e == "ncl") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn nickel_export(nickel: &Path, path: &Path) -> Result<String, String> {
    let out = Command::new(nickel)
        .arg("export")
        .arg("--format")
        .arg("json")
        .arg(path)
        .output()
        .map_err(|e| format!("cannot run {}: {e}", nickel.display()))?;
    if out.status.success() {
        String::from_utf8(out.stdout).map_err(|e| format!("nickel output is not UTF-8: {e}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim_end().to_owned())
    }
}

fn parse_section<T: DeserializeOwned>(
    file: &Path,
    section: &str,
    value: Value,
    out: &mut Vec<Sourced<T>>,
    diags: &mut Vec<Diagnostic>,
) {
    let Value::Array(entries) = value else {
        diags.push(error(
            Code::Schema,
            file,
            None,
            format!("`{section}` must be an array"),
        ));
        return;
    };
    for (i, entry) in entries.into_iter().enumerate() {
        let id = entry.get("id").and_then(Value::as_str).map(str::to_owned);
        match serde_path_to_error::deserialize::<_, T>(entry) {
            Ok(value) => out.push(Sourced {
                file: file.to_path_buf(),
                value,
            }),
            Err(e) => {
                let subject = match id {
                    Some(id) => format!("{section}[{i}] `{id}`"),
                    None => format!("{section}[{i}]"),
                };
                let path = e.path().to_string();
                let message = if path == "." {
                    e.into_inner().to_string()
                } else {
                    format!("{path}: {}", e.into_inner())
                };
                diags.push(error(Code::Schema, file, Some(subject), message));
            }
        }
    }
}

/// Exports and parses all content files. Problems become diagnostics; the
/// returned content holds every entity that parsed.
///
/// # Errors
/// The content root cannot be read.
pub fn load(root: &Path, nickel: &Path) -> std::io::Result<(Content, Vec<Diagnostic>)> {
    let mut content = Content::default();
    let mut diags = Vec::new();
    for path in content_files(root)? {
        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        let json = match nickel_export(nickel, &path) {
            Ok(json) => json,
            Err(msg) => {
                diags.push(error(Code::Nickel, &rel, None, msg));
                continue;
            }
        };
        let value: Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => {
                diags.push(error(Code::Nickel, &rel, None, e.to_string()));
                continue;
            }
        };
        let Value::Object(map) = value else {
            diags.push(error(
                Code::Schema,
                &rel,
                None,
                format!(
                    "file must export a record with fields {}",
                    SECTIONS.join(", ")
                ),
            ));
            continue;
        };
        for (key, value) in map {
            let d = &mut diags;
            match key.as_str() {
                "kcs" => parse_section(&rel, &key, value, &mut content.kcs, d),
                "edges" => parse_section(&rel, &key, value, &mut content.edges, d),
                "misconceptions" => {
                    parse_section(&rel, &key, value, &mut content.misconceptions, d);
                }
                "explanations" => parse_section(&rel, &key, value, &mut content.explanations, d),
                "instructions" => parse_section(&rel, &key, value, &mut content.instructions, d),
                "items" => parse_section(&rel, &key, value, &mut content.items, d),
                "lessons" => parse_section(&rel, &key, value, &mut content.lessons, d),
                _ => d.push(error(
                    Code::Schema,
                    &rel,
                    None,
                    format!(
                        "unknown field `{key}`; expected one of {}",
                        SECTIONS.join(", ")
                    ),
                )),
            }
        }
    }
    Ok((content, diags))
}
