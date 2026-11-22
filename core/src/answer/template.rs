//! Answer template language (`docs/02-layer1-content.md`, «Язык шаблонов ответов»):
//!
//! ```text
//! pattern := seq
//! seq     := elem*
//! elem    := word | group | option
//! group   := "(" seq ("|" seq)+ ")"
//! option  := "[" seq "]"
//! word    := (char | "\" special)+
//! special := "(" | ")" | "[" | "]" | "|" | "\"
//! ```
//!
//! Alternatives are sequences of words, not substrings: `walk(s|ed)` is an
//! error. Empty alternatives, empty options, empty patterns and nesting deeper
//! than [`MAX_DEPTH`] are errors. There is no repetition, so every pattern
//! denotes a finite language.
//!
//! Parsing is independent of item flags. [`Pattern::normalize`] applies the
//! item's normalization to every word and yields a [`NormPattern`] over tokens.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::normalize::{NormalizeOptions, normalize};

/// Maximum nesting of groups and options.
pub const MAX_DEPTH: usize = 4;

const SPECIALS: [char; 6] = ['(', ')', '[', ']', '|', '\\'];

/// A sequence of elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seq(pub Vec<Node>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Literal text between separators, escapes resolved.
    Word(String),
    /// Two or more alternatives.
    Group(Vec<Seq>),
    /// Optional part.
    Option(Seq),
}

/// A parsed template, before normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern(pub Seq);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based character position in the pattern source.
    pub column: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    EmptyPattern,
    EmptyAlternative,
    EmptyOption,
    /// `(a)`: a group needs at least two alternatives.
    SingleAlternative,
    UnclosedGroup,
    UnclosedOption,
    Unexpected(char),
    /// `\` followed by a character that is not special, or by nothing.
    BadEscape,
    TooDeep,
    /// A word touches a group or option without whitespace (`walk(s|ed)`),
    /// or two groups or options touch.
    Attached,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match &self.kind {
            ParseErrorKind::EmptyPattern => "pattern is empty",
            ParseErrorKind::EmptyAlternative => "empty alternative; use [..] for an optional part",
            ParseErrorKind::EmptyOption => "empty option []",
            ParseErrorKind::SingleAlternative => {
                "group has a single alternative; drop the parentheses or use [..]"
            }
            ParseErrorKind::UnclosedGroup => "unclosed '('",
            ParseErrorKind::UnclosedOption => "unclosed '['",
            ParseErrorKind::Unexpected(c) => {
                return write!(f, "column {}: unexpected '{c}'", self.column);
            }
            ParseErrorKind::BadEscape => "'\\' must be followed by one of ( ) [ ] | \\",
            ParseErrorKind::TooDeep => "nesting deeper than 4",
            ParseErrorKind::Attached => {
                "alternatives are whole words: separate words from groups and options \
                 with spaces, e.g. (walks|walked) instead of walk(s|ed)"
            }
        };
        write!(f, "column {}: {what}", self.column)
    }
}

impl core::error::Error for ParseError {}

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
}

/// What the previous element of a sequence was, if nothing separated it from
/// the current position.
#[derive(Clone, Copy)]
enum Prev {
    None,
    /// A word; `punct_only` if it has no letters or digits.
    Word {
        punct_only: bool,
    },
    Bracketed,
}

impl Parser<'_> {
    fn err(column0: usize, kind: ParseErrorKind) -> ParseError {
        ParseError {
            column: column0 + 1,
            kind,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Parses a sequence up to (not including) `)`, `|`, `]` or the end.
    fn seq(&mut self, depth: usize) -> Result<Seq, ParseError> {
        let mut nodes = Vec::new();
        let mut prev = Prev::None;
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += 1;
                prev = Prev::None;
                continue;
            }
            let start = self.pos;
            match c {
                ')' | '|' | ']' => break,
                '(' | '[' => {
                    if matches!(prev, Prev::Bracketed | Prev::Word { punct_only: false }) {
                        return Err(Self::err(start, ParseErrorKind::Attached));
                    }
                    if depth == MAX_DEPTH {
                        return Err(Self::err(start, ParseErrorKind::TooDeep));
                    }
                    self.pos += 1;
                    nodes.push(if c == '(' {
                        self.group(start, depth + 1)?
                    } else {
                        self.option(start, depth + 1)?
                    });
                    prev = Prev::Bracketed;
                }
                _ => {
                    let word = self.word()?;
                    let punct_only = !word.chars().any(char::is_alphanumeric);
                    if matches!(prev, Prev::Bracketed) && !punct_only {
                        return Err(Self::err(start, ParseErrorKind::Attached));
                    }
                    nodes.push(Node::Word(word));
                    prev = Prev::Word { punct_only };
                }
            }
        }
        Ok(Seq(nodes))
    }

    fn group(&mut self, open: usize, depth: usize) -> Result<Node, ParseError> {
        let mut alts = Vec::new();
        loop {
            let alt_start = self.pos;
            let alt = self.seq(depth)?;
            if alt.0.is_empty() {
                return Err(Self::err(alt_start, ParseErrorKind::EmptyAlternative));
            }
            alts.push(alt);
            match self.peek() {
                Some('|') => self.pos += 1,
                Some(')') => {
                    self.pos += 1;
                    break;
                }
                Some(c) => return Err(Self::err(self.pos, ParseErrorKind::Unexpected(c))),
                None => return Err(Self::err(open, ParseErrorKind::UnclosedGroup)),
            }
        }
        if alts.len() < 2 {
            return Err(Self::err(open, ParseErrorKind::SingleAlternative));
        }
        Ok(Node::Group(alts))
    }

    fn option(&mut self, open: usize, depth: usize) -> Result<Node, ParseError> {
        let inner = self.seq(depth)?;
        match self.peek() {
            Some(']') => self.pos += 1,
            Some(c) => return Err(Self::err(self.pos, ParseErrorKind::Unexpected(c))),
            None => return Err(Self::err(open, ParseErrorKind::UnclosedOption)),
        }
        if inner.0.is_empty() {
            return Err(Self::err(open, ParseErrorKind::EmptyOption));
        }
        Ok(Node::Option(inner))
    }

    fn word(&mut self) -> Result<String, ParseError> {
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || (SPECIALS.contains(&c) && c != '\\') {
                break;
            }
            if c == '\\' {
                match self.chars.get(self.pos + 1) {
                    Some(&e) if SPECIALS.contains(&e) => {
                        word.push(e);
                        self.pos += 2;
                    }
                    _ => return Err(Self::err(self.pos, ParseErrorKind::BadEscape)),
                }
            } else {
                word.push(c);
                self.pos += 1;
            }
        }
        Ok(word)
    }
}

impl Pattern {
    /// Parses a template.
    ///
    /// # Errors
    /// Any violation of the grammar or of the rules in the module docs.
    pub fn parse(src: &str) -> Result<Self, ParseError> {
        let chars: Vec<char> = src.chars().collect();
        let mut p = Parser {
            chars: &chars,
            pos: 0,
        };
        let seq = p.seq(0)?;
        if let Some(c) = p.peek() {
            return Err(Parser::err(p.pos, ParseErrorKind::Unexpected(c)));
        }
        if seq.0.is_empty() {
            return Err(Parser::err(0, ParseErrorKind::EmptyPattern));
        }
        Ok(Self(seq))
    }

    /// Applies the item's normalization to every word.
    ///
    /// # Errors
    /// The pattern, an alternative or an option can match the empty answer
    /// after normalization (e.g. `(a|.)` with punctuation ignored, or `[a]`
    /// on its own).
    pub fn normalize(&self, opts: NormalizeOptions) -> Result<NormPattern, NormalizeError> {
        let seq = norm_seq(&self.0, opts)?;
        if nullable(&seq) {
            return Err(NormalizeError::AcceptsEmpty);
        }
        Ok(NormPattern(seq))
    }
}

fn norm_seq(seq: &Seq, opts: NormalizeOptions) -> Result<NormSeq, NormalizeError> {
    let mut out = Vec::new();
    for node in &seq.0 {
        match node {
            Node::Word(w) => {
                let tokens = normalize(w, opts);
                if !tokens.is_empty() {
                    out.push(NormNode::Tokens(tokens));
                }
            }
            Node::Group(alts) => {
                let mut nalts = Vec::with_capacity(alts.len());
                for alt in alts {
                    let n = norm_seq(alt, opts)?;
                    if nullable(&n) {
                        return Err(NormalizeError::EmptyAlternative(DisplaySeq(alt).into()));
                    }
                    nalts.push(n);
                }
                out.push(NormNode::Group(nalts));
            }
            Node::Option(inner) => {
                let n = norm_seq(inner, opts)?;
                if nullable(&n) {
                    return Err(NormalizeError::EmptyOption(DisplaySeq(inner).into()));
                }
                out.push(NormNode::Option(n));
            }
        }
    }
    Ok(NormSeq(out))
}

fn nullable(seq: &NormSeq) -> bool {
    seq.0.iter().all(|n| match n {
        NormNode::Tokens(t) => t.is_empty(),
        NormNode::Group(alts) => alts.iter().any(nullable),
        NormNode::Option(_) => true,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    /// The whole pattern matches the empty answer.
    AcceptsEmpty,
    /// This alternative (source text) can be empty after normalization.
    EmptyAlternative(String),
    /// This option's content (source text) can be empty after normalization.
    EmptyOption(String),
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptsEmpty => f.write_str("pattern accepts an empty answer"),
            Self::EmptyAlternative(a) => write!(
                f,
                "alternative `{a}` can be empty after normalization; use [..] for an optional part"
            ),
            Self::EmptyOption(o) => {
                write!(f, "option `[{o}]` can be empty after normalization")
            }
        }
    }
}

impl core::error::Error for NormalizeError {}

/// A sequence of normalized elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormSeq(pub Vec<NormNode>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormNode {
    /// Tokens of one source word (non-empty).
    Tokens(Vec<String>),
    Group(Vec<NormSeq>),
    Option(NormSeq),
}

/// A template after normalization: words replaced by token sequences.
/// Never matches the empty answer; no alternative or option can be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormPattern(pub NormSeq);

impl NormPattern {
    /// Every token the pattern mentions, with repetitions.
    pub fn tokens(&self) -> Vec<&str> {
        fn walk<'a>(seq: &'a NormSeq, out: &mut Vec<&'a str>) {
            for n in &seq.0 {
                match n {
                    NormNode::Tokens(t) => out.extend(t.iter().map(String::as_str)),
                    NormNode::Group(alts) => alts.iter().for_each(|a| walk(a, out)),
                    NormNode::Option(s) => walk(s, out),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.0, &mut out);
        out
    }
}

struct DisplaySeq<'a>(&'a Seq);

impl From<DisplaySeq<'_>> for String {
    fn from(d: DisplaySeq<'_>) -> String {
        alloc::format!("{d}")
    }
}

impl fmt::Display for DisplaySeq<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, node) in self.0.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            match node {
                Node::Word(w) => {
                    for c in w.chars() {
                        if SPECIALS.contains(&c) {
                            f.write_str("\\")?;
                        }
                        write!(f, "{c}")?;
                    }
                }
                Node::Group(alts) => {
                    f.write_str("(")?;
                    for (j, alt) in alts.iter().enumerate() {
                        if j > 0 {
                            f.write_str("|")?;
                        }
                        write!(f, "{}", DisplaySeq(alt))?;
                    }
                    f.write_str(")")?;
                }
                Node::Option(inner) => write!(f, "[{}]", DisplaySeq(inner))?,
            }
        }
        Ok(())
    }
}

/// Canonical source text: elements separated by single spaces, specials escaped.
/// `Pattern::parse(&p.to_string()) == Ok(p)` for every parsed pattern.
impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        DisplaySeq(&self.0).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Contractions;
    use alloc::string::ToString;

    fn kind(src: &str) -> ParseErrorKind {
        Pattern::parse(src).unwrap_err().kind
    }

    #[test]
    fn parses_the_docs_examples() {
        for src in [
            "(I have|I've) been",
            "[already]",
            "(a|an) apple",
            "(walks|walked)",
            "have went",
            "Hello, (John|Mary).",
            "(dogs|cats)'",
            "f\\(x\\) \\| \\\\",
        ] {
            let p = Pattern::parse(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(Pattern::parse(&p.to_string()), Ok(p), "{src}");
        }
    }

    #[test]
    fn rejects_bad_patterns() {
        assert_eq!(kind(""), ParseErrorKind::EmptyPattern);
        assert_eq!(kind("   "), ParseErrorKind::EmptyPattern);
        assert_eq!(kind("(a|)"), ParseErrorKind::EmptyAlternative);
        assert_eq!(kind("(|a)"), ParseErrorKind::EmptyAlternative);
        assert_eq!(kind("[ ]"), ParseErrorKind::EmptyOption);
        assert_eq!(kind("(a)"), ParseErrorKind::SingleAlternative);
        assert_eq!(kind("(a|b"), ParseErrorKind::UnclosedGroup);
        assert_eq!(kind("[a"), ParseErrorKind::UnclosedOption);
        assert_eq!(kind("a)"), ParseErrorKind::Unexpected(')'));
        assert_eq!(kind("a | b"), ParseErrorKind::Unexpected('|'));
        assert_eq!(kind("(a|b]"), ParseErrorKind::Unexpected(']'));
        assert_eq!(kind("a\\b"), ParseErrorKind::BadEscape);
        assert_eq!(kind("a\\"), ParseErrorKind::BadEscape);
        assert_eq!(kind("walk(s|ed)"), ParseErrorKind::Attached);
        assert_eq!(kind("(a|b)c"), ParseErrorKind::Attached);
        assert_eq!(kind("(John|Mary)'s"), ParseErrorKind::Attached);
        assert_eq!(kind("(a|b)[c]"), ParseErrorKind::Attached);
        assert_eq!(kind("[a][b]"), ParseErrorKind::Attached);
        assert_eq!(Pattern::parse("a (b|c)d").unwrap_err().column, 8);
        assert_eq!(Pattern::parse("(a|b").unwrap_err().column, 1);
    }

    #[test]
    fn nesting_depth_is_limited() {
        assert!(Pattern::parse("(a|(b|(c|[d])))").is_ok());
        assert_eq!(kind("(a|(b|(c|[d [e]])))"), ParseErrorKind::TooDeep);
    }

    #[test]
    fn normalization_rejects_patterns_that_can_be_empty() {
        let d = NormalizeOptions::default();
        let p = |s| Pattern::parse(s).unwrap();
        assert_eq!(p("[a]").normalize(d), Err(NormalizeError::AcceptsEmpty));
        assert_eq!(p(". ,").normalize(d), Err(NormalizeError::AcceptsEmpty));
        assert!(matches!(
            p("x (a|.)").normalize(d),
            Err(NormalizeError::EmptyAlternative(_))
        ));
        assert!(matches!(
            p("x (a|[b])").normalize(d),
            Err(NormalizeError::EmptyAlternative(_))
        ));
        assert!(matches!(
            p("x [!]").normalize(d),
            Err(NormalizeError::EmptyOption(_))
        ));
        let ps = NormalizeOptions {
            punctuation_sensitive: true,
            ..d
        };
        assert!(p("x (a|.)").normalize(ps).is_ok());
        let eq = NormalizeOptions {
            contractions: Contractions::Equivalent,
            ..d
        };
        assert_eq!(
            p("(I don't|I've) [Bob's]").normalize(eq).unwrap().tokens(),
            ["i", "do", "not", "i", "have", "bob's"]
        );
    }
}
