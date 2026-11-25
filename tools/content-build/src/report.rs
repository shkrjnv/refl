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
    /// A KC's level key is not in its subject's scale.
    UnknownLevel,
    /// A KC's domain is not in its subject's list.
    UnknownDomain,
    /// A KC, misconception or family id lacks its subject prefix.
    SubjectPrefix,
    /// A `requires` edge joins KCs of different subjects.
    RequiresAcrossSubjects,
    /// A scale without levels, or with `step_logit <= 0`.
    InvalidScale,
    /// A slot KC or a spelling KC of another subject.
    KcOutsideSubject,
    /// The model projection of a published item version differs from its
    /// entry in the lock file, and the version was not bumped.
    ProjectionChanged,
    /// The lock file cannot be trusted: unreadable, unsorted, or its digest
    /// does not match (an entry was removed or changed by hand).
    LockCorrupt,
    /// With `--locked`: the build would append entries to the lock file.
    LockOutdated,
    /// `review = reviewed` without `by`: review is a person, so the status
    /// comes with a name (docs/02, «Вычитка и показ»).
    ReviewedWithoutReviewer,
    /// A `reviewed` record without a hash, or whose hash is not that of the
    /// text it covers: the text changed after the review.
    ReviewedTextChanged,
    /// A `reviews.lock` line whose record or review group no longer exists.
    OrphanReview,
    /// Every session-available item of a KC has another KC as secondary, with
    /// no `requires` path to it.
    ImpliedPrerequisite,
    /// A `supports` edge between KCs already joined by a `requires` path.
    RedundantSupports,
    /// A slot's secondary KC requires a primary KC of the same slot.
    SecondaryDownstream,
    /// Items of one family have different instructions: fluency reads the
    /// instruction of the family (docs/02, «Таблица семей»).
    FamilyInstruction,
}

impl Code {
    /// Every check; tests require a negative fixture named after each one.
    pub const ALL: [Self; 36] = [
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
        Self::UnknownLevel,
        Self::UnknownDomain,
        Self::SubjectPrefix,
        Self::RequiresAcrossSubjects,
        Self::InvalidScale,
        Self::KcOutsideSubject,
        Self::ProjectionChanged,
        Self::LockCorrupt,
        Self::LockOutdated,
        Self::ReviewedWithoutReviewer,
        Self::ReviewedTextChanged,
        Self::OrphanReview,
        Self::ImpliedPrerequisite,
        Self::RedundantSupports,
        Self::SecondaryDownstream,
        Self::FamilyInstruction,
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
            Self::UnknownLevel => "unknown-level",
            Self::UnknownDomain => "unknown-domain",
            Self::SubjectPrefix => "subject-prefix",
            Self::RequiresAcrossSubjects => "requires-across-subjects",
            Self::InvalidScale => "invalid-scale",
            Self::KcOutsideSubject => "kc-outside-subject",
            Self::ProjectionChanged => "projection-changed",
            Self::LockCorrupt => "lock-corrupt",
            Self::LockOutdated => "lock-outdated",
            Self::ReviewedWithoutReviewer => "reviewed-without-reviewer",
            Self::ReviewedTextChanged => "reviewed-text-changed",
            Self::OrphanReview => "orphan-review",
            Self::ImpliedPrerequisite => "implied-prerequisite",
            Self::RedundantSupports => "redundant-supports",
            Self::SecondaryDownstream => "secondary-downstream",
            Self::FamilyInstruction => "family-instruction",
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
    /// KCs that only receive observations: not live, and not blocked by
    /// anything an author could write.
    pub observed: Vec<String>,
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
        for kc in &self.observed {
            writeln!(f, "observed-only {kc}")?;
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
