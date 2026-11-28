//! `content/book-versions.lock`: the published `version` and token-index
//! hash of every book (`docs/16-vocabulary-library-notes-graph.md`, §2).
//!
//! Token offsets in `book_mark_*` events refer to the sentence's token
//! positions. A change that keeps the structure (a typo fix, a new title)
//! keeps the token-index hash; a re-index that moves tokens must be explicit:
//! a normal `book-build` fails, `book-build --relock` records the new hash.
//!
//! The file is sorted by book id, one compact JSON entry per line, followed
//! by `sha256 <hex>` over the entry lines. A hand edit without the digest is
//! visible and fails the build.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use refl_core::book::Book;
use refl_core::ids::BookId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FILE: &str = "book-versions.lock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockEntry {
    pub book: BookId,
    pub version: String,
    pub token_index: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lock {
    entries: BTreeMap<BookId, LockEntry>,
}

impl Lock {
    pub fn entries(&self) -> impl Iterator<Item = &LockEntry> {
        self.entries.values()
    }

    pub fn get(&self, book: &BookId) -> Option<&LockEntry> {
        self.entries.get(book)
    }

    pub fn insert(&mut self, entry: LockEntry) {
        self.entries.insert(entry.book.clone(), entry);
    }

    /// Parses the lock and verifies the digest.
    ///
    /// # Errors
    /// A malformed line, a repeated or unsorted book, or a digest mismatch.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim_end_matches('\n');
        if trimmed.is_empty() {
            return Ok(Self::default());
        }
        let (body, last) = trimmed
            .rsplit_once('\n')
            .map_or(("", trimmed), |(body, last)| (body, last));
        let Some(expected) = last.strip_prefix("sha256 ") else {
            return Err("the last line is not `sha256 <hex>`".into());
        };
        let digest = Sha256::digest(if body.is_empty() {
            Vec::new()
        } else {
            format!("{body}\n").into_bytes()
        });
        let actual = digest.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
        if actual != expected {
            return Err(
                "the digest does not match the entries: a line was removed or changed by hand"
                    .into(),
            );
        }
        let mut entries = BTreeMap::new();
        for (number, line) in body.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let entry: LockEntry = serde_json::from_str(line)
                .map_err(|error| format!("line {}: {error}", number + 1))?;
            if entries.insert(entry.book.clone(), entry).is_some() {
                return Err(format!("line {}: book is listed twice", number + 1));
            }
        }
        Ok(Self { entries })
    }

    /// Renders entries and their digest.
    ///
    /// # Panics
    /// Never: lock entries always serialize.
    pub fn render(&self) -> String {
        let mut body = String::new();
        for entry in self.entries.values() {
            body.push_str(&serde_json::to_string(entry).expect("lock entry serializes"));
            body.push('\n');
        }
        let digest = Sha256::digest(body.as_bytes());
        let digest = digest.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
        format!("{body}sha256 {digest}\n")
    }
}

/// SHA-256 over the token *structure*: chapter, paragraph, sentence and token
/// counts with their ids. Text edits that keep the structure do not change
/// it; re-splitting a sentence or adding a token does.
///
/// # Panics
/// Never: the structural projection always serializes.
pub fn token_index(book: &Book) -> String {
    #[derive(Serialize)]
    struct Sentence<'a> {
        id: &'a str,
        tokens: usize,
    }
    #[derive(Serialize)]
    struct Paragraph<'a> {
        id: &'a str,
        sentences: Vec<Sentence<'a>>,
    }
    #[derive(Serialize)]
    struct Chapter<'a> {
        id: &'a str,
        paragraphs: Vec<Paragraph<'a>>,
    }
    let chapters: Vec<Chapter<'_>> = book
        .chapters
        .iter()
        .map(|chapter| Chapter {
            id: &chapter.id,
            paragraphs: chapter
                .paragraphs
                .iter()
                .map(|paragraph| Paragraph {
                    id: &paragraph.id,
                    sentences: paragraph
                        .sentences
                        .iter()
                        .map(|sentence| Sentence {
                            id: &sentence.id,
                            tokens: sentence.tokens.len(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect();
    let bytes = serde_json::to_vec(&chapters).expect("token index serializes");
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> LockEntry {
        LockEntry {
            book: BookId::new(id).unwrap(),
            version: "a".repeat(64),
            token_index: "b".repeat(64),
        }
    }

    #[test]
    fn round_trips_and_catches_edits() {
        let mut lock = Lock::default();
        lock.insert(entry("book.one"));
        lock.insert(entry("book.two"));
        let text = lock.render();
        assert_eq!(Lock::parse(&text).unwrap(), lock);
        let edited = text.replace("aaaa", "cccc");
        assert!(Lock::parse(&edited).is_err());
    }

    #[test]
    fn rejects_a_duplicate_book() {
        let mut lock = Lock::default();
        let first = entry("book.one");
        lock.insert(first.clone());
        let mut text = lock.render();
        let duplicate = serde_json::to_string(&first).unwrap();
        text = format!("{duplicate}\n{text}");
        assert!(Lock::parse(&text).is_err());
    }
}
