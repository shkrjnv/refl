//! Diagnostics and the build report.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Fails the build.
    Error,
    /// Reported; the build still succeeds.
    Warning,
}

/// One kind of check. Tests match on codes, not on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    /// `nickel export` failed.
    Nickel,
    /// Exported JSON does not match the refl-core types.
    Schema,
    DuplicateId,
    UnknownReference,
    /// A live misconception lacks a short explanation in ru or en.
    MissingFeedback,
    /// An instruction of a live item lacks ru or en text.
    InstructionText,
    RequiresCycle,
    TemplateSyntax,
    /// A pattern, alternative or option can be empty after normalization.
    TemplateEmpty,
    NoAccepted,
    AnswerNotAccepted,
    AcceptedMatchesError,
    ErrorPatternsOverlap,
    NoPrimaryKc,
    NoQualityRule,
    TooFewDistractors,
    DistractorAccepted,
    DuplicateDistractor,
    TooFewErrorPatterns,
    MissingAudio,
}

impl Code {
    /// Every check; tests require a negative fixture named after each one.
    pub const ALL: [Self; 20] = [
        Self::Nickel,
        Self::Schema,
        Self::DuplicateId,
        Self::UnknownReference,
        Self::MissingFeedback,
        Self::InstructionText,
        Self::RequiresCycle,
        Self::TemplateSyntax,
        Self::TemplateEmpty,
        Self::NoAccepted,
        Self::AnswerNotAccepted,
        Self::AcceptedMatchesError,
        Self::ErrorPatternsOverlap,
        Self::NoPrimaryKc,
        Self::NoQualityRule,
        Self::TooFewDistractors,
        Self::DistractorAccepted,
        Self::DuplicateDistractor,
        Self::TooFewErrorPatterns,
        Self::MissingAudio,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nickel => "nickel",
            Self::Schema => "schema",
            Self::DuplicateId => "duplicate-id",
            Self::UnknownReference => "unknown-reference",
            Self::MissingFeedback => "missing-feedback",
            Self::InstructionText => "instruction-text",
            Self::RequiresCycle => "requires-cycle",
            Self::TemplateSyntax => "template-syntax",
            Self::TemplateEmpty => "template-empty",
            Self::NoAccepted => "no-accepted",
            Self::AnswerNotAccepted => "answer-not-accepted",
            Self::AcceptedMatchesError => "accepted-matches-error",
            Self::ErrorPatternsOverlap => "error-patterns-overlap",
            Self::NoPrimaryKc => "no-primary-kc",
            Self::NoQualityRule => "no-quality-rule",
            Self::TooFewDistractors => "too-few-distractors",
            Self::DistractorAccepted => "distractor-accepted",
            Self::DuplicateDistractor => "duplicate-distractor",
            Self::TooFewErrorPatterns => "too-few-error-patterns",
            Self::MissingAudio => "missing-audio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Code,
    /// Relative to the content root.
    pub file: PathBuf,
    /// The entity, e.g. "item `art.fm.gap.cat`".
    pub subject: Option<String>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{sev}[{}] {}", self.code.as_str(), self.file.display())?;
        if let Some(s) = &self.subject {
            write!(f, ": {s}")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// A KC that stays draft and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftKc {
    pub kc: String,
    pub file: PathBuf,
    pub missing: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub draft: Vec<DraftKc>,
    pub live: Vec<String>,
}

impl Report {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for d in &self.diagnostics {
            writeln!(f, "{d}")?;
        }
        for kc in &self.live {
            writeln!(f, "live  {kc}")?;
        }
        for d in &self.draft {
            writeln!(
                f,
                "draft {} ({}): {}",
                d.kc,
                d.file.display(),
                d.missing.join("; ")
            )?;
        }
        write!(
            f,
            "{} error(s), {} warning(s), {} live KC(s), {} draft KC(s)",
            self.count(Severity::Error),
            self.count(Severity::Warning),
            self.live.len(),
            self.draft.len()
        )
    }
}
