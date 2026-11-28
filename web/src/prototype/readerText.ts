import type { BookSentence } from "../generated/core/BookSentence";

export type ReaderPart =
  | { kind: "text"; text: string }
  | { kind: "token"; text: string; tokenIndex: number };

/**
 * Rebuilds the sentence exactly as printed: tokens remain interactive, while
 * all the punctuation and whitespace between them stay plain text.
 */
export function sentenceParts(sentence: BookSentence): ReaderPart[] {
  const parts: ReaderPart[] = [];
  let cursor = 0;
  sentence.tokens.forEach((token, tokenIndex) => {
    const found = sentence.text.indexOf(token.text, cursor);
    if (found < 0) {
      return;
    }
    if (found > cursor) {
      parts.push({ kind: "text", text: sentence.text.slice(cursor, found) });
    }
    parts.push({ kind: "token", text: token.text, tokenIndex });
    cursor = found + token.text.length;
  });
  if (cursor < sentence.text.length) {
    parts.push({ kind: "text", text: sentence.text.slice(cursor) });
  }
  if (parts.length === 0) {
    parts.push({ kind: "text", text: sentence.text });
  }
  return parts;
}

export interface SelectionSpan {
  sentenceId: string;
  start: number;
  end: number;
}

/** Maps a browser selection to whole-token ranges, grouped by sentence. */
export function selectionSpans(
  selection: Selection,
  root: HTMLElement,
): SelectionSpan[] {
  if (selection.isCollapsed || selection.rangeCount === 0) {
    return [];
  }
  const range = selection.getRangeAt(0);
  if (!root.contains(range.commonAncestorContainer)) {
    return [];
  }
  const ranges = new Map<string, { start: number; end: number }>();
  for (const element of root.querySelectorAll<HTMLElement>("[data-token-index]")) {
    if (!range.intersectsNode(element)) {
      continue;
    }
    const sentenceId = element.dataset["sentenceId"];
    const rawIndex = element.dataset["tokenIndex"];
    if (!sentenceId || rawIndex === undefined) {
      continue;
    }
    const index = Number.parseInt(rawIndex, 10);
    if (!Number.isFinite(index)) {
      continue;
    }
    const current = ranges.get(sentenceId);
    if (current) {
      current.start = Math.min(current.start, index);
      current.end = Math.max(current.end, index);
    } else {
      ranges.set(sentenceId, { start: index, end: index });
    }
  }
  return [...ranges.entries()]
    .map(([sentenceId, value]) => ({ sentenceId, ...value }))
    .sort((left, right) => left.start - right.start);
}
