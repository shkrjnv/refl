//! Deterministic automata over interned tokens (`docs/02-layer1-content.md`,
//! «Язык шаблонов ответов», «Проверки при сборке контента»).
//!
//! content-build compiles templates into minimal DFAs over a [`TokenTable`]
//! shared by the whole bundle; the runtime only executes them. Any token
//! missing from the table maps to [`TokenId::OTHER`], which no automaton has a
//! transition on. Template languages are finite, so every automaton is
//! acyclic.
//!
//! The product of two automata and its shortest witness give exact
//! intersection checks between accepted answers and error patterns.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

use super::template::{NormNode, NormPattern, NormSeq};

/// Interned token. Ids follow the lexicographic order of token strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenId(pub u32);

impl TokenId {
    /// Any token not in the table.
    pub const OTHER: Self = Self(0);
}

/// Sorted, deduplicated token strings; token `i` has id `i + 1`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct TokenTable(Vec<String>);

/// Converts a state or token index into its serialized `u32` form.
fn index_u32(i: usize) -> u32 {
    u32::try_from(i).expect("automata and token tables stay below u32::MAX entries")
}

impl TokenTable {
    /// # Panics
    /// If there are `u32::MAX` or more distinct tokens.
    pub fn new<'a>(tokens: impl IntoIterator<Item = &'a str>) -> Self {
        let set: BTreeSet<&str> = tokens.into_iter().collect();
        let table = Self(set.into_iter().map(String::from).collect());
        index_u32(table.0.len());
        table
    }

    /// The id of `token`, if interned.
    ///
    /// # Panics
    /// Never: tables hold fewer than `u32::MAX` tokens.
    pub fn get(&self, token: &str) -> Option<TokenId> {
        let i = self.0.binary_search_by(|t| t.as_str().cmp(token)).ok()?;
        Some(TokenId(index_u32(i + 1)))
    }

    /// The id of `token`, or [`TokenId::OTHER`].
    pub fn id(&self, token: &str) -> TokenId {
        self.get(token).unwrap_or(TokenId::OTHER)
    }

    /// The text of an interned token; `None` for OTHER or an unknown id.
    pub fn text(&self, id: TokenId) -> Option<&str> {
        let i = usize::try_from(id.0).ok()?.checked_sub(1)?;
        self.0.get(i).map(String::as_str)
    }

    /// Maps a normalized answer into ids.
    pub fn ids<S: AsRef<str>>(&self, tokens: &[S]) -> Vec<TokenId> {
        tokens.iter().map(|t| self.id(t.as_ref())).collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A serialized automaton or token table that breaks the invariants the
/// runtime relies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidAutomaton {
    UnsortedTokens,
    TooManyTokens,
    NoStates,
    UnsortedTransitions,
    TransitionOnOther,
    TargetOutOfRange,
}

impl fmt::Display for InvalidAutomaton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsortedTokens => "token table is not strictly sorted",
            Self::TooManyTokens => "token table has u32::MAX or more tokens",
            Self::NoStates => "automaton has no states",
            Self::UnsortedTransitions => "transitions are not strictly sorted by token",
            Self::TransitionOnOther => "transition on OTHER",
            Self::TargetOutOfRange => "transition target out of range",
        })
    }
}

impl core::error::Error for InvalidAutomaton {}

impl TryFrom<Vec<String>> for TokenTable {
    type Error = InvalidAutomaton;
    fn try_from(tokens: Vec<String>) -> Result<Self, InvalidAutomaton> {
        if !tokens.windows(2).all(|w| w[0] < w[1]) {
            return Err(InvalidAutomaton::UnsortedTokens);
        }
        if u32::try_from(tokens.len()).map_or(true, |n| n == u32::MAX) {
            return Err(InvalidAutomaton::TooManyTokens);
        }
        Ok(Self(tokens))
    }
}

impl From<TokenTable> for Vec<String> {
    fn from(t: TokenTable) -> Self {
        t.0
    }
}

/// One DFA state: transitions sorted by token, no duplicates. A missing
/// transition goes to the implicit dead state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub accepting: bool,
    pub next: Vec<(TokenId, u32)>,
}

/// Deterministic automaton; state 0 is the start state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<State>", into = "Vec<State>")]
pub struct Dfa {
    states: Vec<State>,
}

impl TryFrom<Vec<State>> for Dfa {
    type Error = InvalidAutomaton;
    fn try_from(states: Vec<State>) -> Result<Self, InvalidAutomaton> {
        if states.is_empty() {
            return Err(InvalidAutomaton::NoStates);
        }
        for s in &states {
            if !s.next.windows(2).all(|w| w[0].0 < w[1].0) {
                return Err(InvalidAutomaton::UnsortedTransitions);
            }
            if s.next.iter().any(|&(t, _)| t == TokenId::OTHER) {
                return Err(InvalidAutomaton::TransitionOnOther);
            }
            if s.next.iter().any(|&(_, to)| to as usize >= states.len()) {
                return Err(InvalidAutomaton::TargetOutOfRange);
            }
        }
        Ok(Self { states })
    }
}

impl From<Dfa> for Vec<State> {
    fn from(d: Dfa) -> Self {
        d.states
    }
}

/// Acceptance plus transitions to canonical targets: two states of an acyclic
/// DFA are equivalent exactly when their signatures are equal.
type Signature = (bool, Vec<(TokenId, u32)>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownToken(pub String);

impl fmt::Display for UnknownToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "token `{}` is not in the token table", self.0)
    }
}

impl core::error::Error for UnknownToken {}

#[derive(Default)]
struct Nfa {
    eps: Vec<Vec<usize>>,
    sym: Vec<Vec<(TokenId, usize)>>,
    accepting: BTreeSet<usize>,
}

impl Nfa {
    fn state(&mut self) -> usize {
        self.eps.push(Vec::new());
        self.sym.push(Vec::new());
        self.eps.len() - 1
    }

    fn seq(
        &mut self,
        seq: &NormSeq,
        from: usize,
        table: &TokenTable,
    ) -> Result<usize, UnknownToken> {
        let mut cur = from;
        for node in &seq.0 {
            cur = match node {
                NormNode::Tokens(tokens) => {
                    for t in tokens {
                        let id = table.get(t).ok_or_else(|| UnknownToken(t.clone()))?;
                        let s = self.state();
                        self.sym[cur].push((id, s));
                        cur = s;
                    }
                    cur
                }
                NormNode::Group(alts) => {
                    let end = self.state();
                    for alt in alts {
                        // Each alternative starts from its own state so that
                        // edges added inside it never leak into siblings.
                        let start = self.state();
                        self.eps[cur].push(start);
                        let e = self.seq(alt, start, table)?;
                        self.eps[e].push(end);
                    }
                    end
                }
                NormNode::Option(inner) => {
                    let start = self.state();
                    let end = self.state();
                    self.eps[cur].push(start);
                    self.eps[cur].push(end);
                    let e = self.seq(inner, start, table)?;
                    self.eps[e].push(end);
                    end
                }
            };
        }
        Ok(cur)
    }

    fn closure(&self, set: &mut BTreeSet<usize>) {
        let mut stack: Vec<usize> = set.iter().copied().collect();
        while let Some(s) = stack.pop() {
            for &t in &self.eps[s] {
                if set.insert(t) {
                    stack.push(t);
                }
            }
        }
    }
}

impl Dfa {
    /// Compiles the union of `patterns` into a minimal DFA. Equal languages
    /// give identical automata.
    ///
    /// # Errors
    /// A token of a pattern is missing from `table`.
    ///
    /// # Panics
    /// If the automaton would exceed `u32::MAX` states.
    pub fn compile(patterns: &[NormPattern], table: &TokenTable) -> Result<Self, UnknownToken> {
        let mut nfa = Nfa::default();
        let start = nfa.state();
        for p in patterns {
            let s = nfa.state();
            nfa.eps[start].push(s);
            let end = nfa.seq(&p.0, s, table)?;
            nfa.accepting.insert(end);
        }

        // Subset construction.
        let mut first = BTreeSet::from([start]);
        nfa.closure(&mut first);
        let mut ids: BTreeMap<BTreeSet<usize>, u32> = BTreeMap::new();
        let mut sets: Vec<BTreeSet<usize>> = Vec::new();
        let mut states: Vec<State> = Vec::new();
        ids.insert(first.clone(), 0);
        sets.push(first);
        let mut i = 0;
        while i < sets.len() {
            let mut moves: BTreeMap<TokenId, BTreeSet<usize>> = BTreeMap::new();
            for &s in &sets[i] {
                for &(t, to) in &nfa.sym[s] {
                    moves.entry(t).or_default().insert(to);
                }
            }
            let mut next = Vec::with_capacity(moves.len());
            for (t, mut target) in moves {
                nfa.closure(&mut target);
                let id = if let Some(&id) = ids.get(&target) {
                    id
                } else {
                    let id = index_u32(sets.len());
                    ids.insert(target.clone(), id);
                    sets.push(target);
                    id
                };
                next.push((t, id));
            }
            let accepting = sets[i].iter().any(|s| nfa.accepting.contains(s));
            states.push(State { accepting, next });
            i += 1;
        }
        Ok(Self { states }.minimize())
    }

    /// Merges equivalent states. The automaton is acyclic, so two states are
    /// equivalent exactly when they agree on acceptance and their transitions
    /// lead to equivalent states; canonical ids are assigned bottom-up.
    fn minimize(self) -> Self {
        fn canon(
            s: u32,
            states: &[State],
            memo: &mut BTreeMap<u32, u32>,
            sigs: &mut BTreeMap<Signature, u32>,
            out: &mut Vec<State>,
        ) -> u32 {
            if let Some(&c) = memo.get(&s) {
                return c;
            }
            let st = &states[s as usize];
            let next: Vec<(TokenId, u32)> = st
                .next
                .iter()
                .map(|&(t, to)| (t, canon(to, states, memo, sigs, out)))
                .collect();
            let sig = (st.accepting, next);
            let c = if let Some(&c) = sigs.get(&sig) {
                c
            } else {
                let c = index_u32(out.len());
                out.push(State {
                    accepting: sig.0,
                    next: sig.1.clone(),
                });
                sigs.insert(sig, c);
                c
            };
            memo.insert(s, c);
            c
        }
        let mut merged = Vec::new();
        let root = canon(
            0,
            &self.states,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut merged,
        );
        Self::renumber(&merged, root)
    }

    /// Breadth-first renumbering from `start` in token order, so the layout
    /// depends only on the language.
    fn renumber(states: &[State], start: u32) -> Self {
        let mut order: BTreeMap<u32, u32> = BTreeMap::from([(start, 0)]);
        let mut queue = VecDeque::from([start]);
        let mut visit = Vec::new();
        while let Some(s) = queue.pop_front() {
            visit.push(s);
            for &(_, to) in &states[s as usize].next {
                if !order.contains_key(&to) {
                    let n = index_u32(order.len());
                    order.insert(to, n);
                    queue.push_back(to);
                }
            }
        }
        let states = visit
            .iter()
            .map(|&s| {
                let st = &states[s as usize];
                State {
                    accepting: st.accepting,
                    next: st.next.iter().map(|&(t, to)| (t, order[&to])).collect(),
                }
            })
            .collect();
        Self { states }
    }

    fn step(&self, state: u32, token: TokenId) -> Option<u32> {
        let next = &self.states[state as usize].next;
        next.binary_search_by_key(&token, |&(t, _)| t)
            .ok()
            .map(|i| next[i].1)
    }

    /// Runs the automaton on an interned answer.
    pub fn accepts(&self, tokens: &[TokenId]) -> bool {
        let mut s = 0;
        for &t in tokens {
            match self.step(s, t) {
                Some(n) => s = n,
                None => return false,
            }
        }
        self.states[s as usize].accepting
    }

    pub fn states(&self) -> &[State] {
        &self.states
    }

    /// Product automaton accepting the intersection of both languages.
    /// Only pairs reachable from the start are built.
    ///
    /// # Panics
    /// If the product would exceed `u32::MAX` states.
    #[must_use]
    pub fn product(&self, other: &Self) -> Self {
        let mut ids: BTreeMap<(u32, u32), u32> = BTreeMap::from([((0, 0), 0)]);
        let mut pairs = alloc::vec![(0u32, 0u32)];
        let mut states = Vec::new();
        let mut current = 0;
        while current < pairs.len() {
            let (left, right) = pairs[current];
            let (sl, sr) = (&self.states[left as usize], &other.states[right as usize]);
            let mut next = Vec::new();
            // Merge-join of two sorted transition lists.
            let (mut il, mut ir) = (0, 0);
            while il < sl.next.len() && ir < sr.next.len() {
                let ((tl, nl), (tr, nr)) = (sl.next[il], sr.next[ir]);
                match tl.cmp(&tr) {
                    core::cmp::Ordering::Less => il += 1,
                    core::cmp::Ordering::Greater => ir += 1,
                    core::cmp::Ordering::Equal => {
                        let id = *ids.entry((nl, nr)).or_insert_with(|| {
                            pairs.push((nl, nr));
                            index_u32(pairs.len() - 1)
                        });
                        next.push((tl, id));
                        il += 1;
                        ir += 1;
                    }
                }
            }
            states.push(State {
                accepting: sl.accepting && sr.accepting,
                next,
            });
            current += 1;
        }
        Self { states }
    }

    /// Shortest accepted token sequence; among equally short ones, the
    /// lexicographically smallest (token ids follow string order). `None`
    /// when the language is empty.
    pub fn shortest(&self) -> Option<Vec<TokenId>> {
        let mut parent: BTreeMap<u32, (u32, TokenId)> = BTreeMap::new();
        let mut seen = BTreeSet::from([0u32]);
        let mut queue = VecDeque::from([0u32]);
        while let Some(s) = queue.pop_front() {
            if self.states[s as usize].accepting {
                let mut path = Vec::new();
                let mut cur = s;
                while let Some(&(p, t)) = parent.get(&cur) {
                    path.push(t);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            for &(t, to) in &self.states[s as usize].next {
                if seen.insert(to) {
                    parent.insert(to, (s, t));
                    queue.push_back(to);
                }
            }
        }
        None
    }
}

/// Shortest common string of two automata, if their languages intersect.
pub fn intersection_witness(a: &Dfa, b: &Dfa) -> Option<Vec<TokenId>> {
    a.product(b).shortest()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::normalize::{NormalizeOptions, normalize};
    use crate::answer::template::Pattern;

    fn compile(srcs: &[&str]) -> (Dfa, TokenTable) {
        let opts = NormalizeOptions::default();
        let pats: Vec<NormPattern> = srcs
            .iter()
            .map(|s| Pattern::parse(s).unwrap().normalize(opts).unwrap())
            .collect();
        let table = TokenTable::new(pats.iter().flat_map(NormPattern::tokens));
        (Dfa::compile(&pats, &table).unwrap(), table)
    }

    fn run(dfa: &Dfa, table: &TokenTable, answer: &str) -> bool {
        dfa.accepts(&table.ids(&normalize(answer, NormalizeOptions::default())))
    }

    #[test]
    fn accepts_exactly_the_language() {
        let (dfa, t) = compile(&["(I have|I've) [already] been there"]);
        for ok in [
            "I have been there",
            "I've already been there.",
            "i HAVE ALREADY been there",
        ] {
            assert!(run(&dfa, &t, ok), "{ok}");
        }
        for bad in [
            "I have been",
            "I been there",
            "I have have been there",
            "I've seen there",
            "",
        ] {
            assert!(!run(&dfa, &t, bad), "{bad}");
        }
    }

    #[test]
    fn equal_languages_compile_identically() {
        let (a, _) = compile(&["(a b|a c)"]);
        let (b, _) = compile(&["a (b|c)"]);
        assert_eq!(a, b);
        let (c, _) = compile(&["x [y]", "x y"]);
        let (d, _) = compile(&["x [y]"]);
        assert_eq!(c, d);
        // a (b|c) minimal: start, after a, accept.
        assert_eq!(a.states().len(), 3);
    }

    /// Compiles two pattern sets over one shared token table and returns the
    /// witness of their intersection as text.
    fn witness(left: &[&str], right: &[&str]) -> Option<Vec<String>> {
        let opts = NormalizeOptions::default();
        let norm = |srcs: &[&str]| -> Vec<NormPattern> {
            srcs.iter()
                .map(|s| Pattern::parse(s).unwrap().normalize(opts).unwrap())
                .collect()
        };
        let (l, r) = (norm(left), norm(right));
        let table = TokenTable::new(l.iter().chain(&r).flat_map(NormPattern::tokens));
        let (dl, dr) = (
            Dfa::compile(&l, &table).unwrap(),
            Dfa::compile(&r, &table).unwrap(),
        );
        intersection_witness(&dl, &dr).map(|w| {
            w.iter()
                .map(|&id| String::from(table.text(id).unwrap()))
                .collect()
        })
    }

    #[test]
    fn product_witness_is_shortest_common_string() {
        assert_eq!(
            witness(
                &["[the] (big|small) [red] dog"],
                &["[a] (big|small) red dog"]
            )
            .unwrap(),
            ["big", "red", "dog"]
        );
        assert_eq!(witness(&["a b", "c"], &["a c", "b"]), None);
        // Shortest first, then lexicographically smallest.
        assert_eq!(
            witness(&["(z|y) x", "q r s"], &["(y|z) x", "q r s"]).unwrap(),
            ["y", "x"]
        );
    }

    #[test]
    fn unknown_tokens_map_to_other() {
        let (dfa, t) = compile(&["a"]);
        assert_eq!(t.id("zzz"), TokenId::OTHER);
        assert!(!dfa.accepts(&[TokenId::OTHER]));
        assert_eq!(t.text(TokenId::OTHER), None);
        assert_eq!(t.text(t.id("a")), Some("a"));
    }
}
