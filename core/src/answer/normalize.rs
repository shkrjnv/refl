//! Answer normalization and tokenization (`docs/02-layer1-content.md`,
//! «Язык шаблонов ответов»). One function, applied identically to learner
//! answers and to template literals, so comparison happens in one space.
//!
//! Any change to the rules or tables here changes which answers match, so it
//! bumps [`NORMALIZATION_VERSION`], which makes a new content version and
//! triggers re-checking and `reclassified` events on the server.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use unicode_normalization::UnicodeNormalization;

use crate::item::{Contractions, Item};

/// Version of the normalization rules and tables, recorded in the content bundle.
pub const NORMALIZATION_VERSION: u32 = 1;

/// Per-item comparison flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NormalizeOptions {
    pub case_sensitive: bool,
    pub contractions: Contractions,
    pub punctuation_sensitive: bool,
}

impl NormalizeOptions {
    pub fn of(item: &Item) -> Self {
        Self {
            case_sensitive: item.case_sensitive,
            contractions: item.contractions,
            punctuation_sensitive: item.punctuation_sensitive,
        }
    }
}

/// Contraction table for `contractions = "equivalent"`, sorted by key
/// (lowercase). Only unambiguous contractions: `'s` and `'d` are excluded.
const CONTRACTIONS: &[(&str, &[&str])] = &[
    ("aren't", &["are", "not"]),
    ("can't", &["can", "not"]),
    ("could've", &["could", "have"]),
    ("couldn't", &["could", "not"]),
    ("didn't", &["did", "not"]),
    ("doesn't", &["does", "not"]),
    ("don't", &["do", "not"]),
    ("hadn't", &["had", "not"]),
    ("hasn't", &["has", "not"]),
    ("haven't", &["have", "not"]),
    ("he'll", &["he", "will"]),
    ("i'll", &["i", "will"]),
    ("i'm", &["i", "am"]),
    ("i've", &["i", "have"]),
    ("isn't", &["is", "not"]),
    ("it'll", &["it", "will"]),
    ("might've", &["might", "have"]),
    ("mightn't", &["might", "not"]),
    ("must've", &["must", "have"]),
    ("mustn't", &["must", "not"]),
    ("needn't", &["need", "not"]),
    ("shan't", &["shall", "not"]),
    ("she'll", &["she", "will"]),
    ("should've", &["should", "have"]),
    ("shouldn't", &["should", "not"]),
    ("they'll", &["they", "will"]),
    ("they're", &["they", "are"]),
    ("they've", &["they", "have"]),
    ("wasn't", &["was", "not"]),
    ("we'll", &["we", "will"]),
    ("we're", &["we", "are"]),
    ("we've", &["we", "have"]),
    ("weren't", &["were", "not"]),
    ("won't", &["will", "not"]),
    ("would've", &["would", "have"]),
    ("wouldn't", &["would", "not"]),
    ("you'll", &["you", "will"]),
    ("you're", &["you", "are"]),
    ("you've", &["you", "have"]),
];

/// Apostrophe and quote variants mapped to ASCII.
fn ascii_quote(c: char) -> char {
    match c {
        '\u{2018}' | '\u{2019}' | '\u{201B}' | '\u{02BC}' | '\u{2032}' | '`' | '\u{00B4}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{00AB}' | '\u{00BB}'
        | '\u{2033}' => '"',
        c => c,
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

fn is_joiner(c: char) -> bool {
    c == '\'' || c == '-'
}

/// A raw token before per-item rules are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawToken {
    /// Letters and digits, with apostrophes and hyphens inside the word.
    Word(String),
    /// Any other non-space character, one per token.
    Punct(char),
}

/// NFC, ASCII quotes, then split into words and punctuation; whitespace only
/// separates. Independent of item flags.
pub fn tokenize(text: &str) -> Vec<RawToken> {
    let chars: Vec<char> = text.nfc().map(ascii_quote).collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if is_word_char(c) {
            let mut word = String::new();
            word.push(c);
            i += 1;
            while i < chars.len() {
                let c = chars[i];
                if is_word_char(c) {
                    word.push(c);
                    i += 1;
                } else if is_joiner(c) && chars.get(i + 1).is_some_and(|&n| is_word_char(n)) {
                    word.push(c);
                    word.push(chars[i + 1]);
                    i += 2;
                } else {
                    break;
                }
            }
            tokens.push(RawToken::Word(word));
        } else {
            tokens.push(RawToken::Punct(c));
            i += 1;
        }
    }
    tokens
}

fn expand_contraction(word: &str) -> Option<Vec<String>> {
    let lower = word.to_lowercase();
    let i = CONTRACTIONS
        .binary_search_by(|(k, _)| (*k).cmp(lower.as_str()))
        .ok()?;
    let mut out: Vec<String> = CONTRACTIONS[i].1.iter().map(ToString::to_string).collect();
    // The case of the first letter carries over to the first word: Don't → Do not.
    if word.chars().next().is_some_and(char::is_uppercase) {
        let first = &out[0];
        let mut chars = first.chars();
        if let Some(c) = chars.next() {
            out[0] = c.to_uppercase().chain(chars).collect();
        }
    }
    Some(out)
}

/// Normalizes `text` into the token sequence that answers and template
/// literals are compared in.
pub fn normalize(text: &str, opts: NormalizeOptions) -> Vec<String> {
    let mut out = Vec::new();
    for token in tokenize(text) {
        let words = match token {
            RawToken::Punct(c) => {
                if opts.punctuation_sensitive {
                    out.push(c.to_string());
                }
                continue;
            }
            RawToken::Word(w) => match opts.contractions {
                Contractions::Equivalent => {
                    expand_contraction(&w).unwrap_or_else(|| alloc::vec![w])
                }
                Contractions::Strict => alloc::vec![w],
            },
        };
        for w in words {
            out.push(if opts.case_sensitive {
                w
            } else {
                w.to_lowercase()
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn n(text: &str, opts: NormalizeOptions) -> Vec<String> {
        normalize(text, opts)
    }

    fn d(text: &str) -> Vec<String> {
        n(text, NormalizeOptions::default())
    }

    #[test]
    fn contraction_table_is_sorted_and_unique() {
        assert!(CONTRACTIONS.windows(2).all(|w| w[0].0 < w[1].0));
        for (k, _) in CONTRACTIONS {
            assert_eq!(*k, k.to_lowercase());
            assert!(!k.ends_with("'s") && !k.ends_with("'d"), "{k} is ambiguous");
        }
    }

    #[test]
    fn defaults_fold_case_drop_punctuation_keep_contractions() {
        assert_eq!(d("  I  don’t   know. "), vec!["i", "don't", "know"]);
        assert_eq!(d("Hello, world!"), vec!["hello", "world"]);
    }

    #[test]
    fn words_keep_inner_apostrophes_and_hyphens() {
        assert_eq!(
            d("well-known rock'n'roll"),
            vec!["well-known", "rock'n'roll"]
        );
        let p = NormalizeOptions {
            punctuation_sensitive: true,
            ..NormalizeOptions::default()
        };
        assert_eq!(
            n("dogs' 'em -x- a--b", p),
            vec!["dogs", "'", "'", "em", "-", "x", "-", "a", "-", "-", "b"]
        );
        // Possessive: a trailing apostrophe is punctuation (docs/02).
        assert_eq!(d("dogs'"), d("dogs"));
    }

    #[test]
    fn nfc_and_quote_variants() {
        // e + combining acute == precomposed é.
        assert_eq!(d("cafe\u{301}"), d("caf\u{e9}"));
        assert_eq!(d("it\u{2019}s"), d("it's"));
        let p = NormalizeOptions {
            punctuation_sensitive: true,
            ..NormalizeOptions::default()
        };
        assert_eq!(
            n("\u{201C}hi\u{201D} \u{ab}hi\u{bb}", p),
            vec!["\"", "hi", "\"", "\"", "hi", "\""]
        );
    }

    #[test]
    fn case_sensitive_keeps_case() {
        let p = NormalizeOptions {
            case_sensitive: true,
            ..NormalizeOptions::default()
        };
        assert_eq!(n("London is Big", p), vec!["London", "is", "Big"]);
    }

    #[test]
    fn equivalent_contractions_expand() {
        let p = NormalizeOptions {
            contractions: Contractions::Equivalent,
            ..NormalizeOptions::default()
        };
        assert_eq!(n("I don't", p), n("I do not", p));
        assert_eq!(n("can't", p), vec!["can", "not"]);
        assert_ne!(n("cannot", p), n("can't", p));
        // Ambiguous contractions are left alone.
        assert_eq!(n("he's she'd", p), vec!["he's", "she'd"]);
        let cs = NormalizeOptions {
            case_sensitive: true,
            ..p
        };
        assert_eq!(n("Don't", cs), vec!["Do", "not"]);
        assert_eq!(n("I'm", cs), vec!["I", "am"]);
        assert_eq!(n("i'm", cs), vec!["i", "am"]);
        assert_eq!(n("WON'T", cs), vec!["Will", "not"]);
    }
}
