import type { BookChapter } from "../generated/core/BookChapter";

export interface ReaderSection {
  /** Paragraph id where the section starts; stable across sessions. */
  id: string;
  index: number;
  firstSentenceId: string;
  /** Opening words of the section, cut at a word boundary. */
  label: string;
}

/** Roughly two phone pages of prose per section. */
const SECTION_CHARS = 1600;
const LABEL_WORDS = 7;

function opening(text: string): string {
  const words = text.trim().split(/\s+/).filter(Boolean);
  const head = words.slice(0, LABEL_WORDS).join(" ");
  return words.length > LABEL_WORDS ? `${head.replace(/[,;:.!?—–-]+$/, "")}…` : head;
}

/**
 * Splits a chapter into navigable sections at paragraph boundaries. Books
 * carry chapters only, so sections are derived deterministically from the
 * text: a new section starts once the previous one holds about
 * `SECTION_CHARS` characters. A chapter shorter than that has no sections.
 */
export function chapterSections(
  chapter: BookChapter,
  targetChars = SECTION_CHARS,
): ReaderSection[] {
  const total = chapter.paragraphs.reduce(
    (sum, paragraph) =>
      sum + paragraph.sentences.reduce((count, sentence) => count + sentence.text.length, 0),
    0,
  );
  if (total <= targetChars * 1.5) {
    return [];
  }
  const sections: ReaderSection[] = [];
  let collected = Infinity;
  for (const paragraph of chapter.paragraphs) {
    const first = paragraph.sentences[0];
    if (!first) {
      continue;
    }
    if (collected >= targetChars) {
      sections.push({
        id: paragraph.id,
        index: sections.length,
        firstSentenceId: first.id,
        label: opening(first.text),
      });
      collected = 0;
    }
    collected += paragraph.sentences.reduce((count, sentence) => count + sentence.text.length, 0);
  }
  return sections;
}
