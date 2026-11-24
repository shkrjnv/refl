//! Property tests for the answer template language and its automata: the
//! parser round-trips through the canonical text and never panics; compiled
//! DFAs accept exactly the pattern's (finite) language; the product's witness
//! is the shortest, then lexicographically smallest, common string, and the
//! product is empty exactly when the languages are disjoint.

use std::collections::BTreeSet;

use proptest::prelude::*;
use refl_core::answer::dfa::{Dfa, State, TokenId, TokenTable, intersection_witness};
use refl_core::answer::normalize::NormalizeOptions;
use refl_core::answer::template::{MAX_DEPTH, Node, NormPattern, Pattern, Seq};

/// Patterns as the parser produces them, from words drawn by `word`.
fn seq(word: BoxedStrategy<String>, depth: usize) -> BoxedStrategy<Seq> {
    let leaf = word.clone().prop_map(Node::Word);
    let node = if depth == 0 {
        leaf.boxed()
    } else {
        let inner = seq(word, depth - 1);
        prop_oneof![
            3 => leaf,
            1 => proptest::collection::vec(inner.clone(), 2..4).prop_map(Node::Group),
            1 => inner.prop_map(Node::Option),
        ]
        .boxed()
    };
    proptest::collection::vec(node, 1..4).prop_map(Seq).boxed()
}

fn pattern(word: BoxedStrategy<String>, depth: usize) -> impl Strategy<Value = Pattern> {
    seq(word, depth).prop_map(Pattern)
}

/// Words with specials (escaped on output), punctuation and apostrophes.
fn any_word() -> BoxedStrategy<String> {
    "[a-c()\\[\\]|\\\\.,'-]{1,4}".boxed()
}

/// One-token words over a three-letter alphabet, so languages overlap often.
fn small_word() -> BoxedStrategy<String> {
    prop_oneof![Just("a"), Just("b"), Just("c")]
        .prop_map(String::from)
        .boxed()
}

type Lang = BTreeSet<Vec<String>>;

fn lang_seq(seq: &Seq) -> Lang {
    let mut acc: Lang = BTreeSet::from([Vec::new()]);
    for node in &seq.0 {
        let part = match node {
            Node::Word(w) => BTreeSet::from([vec![w.clone()]]),
            Node::Group(alts) => alts.iter().flat_map(lang_seq).collect(),
            Node::Option(inner) => {
                let mut l = lang_seq(inner);
                l.insert(Vec::new());
                l
            }
        };
        acc = acc
            .iter()
            .flat_map(|prefix| {
                part.iter().map(move |suffix| {
                    let mut s = prefix.clone();
                    s.extend(suffix.iter().cloned());
                    s
                })
            })
            .collect();
    }
    acc
}

/// All strings of an acyclic DFA, as text.
fn dfa_lang(dfa: &Dfa, table: &TokenTable) -> Lang {
    fn walk(states: &[State], s: u32, path: &mut Vec<TokenId>, table: &TokenTable, out: &mut Lang) {
        let st = &states[s as usize];
        if st.accepting {
            out.insert(
                path.iter()
                    .map(|&t| table.text(t).unwrap().to_owned())
                    .collect(),
            );
        }
        for &(t, to) in &st.next {
            path.push(t);
            walk(states, to, path, table, out);
            path.pop();
        }
    }
    let mut out = BTreeSet::new();
    walk(dfa.states(), 0, &mut Vec::new(), table, &mut out);
    out
}

fn normalized(patterns: &[Pattern]) -> Option<Vec<NormPattern>> {
    patterns
        .iter()
        .map(|p| p.normalize(NormalizeOptions::default()).ok())
        .collect()
}

/// A union of 1–3 small patterns that is valid after normalization.
fn pattern_set() -> impl Strategy<Value = (Vec<Pattern>, Vec<NormPattern>)> {
    proptest::collection::vec(pattern(small_word(), 3), 1..4).prop_filter_map(
        "pattern can be empty",
        |ps| {
            let n = normalized(&ps)?;
            Some((ps, n))
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn parse_display_round_trip(p in pattern(any_word(), MAX_DEPTH)) {
        let text = p.to_string();
        prop_assert_eq!(Pattern::parse(&text), Ok(p), "{}", text);
    }

    #[test]
    fn parser_never_panics(src in any::<String>()) {
        if let Ok(p) = Pattern::parse(&src) {
            prop_assert_eq!(Pattern::parse(&p.to_string()), Ok(p));
        }
    }

    #[test]
    fn parser_never_panics_on_template_like_input(src in "[ab ()\\[\\]|\\\\.]{0,24}") {
        if let Ok(p) = Pattern::parse(&src) {
            prop_assert_eq!(Pattern::parse(&p.to_string()), Ok(p));
        }
    }

    #[test]
    fn dfa_accepts_exactly_the_language((ps, norm) in pattern_set()) {
        let table = TokenTable::new(norm.iter().flat_map(NormPattern::tokens));
        let dfa = Dfa::compile(&norm, &table).unwrap();
        let expected: Lang = ps.iter().flat_map(|p| lang_seq(&p.0)).collect();
        prop_assert_eq!(dfa_lang(&dfa, &table), expected.clone());
        for s in &expected {
            prop_assert!(dfa.accepts(&table.ids(s)));
        }
        // The union does not depend on the order of its patterns.
        let mut rev = norm.clone();
        rev.reverse();
        prop_assert_eq!(Dfa::compile(&rev, &table).unwrap(), dfa);
    }

    #[test]
    fn product_witness_matches_brute_force(
        (pa, na) in pattern_set(),
        (pb, nb) in pattern_set(),
    ) {
        let table = TokenTable::new(na.iter().chain(&nb).flat_map(NormPattern::tokens));
        let (da, db) = (Dfa::compile(&na, &table).unwrap(), Dfa::compile(&nb, &table).unwrap());
        let la: Lang = pa.iter().flat_map(|p| lang_seq(&p.0)).collect();
        let lb: Lang = pb.iter().flat_map(|p| lang_seq(&p.0)).collect();
        let common: Vec<&Vec<String>> = la.intersection(&lb).collect();
        let best = common.iter().min_by(|x, y| x.len().cmp(&y.len()).then(x.cmp(y)));
        match intersection_witness(&da, &db) {
            None => prop_assert!(common.is_empty()),
            Some(w) => {
                prop_assert!(da.accepts(&w) && db.accepts(&w));
                let text: Vec<String> =
                    w.iter().map(|&t| table.text(t).unwrap().to_owned()).collect();
                prop_assert!(la.contains(&text) && lb.contains(&text));
                prop_assert_eq!(Some(&&text), best);
            }
        }
    }
}
