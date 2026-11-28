import { unzipSync } from "fflate";

import type { Book } from "../generated/core/Book";
import type { BookChapter } from "../generated/core/BookChapter";
import type { BookParagraph } from "../generated/core/BookParagraph";
import type { BookSentence } from "../generated/core/BookSentence";
import type { BookToken } from "../generated/core/BookToken";
import type { Lexeme } from "../generated/core/Lexeme";
import type { Localized } from "../generated/core/Localized";
import { sha256Hex } from "../db/storage";
import type { PrototypeContent } from "./content";

export const MAX_BOOK_FILE_BYTES = 15 * 1024 * 1024;
export const MAX_BOOK_TEXT_CHARS = 1_500_000;

export type ImportErrorCode =
  | "unsupported_type"
  | "file_too_large"
  | "empty_book"
  | "invalid_epub"
  | "no_text_layer";

export class BookImportError extends Error {
  readonly code: ImportErrorCode;

  constructor(code: ImportErrorCode, message?: string) {
    super(message ?? code);
    this.name = "BookImportError";
    this.code = code;
  }
}

export interface RawChapter {
  title?: string;
  paragraphs: string[];
}

export interface RawBook {
  title: string;
  author?: string;
  chapters: RawChapter[];
  cover?: Blob | null;
}

export interface ImportedBook {
  book: Book;
  cover: Blob | null;
  author: string;
}

export interface BookMetaInput {
  id?: string;
  title?: string;
  level: string;
  author?: string;
}

interface FormIndex {
  byForm: Map<string, Lexeme[]>;
}

function normalizedForm(form: string): string {
  return form.toLocaleLowerCase().replace(/[‘’]/g, "'");
}

function indexLexemes(content: PrototypeContent): FormIndex {
  const byForm = new Map<string, Lexeme[]>();
  for (const lexeme of content.lexemes) {
    const forms = new Set(
      [lexeme.lemma, ...lexeme.forms].map((form) => normalizedForm(form)),
    );
    for (const form of forms) {
      const list = byForm.get(form);
      if (list) {
        if (!list.some((entry) => entry.id === lexeme.id)) {
          list.push(lexeme);
        }
      } else {
        byForm.set(form, [lexeme]);
      }
    }
  }
  return { byForm };
}

/** Every word span the compiler can link, preserving apostrophes and hyphens. */
export function splitWords(text: string): string[] {
  return (
    text.match(/['‘’]?[A-Za-z]+(?:['‘’-][A-Za-z]+)*/g) ?? []
  );
}

/** Splits prose after `.`, `!` or `?`, keeping closing quotes and repeats. */
export function splitSentences(paragraph: string): string[] {
  const out: string[] = [];
  let start = 0;
  let index = 0;
  while (index < paragraph.length) {
    const char = paragraph[index];
    if (char !== "." && char !== "!" && char !== "?") {
      index += 1;
      continue;
    }
    let end = index + 1;
    while (
      end < paragraph.length &&
      ["\"", "'", "”", "’", "!", "?"].includes(paragraph[end] ?? "")
    ) {
      end += 1;
    }
    let after = end;
    while (after < paragraph.length && /\s/.test(paragraph[after] ?? "")) {
      after += 1;
    }
    if (after > end) {
      const sentence = paragraph.slice(start, end).trim();
      if (sentence) {
        out.push(sentence);
      }
      start = after;
      index = after;
    } else {
      index = end;
    }
  }
  const rest = paragraph.slice(start).trim();
  if (rest) {
    out.push(rest);
  }
  return out;
}

export function paragraphsFromText(text: string): string[] {
  return text
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .split(/\n{2,}/)
    .map((paragraph) =>
      paragraph
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
        .join(" ")
        .replace(/\s+/g, " ")
        .trim(),
    )
    .filter(Boolean);
}

function stripInlineMarkdown(line: string): string {
  return line
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[*_~`]/g, "")
    .trim();
}

/** Markdown headings become chapter boundaries; prose keeps its punctuation. */
export function chaptersFromMarkdown(
  text: string,
  fallbackTitle: string,
): RawChapter[] {
  const normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const chapters: RawChapter[] = [];
  let title = fallbackTitle;
  let buffer: string[] = [];
  const flush = () => {
    const paragraphs = paragraphsFromText(buffer.join("\n"))
      .map(stripInlineMarkdown)
      .filter(Boolean);
    if (paragraphs.length > 0) {
      chapters.push({ title, paragraphs });
    }
    buffer = [];
  };
  for (const line of normalized.split("\n")) {
    const heading = /^(#{1,3})\s+(.+)$/.exec(line.trim());
    if (heading) {
      flush();
      title = stripInlineMarkdown(heading[2] ?? "").slice(0, 120) || fallbackTitle;
      continue;
    }
    if (/^\s*([-*+]|\d+\.)\s+/.test(line)) {
      buffer.push(line.replace(/^\s*([-*+]|\d+\.)\s+/, ""));
      buffer.push("");
      continue;
    }
    buffer.push(line);
  }
  flush();
  return chapters.length > 0
    ? chapters
    : [{ title: fallbackTitle, paragraphs: [] }];
}

function titleFromFilename(name: string): string {
  const stem = name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ").trim();
  if (!stem) {
    return "My book";
  }
  return stem.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function slugOf(id: string): string {
  return id
    .replace(/^book\./, "")
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function linkWord(word: string, index: FormIndex): {
  lemma: string;
  lexemeId?: string;
  senseIds: string[];
  kcIds: string[];
} {
  const candidates = index.byForm.get(normalizedForm(word)) ?? [];
  const senses = new Set<string>();
  const kcs = new Set<string>();
  for (const lexeme of candidates) {
    for (const sense of lexeme.senses) {
      senses.add(sense.id);
    }
    for (const kc of lexeme.kc_ids) {
      kcs.add(kc);
    }
  }
  const senseIds = [...senses].sort();
  return {
    lemma: candidates.length === 1 ? (candidates[0]?.lemma ?? word) : word,
    ...(senseIds.length === 1 ? { lexemeId: senseIds[0] } : {}),
    senseIds: senseIds.length === 1 ? [] : senseIds,
    kcIds: [...kcs].sort(),
  };
}

function buildChapter(
  slug: string,
  number: number,
  title: string,
  paragraphs: string[],
  index: FormIndex,
): BookChapter {
  const built: BookParagraph[] = paragraphs.map((paragraph, p) => {
    const paragraphNumber = p + 1;
    const sentences: BookSentence[] = splitSentences(paragraph).map(
      (sentence, s) => {
        const sentenceNumber = s + 1;
        const tokens: BookToken[] = splitWords(sentence).map((word, t) => {
          const linked = linkWord(word, index);
          return {
            id: `token.${slug}.${number}.${paragraphNumber}.${sentenceNumber}.${t + 1}`,
            text: word,
            lemma: linked.lemma,
            ...(linked.lexemeId ? { lexeme_id: linked.lexemeId } : {}),
            sense_ids: linked.senseIds,
            kc_ids: linked.kcIds,
          };
        });
        return {
          id: `sentence.${slug}.${number}.${paragraphNumber}.${sentenceNumber}`,
          text: sentence,
          tokens,
        };
      },
    );
    return {
      id: `para.${slug}.${number}.${paragraphNumber}`,
      sentences,
    };
  });
  return {
    id: `chapter.${slug}.${number}`,
    title: { en: title } satisfies Localized,
    paragraphs: built,
  };
}

/** Compiles an imported source into the same JSON shape as book-build. */
export async function buildBookFromSource(
  source: RawBook,
  meta: BookMetaInput,
  content: PrototypeContent,
): Promise<Book> {
  const title = meta.title?.trim() || source.title.trim() || "My book";
  const id =
    meta.id ??
    `book.personal-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`}`;
  const slug = slugOf(id);
  const index = indexLexemes(content);
  const chapters: BookChapter[] = source.chapters.flatMap((chapter, number) => {
    const paragraphs = chapter.paragraphs
      .map((paragraph) => paragraph.replace(/\s+/g, " ").trim())
      .filter(Boolean);
    if (paragraphs.length === 0) {
      return [];
    }
    return [
      buildChapter(
        slug,
        number + 1,
        chapter.title?.trim() || title,
        paragraphs,
        index,
      ),
    ];
  });
  if (chapters.length === 0) {
    throw new BookImportError("empty_book");
  }
  const author = meta.author?.trim() || source.author?.trim() || "";
  const withoutVersion: Book = {
    id,
    version: "",
    title: { en: title },
    level: meta.level,
    license: "personal",
    source: author ? `Uploaded by you · ${author}` : "Uploaded by you",
    chapters,
  };
  const version = await sha256Hex(JSON.stringify(withoutVersion));
  return { ...withoutVersion, version };
}

function parseXml(text: string): Document {
  const doc = new DOMParser().parseFromString(text, "application/xml");
  if (doc.querySelector("parsererror")) {
    throw new BookImportError("invalid_epub", "EPUB XML is malformed");
  }
  return doc;
}

function decode(bytes: Uint8Array): string {
  return new TextDecoder("utf-8").decode(bytes);
}

function resolveZipPath(base: string, href: string): string {
  const clean = decodeURIComponent(href.split("#")[0] ?? "").replace(/^\/+/, "");
  const parts = base.split("/").slice(0, -1);
  for (const part of clean.split("/")) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  return parts.join("/");
}

function firstText(doc: Document | Element, name: string): string {
  const node = doc.getElementsByTagNameNS("*", name)[0];
  return node?.textContent?.trim() ?? "";
}

function htmlDocument(text: string): Document {
  const parser = new DOMParser();
  const xml = parser.parseFromString(text, "application/xhtml+xml");
  if (!xml.querySelector("parsererror")) {
    return xml;
  }
  return parser.parseFromString(text, "text/html");
}

function blocksFromHtml(text: string): RawChapter {
  const doc = htmlDocument(text);
  const body =
    doc.body ??
    doc.getElementsByTagNameNS("*", "body")[0] ??
    doc.documentElement;
  if (!body) {
    return { paragraphs: [] };
  }
  for (const node of body.querySelectorAll("script,style,svg,nav")) {
    node.remove();
  }
  const blocks = [...body.querySelectorAll("h1,h2,h3,h4,p,li,pre")];
  let title = "";
  const paragraphs: string[] = [];
  for (const block of blocks) {
    if (block.parentElement?.closest("p,li,pre")) {
      continue;
    }
    const value = (block.textContent ?? "").replace(/\s+/g, " ").trim();
    if (!value) {
      continue;
    }
    if (!title && /^H[1-4]$/.test(block.tagName.toUpperCase())) {
      title = value;
      continue;
    }
    paragraphs.push(value);
  }
  if (paragraphs.length === 0) {
    const fallback = (body.textContent ?? "").trim();
    if (fallback) {
      paragraphs.push(...paragraphsFromText(fallback));
    }
  }
  return { title: title || undefined, paragraphs };
}

interface EpubManifestItem {
  id: string;
  href: string;
  mediaType: string;
  properties: string;
}

async function parseEpub(file: File): Promise<RawBook> {
  const files = unzipSync(new Uint8Array(await file.arrayBuffer()));
  const containerBytes = files["META-INF/container.xml"];
  if (!containerBytes) {
    throw new BookImportError("invalid_epub", "EPUB has no container.xml");
  }
  const container = parseXml(decode(containerBytes));
  const opfPath = container
    .getElementsByTagNameNS("*", "rootfile")[0]
    ?.getAttribute("full-path");
  const opfBytes = opfPath ? files[opfPath] : undefined;
  if (!opfPath || !opfBytes) {
    throw new BookImportError("invalid_epub", "EPUB package is missing");
  }
  const opf = parseXml(decode(opfBytes));
  const title = firstText(opf, "title") || titleFromFilename(file.name);
  const author = firstText(opf, "creator") || undefined;
  const manifest = new Map<string, EpubManifestItem>();
  for (const item of opf.getElementsByTagNameNS("*", "item")) {
    const id = item.getAttribute("id");
    const href = item.getAttribute("href");
    if (!id || !href) {
      continue;
    }
    manifest.set(id, {
      id,
      href: resolveZipPath(opfPath, href),
      mediaType: item.getAttribute("media-type") ?? "",
      properties: item.getAttribute("properties") ?? "",
    });
  }
  const coverMeta = [...opf.getElementsByTagNameNS("*", "meta")].find(
    (meta) => meta.getAttribute("name") === "cover",
  );
  const coverId = coverMeta?.getAttribute("content") ?? "";
  const coverItem =
    [...manifest.values()].find((item) =>
      item.properties.split(/\s+/).includes("cover-image"),
    ) ??
    manifest.get(coverId) ??
    [...manifest.values()].find(
      (item) =>
        item.mediaType.startsWith("image/") && /cover/i.test(`${item.id} ${item.href}`),
    );
  let cover: Blob | null = null;
  if (coverItem) {
    const bytes = files[coverItem.href];
    if (bytes) {
      cover = new Blob([bytes as BlobPart], {
        type: coverItem.mediaType || "image/jpeg",
      });
    }
  }
  const chapters: RawChapter[] = [];
  const spine = [...opf.getElementsByTagNameNS("*", "itemref")];
  for (const itemref of spine) {
    const idref = itemref.getAttribute("idref");
    const item = idref ? manifest.get(idref) : undefined;
    if (!item || (!item.mediaType.includes("xhtml") && !item.mediaType.includes("html"))) {
      continue;
    }
    const bytes = files[item.href];
    if (!bytes) {
      continue;
    }
    const chapter = blocksFromHtml(decode(bytes));
    if (chapter.paragraphs.length > 0) {
      chapters.push({
        title: chapter.title ?? title,
        paragraphs: chapter.paragraphs,
      });
    }
  }
  if (chapters.length === 0) {
    throw new BookImportError("empty_book");
  }
  return { title, ...(author ? { author } : {}), chapters, cover };
}

function lineText(items: { str: string; x: number }[]): string {
  const sorted = [...items].sort((left, right) => left.x - right.x);
  let out = "";
  for (const item of sorted) {
    const text = item.str;
    if (!text) {
      continue;
    }
    if (
      out &&
      !/[\s([{"'‘“]$/.test(out) &&
      !/^[\s.,!?;:)\]}%'’”]/.test(text)
    ) {
      out += " ";
    }
    out += text;
  }
  return out.replace(/\s+/g, " ").trim();
}

interface PdfLine {
  text: string;
  y: number;
  height: number;
}

function paragraphsFromPdfLines(lines: PdfLine[]): string[] {
  const paragraphs: string[] = [];
  let buffer: string[] = [];
  let previous: PdfLine | null = null;
  for (const line of lines) {
    if (!line.text) {
      continue;
    }
    const height = line.height > 0 ? line.height : 12;
    if (
      previous &&
      buffer.length > 0 &&
      Math.abs(previous.y - line.y) > height * 1.7
    ) {
      paragraphs.push(buffer.join(" "));
      buffer = [];
    }
    buffer.push(line.text);
    previous = line;
  }
  if (buffer.length > 0) {
    paragraphs.push(buffer.join(" "));
  }
  return paragraphs.map((entry) => entry.replace(/\s+/g, " ").trim()).filter(Boolean);
}

async function parsePdf(file: File, title: string): Promise<RawBook> {
  const pdfjs = await import("pdfjs-dist");
  const worker = await import("pdfjs-dist/build/pdf.worker.min.mjs?url");
  pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
  const document = await pdfjs.getDocument({
    data: new Uint8Array(await file.arrayBuffer()),
    useWorkerFetch: false,
  }).promise;
  const paragraphs: string[] = [];
  for (let number = 1; number <= document.numPages; number += 1) {
    const page = await document.getPage(number);
    const content = await page.getTextContent();
    const lines: PdfLine[] = [];
    let current: { y: number; height: number; items: { str: string; x: number }[] } | null =
      null;
    for (const item of content.items) {
      if (!("str" in item)) {
        continue;
      }
      const transform = item.transform;
      const x = transform[4] ?? 0;
      const y = transform[5] ?? 0;
      const height = Math.abs(transform[3] ?? 12);
      if (!current || Math.abs(current.y - y) > Math.max(2, height * 0.35)) {
        if (current) {
          lines.push({
            y: current.y,
            height: current.height,
            text: lineText(current.items),
          });
        }
        current = { y, height, items: [] };
      }
      current.items.push({ str: item.str, x });
    }
    if (current) {
      lines.push({
        y: current.y,
        height: current.height,
        text: lineText(current.items),
      });
    }
    lines.sort((left, right) => right.y - left.y);
    paragraphs.push(...paragraphsFromPdfLines(lines));
  }
  if (paragraphs.length === 0) {
    throw new BookImportError("no_text_layer");
  }
  return { title, chapters: [{ title, paragraphs }] };
}

function extension(name: string): string {
  const match = /\.([^.]+)$/.exec(name.toLocaleLowerCase());
  return match?.[1] ?? "";
}

/** Reads an EPUB/PDF/TXT/Markdown upload without touching the network. */
export async function parseBookFile(
  file: File,
  content: PrototypeContent,
  meta: BookMetaInput,
): Promise<ImportedBook> {
  if (file.size > MAX_BOOK_FILE_BYTES) {
    throw new BookImportError("file_too_large");
  }
  if (file.size === 0) {
    throw new BookImportError("empty_book");
  }
  const ext = extension(file.name);
  if (!["epub", "pdf", "txt", "md", "markdown"].includes(ext)) {
    throw new BookImportError("unsupported_type");
  }
  let source: RawBook;
  if (ext === "epub") {
    source = await parseEpub(file);
  } else if (ext === "pdf") {
    source = await parsePdf(file, meta.title?.trim() || titleFromFilename(file.name));
  } else {
    const text = await file.text();
    if (text.length > MAX_BOOK_TEXT_CHARS) {
      throw new BookImportError("file_too_large");
    }
    const title = meta.title?.trim() || titleFromFilename(file.name);
    source =
      ext === "md" || ext === "markdown"
        ? { title, chapters: chaptersFromMarkdown(text, title) }
        : {
            title,
            chapters: [{ title, paragraphs: paragraphsFromText(text) }],
          };
  }
  if (meta.author?.trim()) {
    source = { ...source, author: meta.author.trim() };
  }
  if (meta.title?.trim()) {
    source = { ...source, title: meta.title.trim() };
  }
  if (source.title.length === 0) {
    source = { ...source, title: titleFromFilename(file.name) };
  }
  const estimated = source.chapters.reduce(
    (sum, chapter) =>
      sum + chapter.paragraphs.reduce((inner, paragraph) => inner + paragraph.length, 0),
    0,
  );
  if (estimated > MAX_BOOK_TEXT_CHARS) {
    throw new BookImportError("file_too_large");
  }
  const book = await buildBookFromSource(source, meta, content);
  return {
    book,
    cover: source.cover ?? null,
    author: source.author ?? "",
  };
}
