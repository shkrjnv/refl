/** Standard vocabulary interchange for the personal dictionary. */

export type VocabularyFormat = "tsv" | "csv" | "json";
export type VocabularySource = "course" | "personal";

export interface VocabularyWord {
  lemma: string;
  pos: string;
  definition: string;
  glossUz: string;
  glossRu: string;
  examples: string[];
  collocations: string[];
  ipa: string;
  bundleLexemeId?: string | null;
  source: VocabularySource;
}

export interface ExportOptions {
  format: VocabularyFormat;
  course: boolean;
  personal: boolean;
}

export interface ImportCandidate {
  lemma: string;
  pos: string;
  definitionEn: string;
  glossUz: string;
  glossRu: string;
  examples: string[];
  collocations: string[];
  ipa: string;
  bundleLexemeId?: string;
}

export type ImportStatus = "valid" | "duplicate" | "invalid" | "too_large";

export interface ImportRow {
  line: number;
  candidate: ImportCandidate | null;
  status: ImportStatus;
  message: string | null;
  bytes: number;
}

export interface ImportPreview {
  format: VocabularyFormat;
  rows: ImportRow[];
  valid: ImportRow[];
  duplicates: number;
  invalid: number;
  tooLarge: number;
  warnings: string[];
}

const HEADER_LABELS = [
  "Lemma",
  "Definition",
  "Gloss (uz)",
  "Gloss (ru)",
  "Examples",
  "Collocations",
  "IPA",
];

function quote(value: string, delimiter: string): string {
  if (delimiter === "\t") {
    return value.replace(/[\t\r\n]+/g, " ");
  }
  if (/[",\r\n]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

function row(word: VocabularyWord): string[] {
  return [
    word.lemma,
    word.definition,
    word.glossUz,
    word.glossRu,
    word.examples.join(" | "),
    word.collocations.join(" | "),
    word.ipa,
  ];
}

export function exportVocabulary(
  words: readonly VocabularyWord[],
  options: ExportOptions,
): { filename: string; mime: string; text: string } {
  const selected = words.filter(
    (word) =>
      (word.source === "course" && options.course) ||
      (word.source === "personal" && options.personal),
  );
  if (options.format === "json") {
    return {
      filename: "refl-lexicon.json",
      mime: "application/json",
      text: `${JSON.stringify(
        {
          format: "refl-lexicon-v1",
          words: selected.map((word) => ({
            lemma: word.lemma,
            pos: word.pos,
            definition_en: word.definition,
            gloss_uz: word.glossUz,
            gloss_ru: word.glossRu,
            examples: word.examples,
            collocations: word.collocations,
            ipa: word.ipa,
            ...(word.bundleLexemeId
              ? { bundle_lexeme_id: word.bundleLexemeId }
              : {}),
            source: word.source,
          })),
        },
        null,
        2,
      )}\n`,
    };
  }
  const delimiter = options.format === "tsv" ? "\t" : ",";
  const lines = [
    HEADER_LABELS.map((label) => quote(label, delimiter)).join(delimiter),
    ...selected.map((word) =>
      row(word)
        .map((field) => quote(field, delimiter))
        .join(delimiter),
    ),
  ];
  return {
    filename: `refl-lexicon.${options.format}`,
    mime:
      options.format === "tsv"
        ? "text/tab-separated-values;charset=utf-8"
        : "text/csv;charset=utf-8",
    text: `${lines.join("\n")}\n`,
  };
}

function normalizeHeader(value: string): string {
  return value
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, "");
}

const FIELD_ALIASES: Record<string, string[]> = {
  lemma: ["lemma", "word", "front", "term", "english"],
  definition: ["definition", "definitionen", "back", "meaning", "translation"],
  glossUz: ["glossuz", "uzbek", "uz", "glossuzbek"],
  glossRu: ["glossru", "russian", "ru", "glossrussian"],
  examples: ["examples", "example", "sentences", "sentence"],
  collocations: ["collocations", "collocation", "phrases"],
  ipa: ["ipa", "pronunciation", "transcription"],
  pos: ["pos", "partofspeech"],
};

function fieldFor(header: string): string | null {
  const normalized = normalizeHeader(header);
  for (const [field, aliases] of Object.entries(FIELD_ALIASES)) {
    if (aliases.includes(normalized)) {
      return field;
    }
  }
  return null;
}

function parseDelimited(text: string, delimiter: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  const source = text.replace(/^﻿/, "").replace(/\r\n/g, "\n");
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quoted) {
      if (character === '"' && source[index + 1] === '"') {
        field += '"';
        index += 1;
      } else if (character === '"') {
        quoted = false;
      } else {
        field += character;
      }
      continue;
    }
    if (character === '"' && field.length === 0) {
      quoted = true;
    } else if (character === delimiter) {
      row.push(field.trim());
      field = "";
    } else if (character === "\n") {
      row.push(field.trim());
      if (row.some((cell) => cell.length > 0)) {
        rows.push(row);
      }
      row = [];
      field = "";
    } else {
      field += character;
    }
  }
  row.push(field.trim());
  if (row.some((cell) => cell.length > 0)) {
    rows.push(row);
  }
  return rows;
}

function list(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value
      .filter((item): item is string => typeof item === "string")
      .map((item) => item.trim())
      .filter(Boolean);
  }
  if (typeof value === "string") {
    return value
      .split(/\s*\|\s*|\s*;\s*|\n/)
      .map((item) => item.trim())
      .filter(Boolean);
  }
  return [];
}

function string(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function candidateFrom(fields: Record<string, unknown>): ImportCandidate | null {
  const lemma =
    string(fields["lemma"]) || string(fields["word"]) || string(fields["front"]);
  const lemmaTrimmed = lemma.trim();
  const definition =
    string(fields["definition"]) ||
    string(fields["definition_en"]) ||
    string(fields["back"]);
  if (!lemmaTrimmed || !definition) {
    return null;
  }
  return {
    lemma: lemmaTrimmed,
    pos: string(fields["pos"]),
    definitionEn: definition,
    glossUz: string(fields["gloss_uz"]) || string(fields["glossUz"]),
    glossRu: string(fields["gloss_ru"]) || string(fields["glossRu"]),
    examples: list(fields["examples"]),
    collocations: list(fields["collocations"]),
    ipa: string(fields["ipa"]),
    ...(string(fields["bundle_lexeme_id"])
      ? { bundleLexemeId: string(fields["bundle_lexeme_id"]) }
      : {}),
  };
}

export function importFormat(fileName: string, text: string): VocabularyFormat {
  const name = fileName.toLocaleLowerCase();
  if (name.endsWith(".json") || text.trimStart().startsWith("{")) {
    return "json";
  }
  if (name.endsWith(".csv")) {
    return "csv";
  }
  if (name.endsWith(".tsv") || name.endsWith(".txt")) {
    return "tsv";
  }
  const firstLine = text.split(/\r?\n/, 1)[0] ?? "";
  return firstLine.includes("\t") ? "tsv" : "csv";
}

function parseJson(text: string): Record<string, unknown>[] {
  const parsed: unknown = JSON.parse(text);
  if (Array.isArray(parsed)) {
    return parsed.filter(
      (entry): entry is Record<string, unknown> =>
        typeof entry === "object" && entry !== null,
    );
  }
  if (typeof parsed === "object" && parsed !== null) {
    const object = parsed as Record<string, unknown>;
    if (object["format"] !== undefined && object["format"] !== "refl-lexicon-v1") {
      throw new Error("unknown vocabulary JSON format");
    }
    const words = object["words"];
    if (Array.isArray(words)) {
      return words.filter(
        (entry): entry is Record<string, unknown> =>
          typeof entry === "object" && entry !== null,
      );
    }
  }
  throw new Error("JSON must contain a refl-lexicon-v1 words array");
}

function payloadBytes(candidate: ImportCandidate): number {
  const payload = {
    type: "custom_lexeme_created",
    custom_id: "01930000-0000-7000-8000-000000000000",
    source: "own",
    lemma: candidate.lemma,
    ...(candidate.pos ? { pos: candidate.pos } : {}),
    definition_en: candidate.definitionEn,
    gloss_uz: candidate.glossUz,
    gloss_ru: candidate.glossRu,
    examples: candidate.examples,
    collocations: candidate.collocations,
    ...(candidate.ipa ? { ipa: candidate.ipa } : {}),
    ...(candidate.bundleLexemeId
      ? { bundle_lexeme_id: candidate.bundleLexemeId }
      : {}),
    archived: false,
  };
  return new TextEncoder().encode(JSON.stringify(payload)).length;
}

export function previewVocabularyImport(
  fileName: string,
  text: string,
  existingLemmas: readonly string[],
  limit = 1000,
): ImportPreview {
  const format = importFormat(fileName, text);
  const raw: Record<string, unknown>[] = [];
  if (format === "json") {
    raw.push(...parseJson(text));
  } else {
    const delimiter = format === "tsv" ? "\t" : ",";
    const table = parseDelimited(text, delimiter);
    if (table.length === 0) {
      throw new Error("the file has no rows");
    }
    const first = table[0] ?? [];
    const header = first.map(fieldFor);
    const hasHeader = header.some((field) => field === "lemma");
    const mapping = hasHeader
      ? header
      : (["lemma", "definition", "glossUz", "glossRu", "examples", "collocations", "ipa"] as const);
    for (const cells of table.slice(hasHeader ? 1 : 0)) {
      const fields: Record<string, unknown> = {};
      mapping.forEach((field, index) => {
        if (field) {
          fields[field] = cells[index] ?? "";
        }
      });
      raw.push(fields);
    }
  }
  const existing = new Set(existingLemmas.map((lemma) => lemma.toLocaleLowerCase()));
  const seen = new Set<string>();
  const rows: ImportRow[] = [];
  const warnings: string[] = [];
  for (let index = 0; index < raw.length; index += 1) {
    if (rows.length >= limit) {
      warnings.push(`Only the first ${limit} rows were read.`);
      break;
    }
    const line = index + 1;
    const candidate = candidateFrom(raw[index] ?? {});
    if (!candidate) {
      rows.push({
        line,
        candidate: null,
        status: "invalid",
        message: "Lemma and definition are required.",
        bytes: 0,
      });
      continue;
    }
    const key = candidate.lemma.toLocaleLowerCase();
    if (existing.has(key)) {
      rows.push({
        line,
        candidate,
        status: "duplicate",
        message: "Already in the course or your personal list.",
        bytes: 0,
      });
      continue;
    }
    if (seen.has(key)) {
      rows.push({
        line,
        candidate,
        status: "duplicate",
        message: "Appears earlier in this file.",
        bytes: 0,
      });
      continue;
    }
    seen.add(key);
    const bytes = payloadBytes(candidate);
    if (bytes > 4096) {
      warnings.push(`Row  is  bytes and exceeds the 4096-byte limit.`);
      rows.push({
        line,
        candidate,
        status: "too_large",
        message: `Record is  bytes; the limit is 4096.`,
        bytes,
      });
      continue;
    }
    if (bytes > 3000) {
      warnings.push(`Row ${line} is large (${bytes} bytes) and will be close to the limit.`);
    }
    rows.push({
      line,
      candidate,
      status: "valid",
      message: null,
      bytes,
    });
  }
  return {
    format,
    rows,
    valid: rows.filter((entry) => entry.status === "valid"),
    duplicates: rows.filter((entry) => entry.status === "duplicate").length,
    invalid: rows.filter((entry) => entry.status === "invalid").length,
    tooLarge: rows.filter((entry) => entry.status === "too_large").length,
    warnings,
  };
}

/** Converts a valid import row into the RecordStore input shape. */
export function candidateToInput(candidate: ImportCandidate) {
  return {
    lemma: candidate.lemma,
    ...(candidate.pos ? { pos: candidate.pos } : {}),
    definitionEn: candidate.definitionEn,
    glossUz: candidate.glossUz,
    glossRu: candidate.glossRu,
    examples: candidate.examples,
    collocations: candidate.collocations,
    ...(candidate.ipa ? { ipa: candidate.ipa } : {}),
    ...(candidate.bundleLexemeId
      ? { bundleLexemeId: candidate.bundleLexemeId }
      : {}),
    source: "own" as const,
  };
}
