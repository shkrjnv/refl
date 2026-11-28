import { describe, expect, it } from "vitest";

import { boxesOverlap, type Box } from "./graphLayout";
import type { BookMarkView, NoteView, PersonalRecords } from "./records";
import {
  activeWikiQuery,
  buildBacklinks,
  headingId,
  localGraphLayout,
  localNoteGraph,
  noteFilterOf,
  noteSnippet,
  noteOutline,
  noteTitle,
  noteWordCount,
  resolveWikiLinks,
  wikiSuggestions,
  type WikiContext,
} from "./notes";

function note(
  id: string,
  body: string,
  anchor: NoteView["anchor"] = { kind: "standalone", id: null },
  updated = 1,
): NoteView {
  return {
    note_id: id,
    anchor,
    body,
    deleted: false,
    event_id: id,
    updated_at: updated,
  };
}

const n1 = note(
  "n1",
  "# Articles\n\nA rule with **markdown**.\n\n## Examples\n\n- a cat\n- a dog",
  { kind: "kc", id: "en.gram.article.first-mention" },
  2,
);
const n2 = note("n2", "# Reading\n\nSee [[note:n1]] and [[cat]] and [[book:book.aesop-lion-mouse]].", {
  kind: "kc",
  id: "en.gram.article.first-mention",
}, 3);
const n3 = note("n3", "# Backlink\n\nSee [[Reading]] too.", { kind: "standalone", id: null }, 4);
const n4 = note("n4", "# Independent\n\nNo links.", { kind: "lexeme", id: "en.lex.cat.animal" }, 5);

const records: PersonalRecords = {
  lexicalCards: {},
  customLexemes: {},
  notes: { n1, n2, n3, n4 },
  bookMarks: {},
};

const context: WikiContext = {
  notes: [n1, n2, n3, n4],
  kcs: [{ id: "en.gram.article.first-mention", title: "first mention" }],
  lexemes: [{ id: "en.lex.cat.animal", lemma: "cat" }],
  books: [{ id: "book.aesop-lion-mouse", title: "The Lion and the Mouse" }],
};

describe("notes workspace model", () => {
  it("derives titles, counts, outlines, and stable heading ids", () => {
    expect(noteTitle(n1)).toBe("Articles");
    expect(noteWordCount(n1.body)).toBeGreaterThan(3);
    const outline = noteOutline(n1.body);
    expect(outline.map((item) => `${item.level}:${item.text}`)).toEqual([
      "1:Articles",
      "2:Examples",
    ]);
    expect(outline[0]?.id).toBe(headingId(0, "Articles"));
  });

  it("resolves notes, topics, lexemes, and books inside wiki links", () => {
    const links = resolveWikiLinks(n2.body, context);
    expect(links.map((link) => [link.kind, link.id])).toEqual([
      ["note", "n1"],
      ["lexeme", "en.lex.cat.animal"],
      ["book", "book.aesop-lion-mouse"],
    ]);
    expect(links[1]?.label).toBe("cat");
  });

  it("finds incoming, shared, and outgoing connections", () => {
    const backlinks = buildBacklinks(n1, records, context);
    expect(backlinks.incoming.map((entry) => entry.note_id)).toEqual(["n2"]);
    expect(backlinks.shared.map((entry) => entry.note_id)).toEqual(["n2"]);
    expect(backlinks.outgoing).toEqual([]);
    expect(backlinks.highlights).toEqual([]);
  });

  it("builds a small local graph centered on the active note", () => {
    const graph = localNoteGraph(n2, records, context);
    expect(graph.nodes[0]).toMatchObject({ id: "note:n2", kind: "note" });
    expect(graph.nodes.some((node) => node.id === "note:n1")).toBe(true);
    expect(graph.nodes.some((node) => node.id === "kc:en.gram.article.first-mention")).toBe(
      true,
    );
    expect(graph.edges.every((edge) => edge.from === "note:n2")).toBe(true);
  });

  it("suggests links and detects the active [[ query", () => {
    expect(wikiSuggestions("read", context).map((entry) => entry.target)).toContain(
      "note:n2",
    );
    expect(activeWikiQuery("Text [[rea", 10)).toEqual({
      start: 5,
      end: 10,
      query: "rea",
    });
    expect(activeWikiQuery("Text [[done]]", 14)).toBeNull();
  });

  it("keeps highlights that are anchored to the note", () => {
    const mark: BookMarkView = {
      mark_id: "m1",
      book_id: "book.aesop-lion-mouse",
      book_version: "v",
      kind: "highlight",
      chapter_id: "c",
      paragraph_id: "p",
      sentence_id: "s",
      start_token: 0,
      end_token: 1,
      quote_hash: "",
      deleted: false,
      event_id: "m1",
      updated_at: 1,
    };
    const anchored = note("n5", "> quote", { kind: "book_mark", id: "m1" }, 6);
    const withMark: PersonalRecords = {
      ...records,
      notes: { ...records.notes, n5: anchored },
      bookMarks: { m1: mark },
    };
    const backlinks = buildBacklinks(anchored, withMark, context);
    expect(backlinks.highlights.map((entry) => entry.mark_id)).toEqual(["m1"]);
  });
});

describe("notes explorer helpers", () => {
  it("groups anchors into the explorer filters", () => {
    expect(noteFilterOf(note("a", "x", { kind: "kc", id: "en.gram.a" }))).toBe("grammar");
    expect(noteFilterOf(note("b", "x", { kind: "misconception", id: "m" }))).toBe("grammar");
    expect(noteFilterOf(note("c", "x", { kind: "book_mark", id: "m" }))).toBe("books");
    expect(noteFilterOf(note("d", "x", { kind: "lexeme", id: "en.lex.cat" }))).toBe("vocabulary");
    expect(noteFilterOf(note("e", "x"))).toBe("standalone");
  });

  it("shows the first prose line after the title as a snippet", () => {
    expect(noteSnippet("# Title\n\n> [!note] Remember\n> the rule")).toBe("Remember");
    expect(noteSnippet("# Title\n\nUse **a** before [[kc:x|a sound]].")).toBe(
      "Use a before a sound.",
    );
    expect(noteSnippet("Only a title")).toBe("");
    expect(noteSnippet(`# T\n\n${"word ".repeat(40)}`, 20).endsWith("…")).toBe(true);
  });
});

describe("local graph layout", () => {
  it("keeps every pill inside the panel and apart from the others", () => {
    const nodes = [
      { id: "note:center", kind: "note" as const, label: "Articles in my own words" },
      ...Array.from({ length: 9 }, (_, index) => ({
        id: `note:n${index}`,
        kind: "note" as const,
        label: index % 2 ? "Second mention and the listener" : "a/an",
      })),
    ];
    const labels = new Map(nodes.map((node) => [node.id, node.label]));
    for (const width of [264, 320]) {
      const placed = localGraphLayout(
        { nodes, edges: nodes.slice(1).map((node) => ({ from: "note:center", to: node.id })) },
        labels,
        width,
      );
      const boxes = [...placed.boxes.values()];
      expect(boxes).toHaveLength(nodes.length);
      for (const box of boxes) {
        expect(box.x - box.width / 2).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width / 2).toBeLessThanOrEqual(width);
        expect(box.y - box.height / 2).toBeGreaterThanOrEqual(0);
        expect(box.y + box.height / 2).toBeLessThanOrEqual(placed.height);
      }
      for (let i = 0; i < boxes.length; i += 1) {
        for (let j = i + 1; j < boxes.length; j += 1) {
          expect(boxesOverlap(boxes[i] as Box, boxes[j] as Box, 4)).toBe(false);
        }
      }
      expect(placed.edges).toHaveLength(9);
      for (const lines of placed.lines.values()) {
        expect(lines.join(" ")).not.toMatch(/\S-\S$/);
      }
    }
  });
});
