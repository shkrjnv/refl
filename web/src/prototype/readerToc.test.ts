import { describe, expect, it } from "vitest";

import type { BookChapter } from "../generated/core/BookChapter";
import { chapterSections } from "./readerToc";

function chapter(paragraphs: string[]): BookChapter {
  return {
    id: "chapter.test.1",
    title: { en: "One", ru: "Один", uz: "Bir" },
    paragraphs: paragraphs.map((text, index) => ({
      id: `para.test.1.${index + 1}`,
      sentences: [{ id: `sentence.test.1.${index + 1}.1`, text, tokens: [] }],
    })),
  };
}

describe("reader table of contents", () => {
  it("has no sections for a short chapter", () => {
    expect(chapterSections(chapter(["A lion was asleep.", "A mouse ran over him."]))).toEqual([]);
  });

  it("starts sections at paragraph boundaries with their opening words", () => {
    const long = "Della counted the money three times and it was still the same. ".repeat(8);
    const sections = chapterSections(chapter([long, long, long, long, long, long]), 1000);
    expect(sections.length).toBeGreaterThan(1);
    expect(sections[0]).toMatchObject({
      id: "para.test.1.1",
      index: 0,
      firstSentenceId: "sentence.test.1.1.1",
      label: "Della counted the money three times and…",
    });
    for (const section of sections) {
      expect(section.id.startsWith("para.test.1.")).toBe(true);
    }
    expect(new Set(sections.map((section) => section.id)).size).toBe(sections.length);
  });
});
