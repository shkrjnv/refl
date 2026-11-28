import type { LearnerStorage } from "../db/storage";
import { sha256Hex } from "../db/storage";
import type { NoteAnchorKind } from "../generated/core/NoteAnchorKind";
import type { ClientClock } from "../sync/uuidv7";
import { isJsonObject, type JsonObject, type JsonValue } from "../types/json";
import type { FoldResult } from "../wasm/types";
import type { CoreWorkerClient } from "../worker/client";
import { appendEvent, type SessionGate } from "./session";
import type { PrototypeContent } from "./content";

export type LexicalDirectionName = "recognize" | "produce";

export interface LexicalCardView {
  stability: number;
  difficulty: number;
  last_review: number;
  due: number;
  direction: LexicalDirectionName;
  source: "bundle" | "custom";
  exposures: number;
  review_count: number;
  lapse_count: number;
}

export interface CustomLexemeView {
  custom_id: string;
  source: "own" | "book" | "lesson" | "placement";
  bundle_lexeme_id?: string | null;
  lemma: string;
  pos?: string | null;
  definition_en: string;
  gloss_uz: string;
  gloss_ru: string;
  examples: string[];
  collocations: string[];
  ipa?: string | null;
  audio_key?: string | null;
  context?: {
    text: string;
    book_id?: string | null;
    sentence_id?: string | null;
  } | null;
  archived: boolean;
  event_id: string;
  updated_at: number;
}

export interface NoteAnchorView {
  kind: NoteAnchorKind;
  id?: string | null;
}

export interface NoteView {
  note_id: string;
  anchor: NoteAnchorView;
  body: string;
  deleted: boolean;
  event_id: string;
  updated_at: number;
}

export interface BookMarkView {
  mark_id: string;
  book_id: string;
  book_version: string;
  kind: "highlight" | "bookmark";
  chapter_id: string;
  paragraph_id: string;
  sentence_id: string;
  start_token: number;
  end_token: number;
  quote_hash: string;
  color?: string | null;
  deleted: boolean;
  event_id: string;
  updated_at: number;
}

export interface PersonalRecords {
  lexicalCards: Record<
    string,
    Partial<Record<LexicalDirectionName, LexicalCardView>>
  >;
  customLexemes: Record<string, CustomLexemeView>;
  notes: Record<string, NoteView>;
  bookMarks: Record<string, BookMarkView>;
}

const EMPTY: PersonalRecords = {
  lexicalCards: {},
  customLexemes: {},
  notes: {},
  bookMarks: {},
};

function number(value: JsonValue | undefined): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function string(value: JsonValue | undefined): string | null {
  return typeof value === "string" ? value : null;
}

function parseCard(value: JsonValue | undefined): LexicalCardView | null {
  if (!isJsonObject(value)) {
    return null;
  }
  const stability = number(value["stability"]);
  const difficulty = number(value["difficulty"]);
  const last = number(value["last_review"]);
  const due = number(value["due"]);
  if (stability === null || difficulty === null || last === null || due === null) {
    return null;
  }
  return {
    stability,
    difficulty,
    last_review: last,
    due,
    direction: value["direction"] === "produce" ? "produce" : "recognize",
    source: value["source"] === "custom" ? "custom" : "bundle",
    exposures: number(value["exposures"]) ?? 0,
    review_count: number(value["review_count"]) ?? 0,
    lapse_count: number(value["lapse_count"]) ?? 0,
  };
}

export function parseRecords(stateJson: string | null): PersonalRecords {
  if (!stateJson) {
    return EMPTY;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return EMPTY;
  }
  if (!isJsonObject(parsed)) {
    return EMPTY;
  }
  const out: PersonalRecords = {
    lexicalCards: {},
    customLexemes: {},
    notes: {},
    bookMarks: {},
  };
  if (isJsonObject(parsed["lexical_cards"])) {
    for (const [lexeme, directions] of Object.entries(parsed["lexical_cards"])) {
      if (!isJsonObject(directions)) {
        continue;
      }
      const card = parseCard(directions["recognize"]);
      const produce = parseCard(directions["produce"]);
      out.lexicalCards[lexeme] = {
        ...(card ? { recognize: card } : {}),
        ...(produce ? { produce } : {}),
      };
    }
  }
  if (isJsonObject(parsed["custom_lexemes"])) {
    for (const [id, value] of Object.entries(parsed["custom_lexemes"])) {
      if (!isJsonObject(value)) {
        continue;
      }
      const lemma = string(value["lemma"]);
      const definition = string(value["definition_en"]);
      const source = string(value["source"]);
      if (lemma === null || definition === null) {
        continue;
      }
      out.customLexemes[id] = {
        custom_id: id,
        source:
          source === "book" || source === "lesson" || source === "placement"
            ? source
            : "own",
        bundle_lexeme_id: string(value["bundle_lexeme_id"]),
        lemma,
        pos: string(value["pos"]),
        definition_en: definition,
        gloss_uz: string(value["gloss_uz"]) ?? "",
        gloss_ru: string(value["gloss_ru"]) ?? "",
        examples: Array.isArray(value["examples"])
          ? value["examples"].filter((item): item is string => typeof item === "string")
          : [],
        collocations: Array.isArray(value["collocations"])
          ? value["collocations"].filter(
              (item): item is string => typeof item === "string",
            )
          : [],
        ipa: string(value["ipa"]),
        audio_key: string(value["audio_key"]),
        context: isJsonObject(value["context"])
          ? {
              text: string(value["context"]["text"]) ?? "",
              book_id: string(value["context"]["book_id"]),
              sentence_id: string(value["context"]["sentence_id"]),
            }
          : null,
        archived: value["archived"] === true,
        event_id: string(value["event_id"]) ?? "",
        updated_at: number(value["updated_at"]) ?? 0,
      };
    }
  }
  if (isJsonObject(parsed["notes"])) {
    for (const [id, value] of Object.entries(parsed["notes"])) {
      if (!isJsonObject(value) || !isJsonObject(value["anchor"])) {
        continue;
      }
      const body = string(value["body"]);
      const kind = string(value["anchor"]["kind"]);
      if (body === null || kind === null) {
        continue;
      }
      out.notes[id] = {
        note_id: id,
        anchor: {
          kind: kind as NoteAnchorKind,
          id: string(value["anchor"]["id"]),
        },
        body,
        deleted: value["deleted"] === true,
        event_id: string(value["event_id"]) ?? "",
        updated_at: number(value["updated_at"]) ?? 0,
      };
    }
  }
  if (isJsonObject(parsed["book_marks"])) {
    for (const [id, value] of Object.entries(parsed["book_marks"])) {
      if (!isJsonObject(value)) {
        continue;
      }
      const bookId = string(value["book_id"]);
      const sentence = string(value["sentence_id"]);
      if (bookId === null || sentence === null) {
        continue;
      }
      out.bookMarks[id] = {
        mark_id: id,
        book_id: bookId,
        book_version: string(value["book_version"]) ?? "",
        kind: value["kind"] === "highlight" ? "highlight" : "bookmark",
        chapter_id: string(value["chapter_id"]) ?? "",
        paragraph_id: string(value["paragraph_id"]) ?? "",
        sentence_id: sentence,
        start_token: number(value["start_token"]) ?? 0,
        end_token: number(value["end_token"]) ?? 0,
        quote_hash: string(value["quote_hash"]) ?? "",
        color: string(value["color"]),
        deleted: value["deleted"] === true,
        event_id: string(value["event_id"]) ?? "",
        updated_at: number(value["updated_at"]) ?? 0,
      };
    }
  }
  return out;
}

export function activeNotes(
  records: PersonalRecords,
  anchor?: NoteAnchorView,
): NoteView[] {
  return Object.values(records.notes)
    .filter(
      (note) =>
        !note.deleted &&
        (anchor === undefined ||
          (note.anchor.kind === anchor.kind &&
            (note.anchor.id ?? null) === (anchor.id ?? null))),
    )
    .sort((left, right) => right.updated_at - left.updated_at);
}

export function activeMarks(
  records: PersonalRecords,
  bookId: string,
): BookMarkView[] {
  return Object.values(records.bookMarks).filter(
    (mark) => !mark.deleted && mark.book_id === bookId,
  );
}

export function customLexemeForSense(
  records: PersonalRecords,
  senseId: string,
): CustomLexemeView | null {
  return (
    Object.values(records.customLexemes).find(
      (record) => !record.archived && record.custom_id === senseId,
    ) ?? null
  );
}

/** The vocabulary card id of a personal word: its UUID string. */
export function cardIdOf(customId: string): string {
  return customId;
}

interface RecordOptions {
  storage: LearnerStorage;
  core: CoreWorkerClient;
  content: PrototypeContent;
  learnerId: string;
  clock: ClientClock;
  gate?: SessionGate;
}

function eventWith(
  options: RecordOptions,
  payload: JsonObject,
): JsonObject {
  const stamp = options.clock.stamp();
  return {
    schema_version: 2,
    event_id: stamp.event_id,
    learner_id: options.learnerId,
    session_id: null,
    client_ts: stamp.client_ts,
    server_ts: null,
    payload,
  };
}

function payloadBytes(payload: JsonObject): number {
  return new TextEncoder().encode(JSON.stringify(payload)).length;
}

export interface CustomWordInput {
  /** Existing custom id for an edit; omitted for a new word. */
  customId?: string;
  lemma: string;
  pos?: string;
  definitionEn: string;
  glossUz: string;
  glossRu: string;
  examples: string[];
  collocations: string[];
  ipa?: string;
  audioKey?: string;
  bundleLexemeId?: string;
  context?: {
    text: string;
    bookId?: string;
    sentenceId?: string;
  };
  source?: "own" | "book" | "lesson" | "placement";
}

export interface NoteInput {
  noteId?: string;
  anchor: NoteAnchorView;
  body: string;
}

export interface BookMarkInput {
  bookId: string;
  bookVersion: string;
  chapterId: string;
  paragraphId: string;
  sentenceId: string;
  sentenceText: string;
  startToken: number;
  endToken: number;
  kind: "highlight" | "bookmark";
  color?: string;
}

/**
 * Writes private records as ordinary schema-2 events: the same outbox,
 * snapshot fold and sync path as answers. The methods return the new snapshot
 * JSON so the caller can re-render without re-reading IndexedDB.
 */
export class RecordStore {
  readonly #options: RecordOptions;

  constructor(options: RecordOptions) {
    this.#options = options;
  }

  async saveNote(stateJson: string, input: NoteInput): Promise<string> {
    const body = input.body;
    if (new TextEncoder().encode(body).length > 4096) {
      throw new Error("note exceeds 4096 bytes");
    }
    if (body.trim().length === 0) {
      throw new Error("an empty note is a deletion");
    }
    if (input.anchor.kind === "standalone" && input.anchor.id) {
      throw new Error("a standalone note has no anchor id");
    }
    if (input.anchor.kind !== "standalone" && !input.anchor.id) {
      throw new Error("an anchored note needs an anchor id");
    }
    const event = eventWith(this.#options, {
      type: "note_saved",
      note_id: input.noteId ?? "",
      anchor: {
        kind: input.anchor.kind,
        ...(input.anchor.id ? { id: input.anchor.id } : {}),
      },
      body,
    });
    // A new note's id is its creation event; an edit keeps the same id.
    if (!input.noteId) {
      const payload = event["payload"] as JsonObject;
      payload["note_id"] = event["event_id"] as string;
    }
    return this.#append(stateJson, event);
  }

  async deleteNote(stateJson: string, noteId: string): Promise<string> {
    return this.#append(
      stateJson,
      eventWith(this.#options, { type: "note_deleted", note_id: noteId }),
    );
  }

  async saveCustomWord(
    stateJson: string,
    input: CustomWordInput,
  ): Promise<string> {
    if (!input.definitionEn.trim() || !input.lemma.trim()) {
      throw new Error("a personal word needs a lemma and a definition");
    }
    const event = eventWith(this.#options, {
      type: input.customId ? "custom_lexeme_updated" : "custom_lexeme_created",
      custom_id: input.customId ?? "",
      source: input.source ?? (input.context?.bookId ? "book" : "own"),
      ...(input.bundleLexemeId ? { bundle_lexeme_id: input.bundleLexemeId } : {}),
      lemma: input.lemma.trim(),
      ...(input.pos ? { pos: input.pos } : {}),
      definition_en: input.definitionEn.trim(),
      gloss_uz: input.glossUz.trim(),
      gloss_ru: input.glossRu.trim(),
      examples: input.examples.filter((entry) => entry.trim().length > 0),
      collocations: input.collocations.filter((entry) => entry.trim().length > 0),
      ...(input.ipa ? { ipa: input.ipa } : {}),
      ...(input.audioKey ? { audio_key: input.audioKey } : {}),
      ...(input.context
        ? {
            context: {
              text: input.context.text,
              ...(input.context.bookId ? { book_id: input.context.bookId } : {}),
              ...(input.context.sentenceId
                ? { sentence_id: input.context.sentenceId }
                : {}),
            },
          }
        : {}),
      archived: false,
    });
    const payload = event["payload"] as JsonObject;
    if (!input.customId) {
      payload["custom_id"] = event["event_id"] as string;
    }
    if (payloadBytes(payload) > 4096) {
      throw new Error("personal word exceeds 4096 JSON bytes");
    }
    return this.#append(stateJson, event);
  }

  async archiveCustomWord(stateJson: string, customId: string): Promise<string> {
    return this.#append(
      stateJson,
      eventWith(this.#options, {
        type: "custom_lexeme_archived",
        custom_id: customId,
        archived: true,
      }),
    );
  }

  async reviewLexeme(
    stateJson: string,
    lexeme: string,
    correct: boolean,
  ): Promise<string> {
    return this.#append(
      stateJson,
      eventWith(this.#options, {
        type: "lexical_review",
        lexeme,
        direction: "recognize",
        correct,
      }),
    );
  }

  async createBookMark(
    stateJson: string,
    input: BookMarkInput,
  ): Promise<{ stateJson: string; markId: string }> {
    if (
      input.startToken < 0 ||
      input.startToken > input.endToken ||
      input.endToken > Number.MAX_SAFE_INTEGER
    ) {
      throw new Error("invalid token range");
    }
    const quoteHash = await sha256Hex(input.sentenceText);
    const event = eventWith(this.#options, {
      type: "book_mark_created",
      mark_id: "",
      book_id: input.bookId,
      book_version: input.bookVersion,
      kind: input.kind,
      chapter_id: input.chapterId,
      paragraph_id: input.paragraphId,
      sentence_id: input.sentenceId,
      start_token: input.startToken,
      end_token: input.endToken,
      quote_hash: quoteHash,
      ...(input.color ? { color: input.color } : {}),
    });
    const payload = event["payload"] as JsonObject;
    payload["mark_id"] = event["event_id"] as string;
    const state = await this.#append(stateJson, event);
    const markId = event["event_id"] as string;
    return { stateJson: state, markId };
  }

  async deleteBookMark(stateJson: string, markId: string): Promise<string> {
    return this.#append(
      stateJson,
      eventWith(this.#options, { type: "book_mark_deleted", mark_id: markId }),
    );
  }

  async selectCompanion(stateJson: string, companion: string): Promise<string> {
    return this.#append(
      stateJson,
      eventWith(this.#options, { type: "companion_selected", companion }),
    );
  }

  async #append(stateJson: string, event: JsonObject): Promise<string> {
    return appendEvent(
      {
        storage: this.#options.storage,
        core: this.#options.core,
        content: this.#options.content,
        learnerId: this.#options.learnerId,
        clock: this.#options.clock,
        gate: this.#options.gate,
      },
      stateJson,
      event,
    );
  }
}

export type { FoldResult };
