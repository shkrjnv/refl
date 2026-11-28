import { describe, expect, it } from "vitest";

import { PrototypeContent } from "./content";
import {
  buildBookFromSource,
  chaptersFromMarkdown,
  parseBookFile,
  splitSentences,
  splitWords,
} from "./bookImport";

function content(): PrototypeContent {
  const lexeme = {
    id: "en.lex.cat",
    lemma: "cat",
    pos: "noun",
    frequency_rank: 100,
    level: "A1",
    forms: ["cats"],
    senses: [
      {
        id: "en.lex.cat.animal",
        definition_en: "a small animal with soft fur",
        gloss_uz: "mushuk",
        gloss_ru: "кошка",
        examples: [],
        collocations: [],
        ipa_uk: "",
        ipa_us: "",
      },
    ],
    kc_ids: ["en.gram.article.first-mention"],
    source: "test",
    license: "original",
  };
  return new PrototypeContent(
    JSON.stringify({
      content: {
        kcs: [],
        items: [],
        explanations: [],
        instructions: [],
        misconceptions: [],
        lexemes: [lexeme],
        edges: [],
        item_lexemes: {},
      },
    }),
    "content-v1",
    "digest",
    "cat\n",
  );
}

describe("uploaded book compiler", () => {
  it("keeps sentence punctuation, quotes, apostrophes, and paragraph breaks", async () => {
    const book = await buildBookFromSource(
      {
        title: "A Small Story",
        chapters: [
          {
            title: "One",
            paragraphs: [
              "Mira has a cat. \"Don't go!\" she said.",
              "The cat sleeps. It dreams, and the house is quiet.",
            ],
          },
        ],
      },
      { level: "A1", title: "A Small Story" },
      content(),
    );
    expect(book.id.startsWith("book.personal-")).toBe(true);
    expect(book.version).toMatch(/^[0-9a-f]{64}$/);
    const sentences = book.chapters[0]?.paragraphs[0]?.sentences ?? [];
    expect(sentences.map((sentence) => sentence.text)).toEqual([
      "Mira has a cat.",
      "\"Don't go!\"",
      "she said.",
    ]);
    const words = sentences.flatMap((sentence) =>
      sentence.tokens.map((token) => token.text),
    );
    expect(words).toContain("Don't");
    const cat = sentences[0]?.tokens.find((token) => token.text === "cat");
    expect(cat?.lexeme_id).toBe("en.lex.cat.animal");
    expect(cat?.kc_ids).toEqual(["en.gram.article.first-mention"]);
  });

  it("splits sentences and words without dropping typographic apostrophes", () => {
    expect(splitSentences("He said, \"Stop!\" Then he left. Ok?")).toEqual([
      "He said, \"Stop!\"",
      "Then he left.",
      "Ok?",
    ]);
    expect(splitWords("Don’t stop, well-known cat.")).toEqual([
      "Don’t",
      "stop",
      "well-known",
      "cat",
    ]);
  });

  it("turns Markdown headings into chapters and keeps prose punctuation", () => {
    const chapters = chaptersFromMarkdown(
      "# One\n\nFirst, a **cat** sleeps.\n\n## Two\n\nShe wakes up!\n",
      "Notes",
    );
    expect(chapters).toHaveLength(2);
    expect(chapters[0]?.title).toBe("One");
    expect(chapters[0]?.paragraphs).toEqual(["First, a cat sleeps."]);
    expect(chapters[1]?.paragraphs).toEqual(["She wakes up!"]);
  });

  it("imports a text file through the same compiler and rejects another type", async () => {
    const file = new File(
      ["A cat sits by the water.\n\nIt is a warm day."],
      "warm-day.txt",
      { type: "text/plain" },
    );
    const imported = await parseBookFile(file, content(), { level: "A1" });
    expect(imported.book.title.en).toBe("Warm Day");
    expect(imported.book.level).toBe("A1");
    expect(imported.book.chapters[0]?.paragraphs[0]?.sentences[0]?.text).toBe(
      "A cat sits by the water.",
    );

    const bad = new File(["nope"], "book.docx", { type: "application/msword" });
    await expect(parseBookFile(bad, content(), { level: "A1" })).rejects.toMatchObject({
      code: "unsupported_type",
    });
  });
});
