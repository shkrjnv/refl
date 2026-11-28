import type { BookMarkView, NoteView, PersonalRecords } from "./records";

export type WikiKind = "note" | "kc" | "lexeme" | "book" | "highlight";

export interface WikiTarget {
  kind: WikiKind;
  id: string;
  label: string;
  target: string;
}

export interface WikiContext {
  notes: readonly NoteView[];
  kcs: readonly { id: string; title: string }[];
  lexemes: readonly { id: string; lemma: string }[];
  books: readonly { id: string; title: string }[];
}

export interface OutlineItem {
  id: string;
  level: 1 | 2 | 3;
  text: string;
  line: number;
}

export interface Backlinks {
  outgoing: WikiTarget[];
  incoming: NoteView[];
  shared: NoteView[];
  highlights: BookMarkView[];
}

export interface LocalGraphNode {
  id: string;
  kind: WikiKind;
  label: string;
}

export interface LocalGraphEdge {
  from: string;
  to: string;
}

export interface WikiSuggestion {
  target: string;
  label: string;
  kind: WikiKind;
}

function stripMarkdown(text: string): string {
  return text
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g, (_match, target, label) => label || target)
    .replace(/[*_~`>#-]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/** The first heading, or the first prose line, names a note. */
export function noteTitle(note: Pick<NoteView, "body">): string {
  for (const line of note.body.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const title = stripMarkdown(trimmed.replace(/^#{1,6}\s+/, ""));
    if (title) {
      return title.length > 90 ? `${title.slice(0, 87)}…` : title;
    }
  }
  return "Untitled note";
}

export function noteWordCount(body: string): number {
  const text = stripMarkdown(body);
  return text ? text.split(/\s+/).length : 0;
}

export function headingId(line: number, text: string): string {
  const slug = stripMarkdown(text)
    .toLocaleLowerCase()
    .replace(/[^a-z0-9Ѐ-ӿ]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  return `note-h-${line}${slug ? `-${slug}` : ""}`;
}

/** H1–H3 headings outside fenced code form the outline. */
export function noteOutline(body: string): OutlineItem[] {
  const out: OutlineItem[] = [];
  let fenced = false;
  body.split("\n").forEach((line, index) => {
    if (line.trim().startsWith("```")) {
      fenced = !fenced;
      return;
    }
    if (fenced) {
      return;
    }
    const match = /^(#{1,3})\s+(.+)$/.exec(line.trim());
    if (!match) {
      return;
    }
    const level = Math.min(3, (match[1] ?? "#").length) as 1 | 2 | 3;
    const text = stripMarkdown(match[2] ?? "");
    if (text) {
      out.push({ id: headingId(index, text), level, text, line: index });
    }
  });
  return out;
}

export function wikiTargets(body: string): { kind: WikiKind; id: string; label: string; raw: string }[] {
  const out: { kind: WikiKind; id: string; label: string; raw: string }[] = [];
  for (const match of body.matchAll(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g)) {
    const raw = (match[1] ?? "").trim();
    if (!raw) {
      continue;
    }
    const label = (match[2] ?? "").trim() || raw;
    const prefixed = /^(note|kc|lexeme|book|highlight):(.+)$/i.exec(raw);
    if (prefixed) {
      const kind = (prefixed[1] ?? "").toLocaleLowerCase() as WikiKind;
      out.push({
        kind: kind === "highlight" ? "highlight" : kind,
        id: (prefixed[2] ?? "").trim(),
        label,
        raw,
      });
    } else {
      out.push({ kind: "note", id: raw, label, raw });
    }
  }
  return out;
}

function targetLabel(
  target: { kind: WikiKind; id: string; label: string },
  context: WikiContext,
): string {
  if (target.label && target.label !== target.id) {
    return target.label;
  }
  if (target.kind === "note") {
    const note = context.notes.find(
      (entry) =>
        entry.note_id === target.id ||
        noteTitle(entry).toLocaleLowerCase() === target.id.toLocaleLowerCase(),
    );
    return note ? noteTitle(note) : target.label;
  }
  if (target.kind === "kc") {
    return (
      context.kcs.find((entry) => entry.id === target.id)?.title ?? target.label
    );
  }
  if (target.kind === "lexeme") {
    return (
      context.lexemes.find((entry) => entry.id === target.id)?.lemma ?? target.label
    );
  }
  if (target.kind === "book") {
    return (
      context.books.find((entry) => entry.id === target.id)?.title ?? target.label
    );
  }
  return target.label;
}

export function resolveWikiLink(
  target: { kind: WikiKind; id: string; label: string; raw: string },
  context: WikiContext,
): WikiTarget {
  const resolved = { ...target };
  if (!target.raw.includes(":")) {
    const needle = target.id.toLocaleLowerCase();
    const note =
      context.notes.find((entry) => entry.note_id === target.id) ??
      context.notes.find(
        (entry) => noteTitle(entry).toLocaleLowerCase() === needle,
      );
    const kc = context.kcs.find(
      (entry) =>
        entry.id.toLocaleLowerCase() === needle ||
        entry.title.toLocaleLowerCase() === needle,
    );
    const lexeme = context.lexemes.find(
      (entry) =>
        entry.id.toLocaleLowerCase() === needle ||
        entry.lemma.toLocaleLowerCase() === needle,
    );
    const book = context.books.find(
      (entry) =>
        entry.id.toLocaleLowerCase() === needle ||
        entry.title.toLocaleLowerCase() === needle,
    );
    if (note) {
      resolved.kind = "note";
      resolved.id = note.note_id;
      resolved.label = noteTitle(note);
    } else if (kc) {
      resolved.kind = "kc";
      resolved.id = kc.id;
      resolved.label = kc.title;
    } else if (lexeme) {
      resolved.kind = "lexeme";
      resolved.id = lexeme.id;
      resolved.label = lexeme.lemma;
    } else if (book) {
      resolved.kind = "book";
      resolved.id = book.id;
      resolved.label = book.title;
    }
  }
  return {
    kind: resolved.kind,
    id: resolved.id,
    label: targetLabel(resolved, context),
    target: `${resolved.kind}:${resolved.id}`,
  };
}

export function resolveWikiLinks(body: string, context: WikiContext): WikiTarget[] {
  const seen = new Set<string>();
  const out: WikiTarget[] = [];
  for (const target of wikiTargets(body)) {
    const resolved = resolveWikiLink(target, context);
    if (seen.has(resolved.target)) {
      continue;
    }
    seen.add(resolved.target);
    out.push(resolved);
  }
  return out;
}

export function noteOutgoing(note: NoteView, context: WikiContext): WikiTarget[] {
  return resolveWikiLinks(note.body, context);
}

export function buildBacklinks(
  note: NoteView,
  records: PersonalRecords,
  context: WikiContext,
): Backlinks {
  const notes = Object.values(records.notes).filter((entry) => !entry.deleted);
  const incoming: NoteView[] = [];
  const shared: NoteView[] = [];
  const anchors = new Set<string>();
  for (const other of notes) {
    if (other.note_id === note.note_id) {
      continue;
    }
    const linksToNote = resolveWikiLinks(other.body, context).some(
      (target) =>
        target.kind === "note" &&
        (target.id === note.note_id ||
          target.label.toLocaleLowerCase() === noteTitle(note).toLocaleLowerCase()),
    );
    if (linksToNote) {
      incoming.push(other);
    }
    const anchorMatches =
      note.anchor.kind !== "standalone" &&
      other.anchor.kind === note.anchor.kind &&
      (other.anchor.id ?? null) === (note.anchor.id ?? null);
    if (anchorMatches) {
      shared.push(other);
      if (other.anchor.id) {
        anchors.add(other.anchor.id);
      }
    }
  }
  if (note.anchor.kind === "book_mark" && note.anchor.id) {
    anchors.add(note.anchor.id);
  }
  const highlights = Object.values(records.bookMarks).filter(
    (mark) => !mark.deleted && anchors.has(mark.mark_id),
  );
  return {
    outgoing: noteOutgoing(note, context),
    incoming: incoming.sort((left, right) => right.updated_at - left.updated_at),
    shared: shared.sort((left, right) => right.updated_at - left.updated_at),
    highlights,
  };
}

function addGraphNode(
  nodes: Map<string, LocalGraphNode>,
  node: LocalGraphNode,
): void {
  if (!nodes.has(node.id)) {
    nodes.set(node.id, node);
  }
}

export function localNoteGraph(
  note: NoteView,
  records: PersonalRecords,
  context: WikiContext,
  limit = 12,
): { nodes: LocalGraphNode[]; edges: LocalGraphEdge[] } {
  const center = `note:${note.note_id}`;
  const nodes = new Map<string, LocalGraphNode>();
  const edges: LocalGraphEdge[] = [];
  const edgeKeys = new Set<string>();
  addGraphNode(nodes, { id: center, kind: "note", label: noteTitle(note) });
  const addEdge = (to: string) => {
    const key = `${center}->${to}`;
    if (!edgeKeys.has(key)) {
      edgeKeys.add(key);
      edges.push({ from: center, to });
    }
  };
  const backlinks = buildBacklinks(note, records, context);
  for (const target of [...backlinks.outgoing, ...backlinks.incoming.map((entry) => ({
    kind: "note" as const,
    id: entry.note_id,
    label: noteTitle(entry),
    target: `note:${entry.note_id}`,
  }))]) {
    if (nodes.size >= limit) {
      break;
    }
    addGraphNode(nodes, { id: target.target, kind: target.kind, label: target.label });
    addEdge(target.target);
  }
  for (const related of backlinks.shared) {
    if (nodes.size >= limit) {
      break;
    }
    const id = `note:${related.note_id}`;
    addGraphNode(nodes, { id, kind: "note", label: noteTitle(related) });
    addEdge(id);
  }
  if (
    note.anchor.id &&
    (note.anchor.kind === "kc" ||
      note.anchor.kind === "lexeme" ||
      note.anchor.kind === "book_mark")
  ) {
    const kind: WikiKind =
      note.anchor.kind === "book_mark"
        ? "highlight"
        : note.anchor.kind === "lexeme"
          ? "lexeme"
          : "kc";
    const id = `${kind}:${note.anchor.id}`;
    if (nodes.size < limit) {
      addGraphNode(nodes, {
        id,
        kind,
        label: kind === "highlight" ? "Book highlight" : note.anchor.id,
      });
      addEdge(id);
    }
  }
  return { nodes: [...nodes.values()], edges };
}

export function wikiSuggestions(
  query: string,
  context: WikiContext,
  excludeNoteId?: string,
  limit = 8,
): WikiSuggestion[] {
  const needle = query.trim().toLocaleLowerCase();
  const all: WikiSuggestion[] = [
    ...context.notes
      .filter((note) => note.note_id !== excludeNoteId)
      .map((note) => ({
        target: `note:${note.note_id}`,
        label: noteTitle(note),
        kind: "note" as const,
      })),
    ...context.kcs.map((entry) => ({
      target: `kc:${entry.id}`,
      label: entry.title,
      kind: "kc" as const,
    })),
    ...context.lexemes.map((entry) => ({
      target: `lexeme:${entry.id}`,
      label: entry.lemma,
      kind: "lexeme" as const,
    })),
    ...context.books.map((entry) => ({
      target: `book:${entry.id}`,
      label: entry.title,
      kind: "book" as const,
    })),
  ];
  const seen = new Set<string>();
  return all
    .filter((entry) => entry.label.toLocaleLowerCase().includes(needle))
    .filter((entry) =>
      seen.has(entry.target) ? false : (seen.add(entry.target), true),
    )
    .slice(0, limit);
}

export function activeWikiQuery(
  body: string,
  caret: number,
): { start: number; end: number; query: string } | null {
  const before = body.slice(0, caret);
  const open = before.lastIndexOf("[[");
  if (open < 0) {
    return null;
  }
  const between = before.slice(open + 2);
  if (between.includes("]]") || between.includes("\n") || between.includes("|")) {
    return null;
  }
  return { start: open, end: caret, query: between };
}
