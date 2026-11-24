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

/// Damerau–Levenshtein distance (insertion, deletion, substitution, swap of
/// neighbours; the optimal string alignment variant) over characters.
fn distance(a: &[char], b: &[char]) -> usize {
    let mut rows: [Vec<usize>; 3] = [
        alloc::vec![0; b.len() + 1],
        (0..=b.len()).collect(),
        alloc::vec![0; b.len() + 1],
    ];
    // rows[0]: two back, rows[1]: previous, rows[2]: current.
    for i in 1..=a.len() {
        rows[2][0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (rows[1][j - 1] + cost)
                .min(rows[1][j] + 1)
                .min(rows[2][j - 1] + 1);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(rows[0][j - 2] + 1);
            }
            rows[2][j] = d;
        }
        rows.rotate_left(1);
    }
    rows[1][b.len()]
}

/// `word` is a misspelling of `accepted`: at least one edit, at most 1 for
/// accepted words of up to 5 letters and 2 for longer ones.
fn is_near(word: &str, accepted: &str) -> bool {
    let a: Vec<char> = word.chars().collect();
    let b: Vec<char> = accepted.chars().collect();
    let bound = if b.len() <= 5 { 1 } else { 2 };
    a != b && a.len().abs_diff(b.len()) <= bound && distance(&a, &b) <= bound
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
                            .is_some_and(|w| is_letter_word(w) && is_near(word, w)));
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
        // A swap of neighbours is one edit.
        assert!(is_near("teh", "the"));
        assert!(is_near("recieve", "receive"));
        assert!(is_near("cat", "cats"));
        assert!(is_near("é", "e"));
        assert!(!is_near("cat", "cat"));
        assert!(!is_near("cat", "dogs"));
        assert!(!is_near("a", "abcd"));
        // Short accepted words allow one edit, longer ones two.
        assert!(!is_near("asdf", "the"));
        assert!(!is_near("tre", "teh"));
        // Two edits on a 4-letter word: no.
        assert!(!is_near("hoes", "show"));
        assert!(!is_near("wd", "word"));
        assert!(is_near("cta", "cat"));
        assert!(is_near("wrld", "world"));
        assert!(is_near("wrlds", "worlds"));
        // Two edits on a longer word: yes; three: no.
        assert!(is_near("musuem", "museum"));
        assert!(is_near("beleivee", "believe"));
        assert!(!is_near("bxlixvxe", "believe"));
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
