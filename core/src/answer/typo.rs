//! Step 4 of the input pipeline, the typo case (`docs/02-layer1-content.md`,
//! «Проверка ответов на ввод»): the answer is aligned word by word with the
//! answers a slot accepts. It is a typo when it has as many words as one of
//! them and every word that differs is a typo of the accepted word: letters
//! only (inner apostrophes and hyphens allowed, no digits), one or two
//! Levenshtein edits away, and not itself a lexicon word. So then/than and
//! its/it's are not typos, teh → the and recieve → receive are.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use crate::answer::dfa::{Dfa, TokenTable};
use crate::lexicon::Lexicon;

/// Letters, with apostrophes and hyphens only between letters (the
/// tokenizer already guarantees the "between").
fn is_letter_word(w: &str) -> bool {
    w.chars().any(char::is_alphabetic)
        && w.chars()
            .all(|c| c.is_alphabetic() || c == '\'' || c == '-')
}

/// Levenshtein distance over characters is 1 or 2.
fn one_or_two_edits(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a == b || a.len().abs_diff(b.len()) > 2 {
        return false;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut row = Vec::with_capacity(b.len() + 1);
        row.push(i + 1);
        for (j, &cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            row.push(substitute.min(prev[j + 1] + 1).min(row[j] + 1));
        }
        // Every later row is at least the minimum of this one.
        if row.iter().min().is_some_and(|&m| m > 2) {
            return false;
        }
        prev = row;
    }
    prev[b.len()] <= 2
}

/// Whether the normalized `answer` is a typo of an answer `accepted`
/// accepts. Call it only after `accepted` rejected the answer as it is.
pub fn is_typo(answer: &[String], accepted: &Dfa, table: &TokenTable, lexicon: &Lexicon) -> bool {
    let states = accepted.states();
    // States reachable with every word so far equal or a typo; the accepted
    // language is finite, so this set stays small.
    let mut current: BTreeSet<u32> = BTreeSet::from([0]);
    for word in answer {
        let exact = table.get(word);
        let may_be_typo = is_letter_word(word) && !lexicon.contains(word);
        let mut next = BTreeSet::new();
        for &s in &current {
            for &(token, to) in &states[s as usize].next {
                let matches = Some(token) == exact
                    || (may_be_typo
                        && table
                            .text(token)
                            .is_some_and(|w| is_letter_word(w) && one_or_two_edits(word, w)));
                if matches {
                    next.insert(to);
                }
            }
        }
        if next.is_empty() {
            return false;
        }
        current = next;
    }
    current.iter().any(|&s| states[s as usize].accepting)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits() {
        assert!(one_or_two_edits("teh", "the"));
        assert!(one_or_two_edits("recieve", "receive"));
        assert!(one_or_two_edits("cat", "cats"));
        assert!(!one_or_two_edits("cat", "cat"));
        assert!(!one_or_two_edits("cat", "dogs"));
        assert!(!one_or_two_edits("a", "abcd"));
        assert!(one_or_two_edits("é", "e"));
    }

    #[test]
    fn letter_words() {
        assert!(is_letter_word("it's"));
        assert!(is_letter_word("well-known"));
        assert!(is_letter_word("ñandú"));
        assert!(!is_letter_word("1990"));
        assert!(!is_letter_word("b2"));
        assert!(!is_letter_word("."));
    }
}
