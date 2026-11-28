// @vitest-environment jsdom

import { strToU8, zipSync } from "fflate";
import { describe, expect, it } from "vitest";

import { parseBookFile } from "./bookImport";
import { PrototypeContent } from "./content";

function content(): PrototypeContent {
  return new PrototypeContent(
    JSON.stringify({
      content: {
        kcs: [],
        items: [],
        explanations: [],
        instructions: [],
        misconceptions: [],
        lexemes: [],
        edges: [],
        item_lexemes: {},
      },
    }),
    "content-v1",
    "digest",
    "",
  );
}

function epubBytes(): Uint8Array {
  return zipSync({
    "META-INF/container.xml": strToU8(
      '<?xml version="1.0"?><container><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>',
    ),
    "OEBPS/content.opf": strToU8(
      '<?xml version="1.0"?><package><metadata><title>An EPUB Story</title><creator>A Writer</creator></metadata><manifest><item id="c1" href="chap.xhtml" media-type="application/xhtml+xml"/><item id="cover" href="cover.jpg" media-type="image/jpeg" properties="cover-image"/></manifest><spine><itemref idref="c1"/></spine></package>',
    ),
    "OEBPS/chap.xhtml": strToU8(
      '<?xml version="1.0"?><html><body><h1>Chapter One</h1><p>Hello, “world”! A cat sleeps.</p><p>It wakes up.</p></body></html>',
    ),
    "OEBPS/cover.jpg": new Uint8Array([0xff, 0xd8, 0xff, 0xd9]),
  });
}

describe("EPUB import", () => {
  it("keeps chapter punctuation, metadata, and the embedded cover", async () => {
    const file = new File([epubBytes() as BlobPart], "story.epub", {
      type: "application/epub+zip",
    });
    const imported = await parseBookFile(file, content(), { level: "A2" });
    expect(imported.book.title.en).toBe("An EPUB Story");
    expect(imported.book.source).toContain("A Writer");
    expect(imported.cover).not.toBeNull();
    expect(imported.book.chapters[0]?.title.en).toBe("Chapter One");
    const sentences =
      imported.book.chapters[0]?.paragraphs[0]?.sentences.map(
        (sentence) => sentence.text,
      ) ?? [];
    expect(sentences).toEqual(["Hello, “world”!", "A cat sleeps."]);
    expect(imported.book.chapters[0]?.paragraphs).toHaveLength(2);
  });
});
