//! The checker's word list (`docs/02-layer1-content.md`, «Проверка ответов на
//! ввод», «Словарь проверщика»): lowercase words, sorted, one per line. A
//! 1–2 character difference is a typo only if the answer's word is not in
//! it. content-build writes it as a separate file named by its version; the
//! bundle references it with [`LexiconRef`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// What the bundle records about its lexicon: the version is part of the
/// checker and content versions, so a new word list re-checks old answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexiconRef {
    /// Hex SHA-256 of the lexicon file.
    pub version: String,
    pub words: u32,
    /// Size of the lexicon file.
    pub bytes: u32,
}

/// Sorted, unique, lowercase words over one string; lookups are a binary
/// search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lexicon {
    /// The file contents: every word followed by `\n`.
    text: String,
    /// Byte offset of each word in `text`.
    starts: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexiconError {
    /// Line (0-based) is empty, not lowercase, or not after the previous one.
    BadWord(usize),
    MissingFinalNewline,
    TooLarge,
}

impl fmt::Display for LexiconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadWord(i) => write!(
                f,
                "lexicon line {i} is empty, not lowercase, or out of order"
            ),
            Self::MissingFinalNewline => f.write_str("lexicon must end with a newline"),
            Self::TooLarge => f.write_str("lexicon exceeds 4 GiB"),
        }
    }
}

impl core::error::Error for LexiconError {}

impl Lexicon {
    /// Builds a lexicon from any words: lowercased, sorted, deduplicated.
    ///
    /// # Panics
    /// If the words take more than 4 GiB.
    pub fn from_words(words: impl IntoIterator<Item = String>) -> Self {
        let mut words: Vec<String> = words
            .into_iter()
            .map(|w| w.to_lowercase())
            .filter(|w| !w.is_empty() && !w.contains('\n'))
            .collect();
        words.sort();
        words.dedup();
        let mut text = String::new();
        for w in &words {
            text.push_str(w);
            text.push('\n');
        }
        Self::parse(text).expect("sorted, unique, lowercase words form a valid lexicon")
    }

    /// Reads a lexicon file.
    ///
    /// # Errors
    /// An empty or non-lowercase line, lines out of strict order, no final
    /// newline, or a file over 4 GiB.
    pub fn parse(text: String) -> Result<Self, LexiconError> {
        if u32::try_from(text.len()).is_err() {
            return Err(LexiconError::TooLarge);
        }
        if !text.is_empty() && !text.ends_with('\n') {
            return Err(LexiconError::MissingFinalNewline);
        }
        let mut starts = Vec::new();
        let mut prev: Option<&str> = None;
        let mut at = 0usize;
        for (i, word) in text.split_terminator('\n').enumerate() {
            let ordered = prev.is_none_or(|p| p < word);
            if word.is_empty() || word.to_lowercase() != word || !ordered {
                return Err(LexiconError::BadWord(i));
            }
            starts.push(u32::try_from(at).map_err(|_| LexiconError::TooLarge)?);
            at += word.len() + 1;
            prev = Some(word);
        }
        Ok(Self { text, starts })
    }

    fn word(&self, i: usize) -> &str {
        let start = self.starts[i] as usize;
        let end = self
            .starts
            .get(i + 1)
            .map_or(self.text.len(), |&s| s as usize);
        &self.text[start..end - 1]
    }

    /// Whether `word` is in the lexicon, ignoring case.
    pub fn contains(&self, word: &str) -> bool {
        let word = word.to_lowercase();
        let (mut lo, mut hi) = (0, self.starts.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.word(mid).cmp(word.as_str()) {
                core::cmp::Ordering::Less => lo = mid + 1,
                core::cmp::Ordering::Greater => hi = mid,
                core::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// The file contents.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn builds_sorted_unique_lowercase() {
        let lex = Lexicon::from_words(["Than", "then", "than", "it's"].map(ToString::to_string));
        assert_eq!(lex.as_str(), "it's\nthan\nthen\n");
        assert_eq!(lex.len(), 3);
        assert!(lex.contains("THEN"));
        assert!(lex.contains("it's"));
        assert!(!lex.contains("teh"));
        assert!(!lex.contains(""));
        assert_eq!(Lexicon::parse(lex.as_str().into()), Ok(lex));
    }

    #[test]
    fn rejects_malformed_files() {
        let parse = |s: &str| Lexicon::parse(s.into());
        assert_eq!(parse("b\na\n"), Err(LexiconError::BadWord(1)));
        assert_eq!(parse("a\na\n"), Err(LexiconError::BadWord(1)));
        assert_eq!(parse("A\n"), Err(LexiconError::BadWord(0)));
        assert_eq!(parse("a\n\nb\n"), Err(LexiconError::BadWord(1)));
        assert_eq!(parse("a"), Err(LexiconError::MissingFinalNewline));
        assert!(parse("").unwrap().is_empty());
    }
}
