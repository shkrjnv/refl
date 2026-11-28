// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

import type { LearnerStorage, OutboxRecord } from "../db/storage";
import type { ClientClock } from "../sync/uuidv7";
import type { CoreWorkerClient } from "../worker/client";
import { RecordStore, parseRecords } from "./records";
import { testContent } from "../screens/testContent";

function clock(): ClientClock {
  let n = 0;
  return {
    nextEventId: () => {
      n += 1;
      return `01930000-0000-7000-8000-${n.toString().padStart(12, "0")}`;
    },
    stamp: () => {
      n += 1;
      return {
        event_id: `01930000-0000-7000-8000-${n.toString().padStart(12, "0")}`,
        client_ts: 1_700_000_000_000 + n,
      };
    },
    now: () => 1_700_000_000_000 + n,
    setOffset: () => undefined,
    update: () => ({ offsetMs: 0, suspicious: false }),
    offsetMs: 0,
  } as unknown as ClientClock;
}

async function store(): Promise<{
  writer: RecordStore;
  events: OutboxRecord[];
}> {
  const events: OutboxRecord[] = [];
  const storage = {
    queueOutbox: vi.fn(async (record: OutboxRecord) => {
      events.push(record);
    }),
    putSnapshot: vi.fn(async () => undefined),
  } as unknown as LearnerStorage;
  const core = {
    coreInfo: async () => ({
      core_version: "test",
      schema_version: 2,
      fold_version: "fold",
      max_bundle_format: 10,
    }),
    foldState: async (
      _bundle: string,
      _lexicon: string,
      _snapshot: string | null,
      incoming: { event_id: string }[],
    ) => ({
      snapshot: {},
      state_hash: "hash",
      incomplete: false,
      last_event_id: incoming[0]?.event_id ?? null,
    }),
  } as unknown as CoreWorkerClient;
  return {
    writer: new RecordStore({
      storage,
      core,
      content: testContent(),
      learnerId: "01930000-0000-7000-8000-0000000000aa",
      clock: clock(),
    }),
    events,
  };
}

describe("private record writer", () => {
  it("uses the creation event id as the note id", async () => {
    const { writer, events } = await store();
    await writer.saveNote("{}", {
      anchor: { kind: "standalone" },
      body: "hello",
    });
    const event = events[0]?.event as { event_id: string; payload: { note_id: string } };
    expect(event.payload.note_id).toBe(event.event_id);
  });

  it("uses the creation event id as the custom word id", async () => {
    const { writer, events } = await store();
    await writer.saveCustomWord("{}", {
      lemma: "mira",
      definitionEn: "a name",
      glossUz: "",
      glossRu: "",
      examples: [],
      collocations: [],
    });
    const event = events[0]?.event as { event_id: string; payload: { custom_id: string } };
    expect(event.payload.custom_id).toBe(event.event_id);
  });

  it("writes book marks with a quote hash and safe token range", async () => {
    const { writer, events } = await store();
    const result = await writer.createBookMark("{}", {
      bookId: "book.first-day",
      bookVersion: "a".repeat(64),
      chapterId: "chapter.first-day.1",
      paragraphId: "para.first-day.1.1",
      sentenceId: "sentence.first-day.1.1.1",
      sentenceText: "Mira has a book.",
      startToken: 0,
      endToken: 2,
      kind: "highlight",
    });
    const event = events[0]?.event as {
      payload: { mark_id: string; quote_hash: string; start_token: number };
    };
    expect(event.payload.mark_id).toBe(result.markId);
    expect(event.payload.quote_hash).toMatch(/^[0-9a-f]{64}$/);
    expect(event.payload.start_token).toBe(0);
  });

  it("logs lexical reviews and refuses an oversized note", async () => {
    const { writer, events } = await store();
    await writer.reviewLexeme("{}", "en.lex.cat.animal", true);
    expect((events[0]?.event as { payload: { type: string } }).payload.type).toBe(
      "lexical_review",
    );
    await expect(
      writer.saveNote("{}", {
        anchor: { kind: "standalone" },
        body: "a".repeat(4097),
      }),
    ).rejects.toThrow("4096");
  });

  it("parses anchors, cards and tombstones from a snapshot", () => {
    const records = parseRecords(
      JSON.stringify({
        lexical_cards: {
          "en.lex.cat.animal": {
            recognize: {
              stability: 2,
              difficulty: 3,
              last_review: 1,
              due: 2,
              direction: "recognize",
              source: "bundle",
              exposures: 0.45,
              review_count: 0,
              lapse_count: 0,
            },
          },
        },
        notes: {
          n: {
            note_id: "n",
            anchor: { kind: "kc", id: "en.gram.article.a-an-sound" },
            body: "note",
            deleted: false,
            event_id: "e",
            updated_at: 1,
          },
        },
        book_marks: {
          m: {
            mark_id: "m",
            book_id: "book.first-day",
            book_version: "a".repeat(64),
            kind: "bookmark",
            chapter_id: "c",
            paragraph_id: "p",
            sentence_id: "s",
            start_token: 0,
            end_token: 1,
            quote_hash: "b".repeat(64),
            deleted: false,
            event_id: "e",
            updated_at: 1,
          },
        },
      }),
    );
    expect(records.lexicalCards["en.lex.cat.animal"]?.recognize?.exposures).toBe(0.45);
    expect(records.notes["n"]?.anchor.kind).toBe("kc");
    expect(Object.keys(records.bookMarks)).toEqual(["m"]);
  });
});
