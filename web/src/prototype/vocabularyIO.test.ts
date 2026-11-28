import { describe, expect, it } from "vitest";

import {
  exportVocabulary,
  importFormat,
  previewVocabularyImport,
  type VocabularyWord,
} from "./vocabularyIO";

const words: VocabularyWord[] = [
  {
    lemma: "cat",
    pos: "noun",
    definition: "a small animal",
    glossUz: "mushuk",
    glossRu: "кошка",
    examples: ["I have a cat.", "The cat naps."],
    collocations: ["a black cat"],
    ipa: "kæt",
    bundleLexemeId: "en.lex.cat.animal",
    source: "course",
  },
  {
    lemma: "mira",
    pos: "noun",
    definition: "a name, with a comma",
    glossUz: "",
    glossRu: "имя",
    examples: ["Mira has a book."],
    collocations: [],
    ipa: "",
    source: "personal",
  },
];

describe("vocabulary export", () => {
  it("writes Anki/Quizlet-friendly TSV and CSV", () => {
    const tsv = exportVocabulary(words, {
      format: "tsv",
      course: true,
      personal: true,
    });
    expect(tsv.filename).toBe("refl-lexicon.tsv");
    expect(tsv.text.split("\n")[0]).toContain("Lemma\tDefinition");
    expect(tsv.text).toContain("cat\ta small animal\tmushuk\tкошка");
    expect(tsv.text).toContain("I have a cat. | The cat naps.");
    const csv = exportVocabulary(words, {
      format: "csv",
      course: false,
      personal: true,
    });
    expect(csv.text).not.toContain("cat,");
    expect(csv.text).toContain('"a name, with a comma"');
  });

  it("writes refl-lexicon-v1 JSON with senses and glosses", () => {
    const json = exportVocabulary(words, {
      format: "json",
      course: true,
      personal: false,
    });
    const parsed = JSON.parse(json.text) as {
      format: string;
      words: Record<string, unknown>[];
    };
    expect(parsed.format).toBe("refl-lexicon-v1");
    expect(parsed.words).toHaveLength(1);
    expect(parsed.words[0]?.["lemma"]).toBe("cat");
    expect(parsed.words[0]?.["gloss_uz"]).toBe("mushuk");
    expect(parsed.words[0]?.["bundle_lexeme_id"]).toBe("en.lex.cat.animal");
  });
});

describe("vocabulary import", () => {
  it("parses an Anki plain-text TSV with front/back headers", () => {
    const preview = previewVocabularyImport(
      "deck.txt",
      "Front\tBack\ncat\ta small animal\ndog\ta barking animal\n",
      [],
    );
    expect(preview.format).toBe("tsv");
    expect(preview.valid).toHaveLength(2);
    expect(preview.valid[0]?.candidate?.lemma).toBe("cat");
    expect(preview.valid[0]?.candidate?.definitionEn).toBe("a small animal");
  });

  it("parses quoted CSV fields and keeps the second definition column", () => {
    const preview = previewVocabularyImport(
      "deck.csv",
      'lemma,definition,gloss_uz\ncat,"a small, soft animal",mushuk\n',
      [],
    );
    expect(preview.format).toBe("csv");
    expect(preview.valid[0]?.candidate?.definitionEn).toBe("a small, soft animal");
    expect(preview.valid[0]?.candidate?.glossUz).toBe("mushuk");
  });

  it("deduplicates against course and personal lemmas and within the file", () => {
    const preview = previewVocabularyImport(
      "deck.tsv",
      "lemma\tdefinition\ncat\ta small animal\nCat\tanother\ndog\ta barking animal\ndog\ta second dog\n",
      ["cat"],
    );
    expect(preview.duplicates).toBe(3);
    expect(preview.valid).toHaveLength(1);
    expect(preview.valid[0]?.candidate?.lemma).toBe("dog");
  });

  it("reports invalid rows and records over the 4 KB cap", () => {
    const long = "a".repeat(5000);
    const preview = previewVocabularyImport(
      "deck.json",
      JSON.stringify({
        format: "refl-lexicon-v1",
        words: [
          { lemma: "ok", definition_en: "fine" },
          { lemma: "", definition_en: "missing lemma" },
          { lemma: "huge", definition_en: long },
        ],
      }),
      [],
    );
    expect(preview.invalid).toBe(1);
    expect(preview.tooLarge).toBe(1);
    expect(preview.valid).toHaveLength(1);
    expect(preview.warnings.some((warning) => warning.includes("4096") || warning.includes("limit"))).toBe(true);
  });

  it("detects the format from the extension or the header", () => {
    expect(importFormat("deck.json", "")).toBe("json");
    expect(importFormat("deck.csv", "a,b")).toBe("csv");
    expect(importFormat("deck.txt", "a\tb")).toBe("tsv");
    expect(importFormat("unknown", "a\tb")).toBe("tsv");
  });
});
