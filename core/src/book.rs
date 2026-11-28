//! Offline graded-reader artifacts and personalized lexical coverage
//! (`docs/16-vocabulary-library-notes-graph.md`, §2).

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::ids::{BookId, KcId, LexemeId};
use crate::model::{KnowledgeModel, LexicalDirection, State};
use crate::subject::LevelKey;
use crate::value::{Localized, Timestamp};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Book {
    pub id: BookId,
    /// SHA-256 of the canonical book JSON, also stored in book-mark events.
    pub version: String,
    pub title: Localized,
    pub level: LevelKey,
    pub license: String,
    pub source: String,
    pub chapters: Vec<BookChapter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BookChapter {
    pub id: String,
    pub title: Localized,
    pub paragraphs: Vec<BookParagraph>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BookParagraph {
    pub id: String,
    pub sentences: Vec<BookSentence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BookSentence {
    pub id: String,
    pub text: String,
    pub tokens: Vec<BookToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BookToken {
    /// Deterministic `token.<book>.<chapter>.<para>.<sentence>.<n>`.
    pub id: String,
    pub text: String,
    pub lemma: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lexeme_id: Option<LexemeId>,
    #[serde(default)]
    pub sense_ids: Vec<LexemeId>,
    #[serde(default)]
    pub kc_ids: Vec<KcId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Coverage {
    pub known_tokens: u32,
    pub total_tokens: u32,
    /// `known_tokens / total_tokens`, 0 for a book with no lexical tokens.
    pub percent: f64,
}

/// Personalized lexical coverage: a token counts when its recognition card
/// has at least one direct review and `R >= 0.8`. Unknown tokens stay in the
/// denominator; tokens without a lexeme are ignored.
#[allow(clippy::cast_precision_loss)]
pub fn coverage(book: &Book, state: &State, model: &KnowledgeModel, now: Timestamp) -> Coverage {
    let mut known = 0u32;
    let mut total = 0u32;
    for chapter in &book.chapters {
        for paragraph in &chapter.paragraphs {
            for sentence in &paragraph.sentences {
                for token in &sentence.tokens {
                    let Some(lexeme) = &token.lexeme_id else {
                        continue;
                    };
                    total = total.saturating_add(1);
                    let card = state.lexical_card(lexeme, LexicalDirection::Recognize);
                    if card.is_some_and(|card| card.review_count > 0)
                        && model.lexical_recall(state, lexeme, LexicalDirection::Recognize, now)
                            >= 0.8
                    {
                        known = known.saturating_add(1);
                    }
                }
            }
        }
    }
    Coverage {
        known_tokens: known,
        total_tokens: total,
        percent: if total == 0 {
            0.0
        } else {
            f64::from(known) / f64::from(total)
        },
    }
}
