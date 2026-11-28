// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import type { BookSentence } from "../generated/core/BookSentence";
import { selectionSpans, sentenceParts } from "./readerText";

const sentence: BookSentence = {
  id: "s1",
  text: "“Don't go!” she said, and the cat slept.",
  tokens: [
    { id: "t1", text: "Don't", lemma: "don't", sense_ids: [], kc_ids: [] },
    { id: "t2", text: "go", lemma: "go", sense_ids: [], kc_ids: [] },
    { id: "t3", text: "she", lemma: "she", sense_ids: [], kc_ids: [] },
    { id: "t4", text: "said", lemma: "said", sense_ids: [], kc_ids: [] },
    { id: "t5", text: "and", lemma: "and", sense_ids: [], kc_ids: [] },
    { id: "t6", text: "the", lemma: "the", sense_ids: [], kc_ids: [] },
    { id: "t7", text: "cat", lemma: "cat", sense_ids: [], kc_ids: [] },
    { id: "t8", text: "slept", lemma: "sleep", sense_ids: [], kc_ids: [] },
  ],
};

describe("reader sentence reconstruction", () => {
  it("renders every punctuation mark and the exact spacing", () => {
    const parts = sentenceParts(sentence);
    expect(parts.map((part) => part.text).join("")).toBe(sentence.text);
    expect(parts.some((part) => part.kind === "text" && part.text.includes("“"))).toBe(
      true,
    );
    expect(parts.some((part) => part.kind === "text" && part.text.includes(","))).toBe(
      true,
    );
    expect(parts.filter((part) => part.kind === "token")).toHaveLength(8);
  });

  it("maps a DOM selection to whole token ranges per sentence", () => {
    const root = document.createElement("article");
    const content = sentenceParts(sentence)
      .map((part) =>
        part.kind === "token"
          ? `<span data-token-index="${part.tokenIndex}" data-sentence-id="s1">${part.text}</span>`
          : `<span>${part.text}</span>`,
      )
      .join("");
    root.innerHTML = content;
    document.body.append(root);
    const tokens = root.querySelectorAll<HTMLElement>("[data-token-index]");
    const range = document.createRange();
    range.setStart(tokens[1]?.firstChild ?? root, 0);
    range.setEnd(tokens[3]?.firstChild ?? root, 4);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    expect(selectionSpans(selection as Selection, root)).toEqual([
      { sentenceId: "s1", start: 1, end: 3 },
    ]);
    root.remove();
  });
});
