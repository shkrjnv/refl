import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";

import { NoteMarkdown } from "../components/NoteMarkdown";
import { NotesLocalGraph } from "../components/NotesLocalGraph";
import { StudioIcon, type StudioIconName } from "../components/StudioIcons";
import { Button } from "../components/ui";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import { applyFormat, type FormatAction } from "../prototype/noteFormat";
import {
  NOTE_FILTERS,
  activeWikiQuery,
  buildBacklinks,
  localNoteGraph,
  noteFilterOf,
  noteOutline,
  noteSnippet,
  noteTitle,
  noteWordCount,
  resolveWikiLink,
  resolveWikiLinks,
  wikiSuggestions,
  type LocalGraphNode,
  type NoteFilter,
  type OutlineItem,
  type WikiContext,
  type WikiKind,
  type WikiTarget,
} from "../prototype/notes";
import {
  activeNotes,
  type BookMarkView,
  type NoteView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";

export interface NotesWorkspaceProps {
  records: PersonalRecords;
  content: PrototypeContent;
  stateJson: string;
  locale: Locale;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
  books?: LoadedBook[];
  initialNoteId?: string | null;
  onBack?: () => void;
  onPractice?: (kcId: string) => void;
  onOpenBook?: (bookId: string) => void;
}

type InspectorTab = "outline" | "backlinks" | "graph";

interface NoteDetail {
  kind: WikiKind;
  id: string;
  label: string;
}

/** The `note_saved` body cap from docs/16, in UTF-8 bytes. */
const NOTE_LIMIT = 4096;
const DRAFT_PREFIX = "draft:";
const LOCAL_GRAPH_LIMIT = 10;

const TOOLBAR: { action: FormatAction; icon: StudioIconName; key: string }[] = [
  { action: "h1", icon: "heading-1", key: "notes-format-h1" },
  { action: "h2", icon: "heading-2", key: "notes-format-h2" },
  { action: "h3", icon: "heading-3", key: "notes-format-h3" },
  { action: "bold", icon: "bold", key: "notes-format-bold" },
  { action: "italic", icon: "italic", key: "notes-format-italic" },
  { action: "list", icon: "list", key: "notes-format-list" },
  { action: "quote", icon: "quote", key: "notes-format-quote" },
  { action: "callout", icon: "callout", key: "notes-format-callout" },
  { action: "wiki", icon: "wiki-link", key: "notes-format-wiki" },
];

const SHORTCUTS: Record<string, FormatAction> = {
  b: "bold",
  i: "italic",
  k: "wiki",
};

function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

function isDraftId(id: string): boolean {
  return id.startsWith(DRAFT_PREFIX);
}

/** Character offset where a 0-based line starts. */
function lineOffset(text: string, line: number): number {
  let offset = 0;
  for (let index = 0; index < line; index += 1) {
    const next = text.indexOf("\n", offset);
    if (next < 0) {
      return text.length;
    }
    offset = next + 1;
  }
  return offset;
}

interface OutlineNode {
  item: OutlineItem;
  children: OutlineNode[];
}

/** Nests H1 › H2 › H3 so the outline reads as a tree. */
function outlineTree(items: readonly OutlineItem[]): OutlineNode[] {
  const roots: OutlineNode[] = [];
  const stack: OutlineNode[] = [];
  for (const item of items) {
    const node: OutlineNode = { item, children: [] };
    while (stack.length > 0 && (stack[stack.length - 1]?.item.level ?? 0) >= item.level) {
      stack.pop();
    }
    const parent = stack[stack.length - 1];
    if (parent) {
      parent.children.push(node);
    } else {
      roots.push(node);
    }
    stack.push(node);
  }
  return roots;
}

export function NotesWorkspace({
  records,
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
  books = [],
  initialNoteId = null,
  onBack,
  onPractice,
  onOpenBook,
}: NotesWorkspaceProps) {
  const { t } = useTranslation();
  const notes = useMemo(() => activeNotes(records), [records]);
  const firstId = initialNoteId ?? notes[0]?.note_id ?? null;
  const [tabs, setTabs] = useState<string[]>(firstId ? [firstId] : []);
  const [activeTab, setActiveTab] = useState<string | null>(firstId);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [editing, setEditing] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<NoteFilter>("all");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("outline");
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [explorerOpen, setExplorerOpen] = useState(false);
  const [detail, setDetail] = useState<NoteDetail | null>(null);
  const [suggestions, setSuggestions] = useState<ReturnType<typeof wikiSuggestions>>([]);
  const [wikiRange, setWikiRange] = useState<{ start: number; end: number } | null>(null);
  const [savingTab, setSavingTab] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const previewRef = useRef<HTMLDivElement | null>(null);
  const draftCounter = useRef(0);

  // Saving chains state through each write; refs keep the latest values
  // available to the save-on-leave flush after unmount.
  const stateRef = useRef(stateJson);
  stateRef.current = stateJson;
  const draftsRef = useRef(drafts);
  draftsRef.current = drafts;
  const notesRef = useRef(notes);
  notesRef.current = notes;
  const writerRef = useRef(recordsWriter);
  writerRef.current = recordsWriter;
  const onStateChangeRef = useRef(onStateChange);
  onStateChangeRef.current = onStateChange;
  const templateRef = useRef("");
  templateRef.current = `# ${t("notes-untitled")}\n\n`;

  const noteById = (id: string | null): NoteView | null =>
    id ? (notes.find((note) => note.note_id === id) ?? null) : null;

  const isDirty = (id: string): boolean => {
    const draft = drafts[id];
    if (draft === undefined) {
      return false;
    }
    if (isDraftId(id)) {
      return draft.trim() !== "" && draft !== templateRef.current;
    }
    return draft !== (noteById(id)?.body ?? "");
  };

  const savedNote = noteById(activeTab);
  const activeDraft = activeTab ? drafts[activeTab] : undefined;
  const body = activeDraft ?? savedNote?.body ?? "";
  // A new tab has no saved record yet; the inspector still needs a note shape.
  const activeNote: NoteView | null = savedNote
    ? { ...savedNote, body }
    : activeTab && isDraftId(activeTab)
      ? {
          note_id: activeTab,
          anchor: { kind: "standalone", id: null },
          body,
          deleted: false,
          event_id: activeTab,
          updated_at: Date.now(),
        }
      : null;
  const mode: "edit" | "preview" =
    activeTab && editing.includes(activeTab) ? "edit" : "preview";
  const dirty = activeTab ? isDirty(activeTab) : false;
  const bytes = byteLength(body);

  useEffect(() => {
    if (!initialNoteId) {
      return;
    }
    setTabs((current) => (current.includes(initialNoteId) ? current : [...current, initialNoteId]));
    setActiveTab(initialNoteId);
  }, [initialNoteId]);

  // A note deleted elsewhere (another device, the anchored drawer) closes
  // its tab unless this tab holds unsaved text.
  useEffect(() => {
    const known = new Set(notes.map((note) => note.note_id));
    setTabs((current) => {
      const next = current.filter(
        (id) => isDraftId(id) || known.has(id) || draftsRef.current[id] !== undefined,
      );
      return next.length === current.length ? current : next;
    });
  }, [notes]);

  useEffect(() => {
    if (activeTab && tabs.includes(activeTab)) {
      return;
    }
    setActiveTab(tabs[tabs.length - 1] ?? null);
  }, [tabs, activeTab]);

  // Leaving the screen saves every valid unsaved draft (docs/16: save on the
  // button or when leaving, never on each keystroke).
  useEffect(() => {
    return () => {
      const pending = Object.entries(draftsRef.current);
      void (async () => {
        let state = stateRef.current;
        for (const [id, text] of pending) {
          const note = notesRef.current.find((entry) => entry.note_id === id);
          const unchanged = note ? note.body === text : text === templateRef.current;
          if (unchanged || !text.trim() || byteLength(text) > NOTE_LIMIT) {
            continue;
          }
          try {
            const result = await writerRef.current.saveNoteRecord(state, {
              ...(note ? { noteId: note.note_id } : {}),
              anchor: note?.anchor ?? { kind: "standalone" },
              body: text,
            });
            state = result.stateJson;
            onStateChangeRef.current(state);
          } catch {
            // The draft stays unsent; the learner sees the old text next time.
          }
        }
      })();
    };
  }, []);

  const context = useMemo<WikiContext>(
    () => ({
      notes,
      kcs: content.liveKcs().map((entry) => ({
        id: entry.kc.id,
        title: content.title(entry, locale),
      })),
      lexemes: content.lexemes.flatMap((lexeme) =>
        lexeme.senses.map((sense) => ({ id: sense.id, lemma: lexeme.lemma })),
      ),
      books: books.map((book) => ({
        id: book.meta.id,
        title: localizedText(book.meta.title, locale),
      })),
    }),
    [books, content, locale, notes],
  );

  const sentenceIndex = useMemo(() => {
    const map = new Map<string, { text: string; bookId: string; bookTitle: string }>();
    for (const book of books) {
      const bookTitle = localizedText(book.meta.title, locale);
      for (const chapter of book.book.chapters) {
        for (const paragraph of chapter.paragraphs) {
          for (const sentence of paragraph.sentences) {
            map.set(sentence.id, { text: sentence.text, bookId: book.meta.id, bookTitle });
          }
        }
      }
    }
    return map;
  }, [books, locale]);

  const outline = useMemo(() => noteOutline(body), [body]);
  const tree = useMemo(() => outlineTree(outline), [outline]);
  const title = activeNote
    ? body.trim()
      ? noteTitle({ body })
      : t("notes-untitled")
    : t("notes-workspace-title");
  const recordsForLinks = useMemo<PersonalRecords>(
    () =>
      activeNote && !savedNote
        ? { ...records, notes: { ...records.notes, [activeNote.note_id]: activeNote } }
        : records,
    [records, activeNote, savedNote],
  );
  const backlinks = useMemo(
    () =>
      activeNote
        ? buildBacklinks(activeNote, recordsForLinks, context)
        : { outgoing: [], incoming: [], shared: [], highlights: [] },
    [activeNote, recordsForLinks, context],
  );
  const graph = useMemo(
    () =>
      activeNote
        ? localNoteGraph(activeNote, recordsForLinks, context, LOCAL_GRAPH_LIMIT)
        : { nodes: [], edges: [] },
    [activeNote, recordsForLinks, context],
  );

  const counts = useMemo(() => {
    const result: Record<NoteFilter, number> = {
      all: notes.length,
      grammar: 0,
      books: 0,
      vocabulary: 0,
      standalone: 0,
    };
    for (const note of notes) {
      result[noteFilterOf(note)] += 1;
    }
    return result;
  }, [notes]);

  const filteredNotes = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return notes.filter(
      (note) =>
        (filter === "all" || noteFilterOf(note) === filter) &&
        (!needle || `${noteTitle(note)} ${note.body}`.toLocaleLowerCase().includes(needle)),
    );
  }, [notes, query, filter]);

  const kindLabel = (kind: WikiKind): string => t(`notes-kind-${kind}`);

  const anchorLabel = (note: Pick<NoteView, "anchor">): string => {
    switch (note.anchor.kind) {
      case "standalone":
        return t("notes-anchor-standalone");
      case "kc":
      case "misconception":
      case "item":
        return t("notes-anchor-kc");
      case "lexeme":
        return t("notes-anchor-lexeme");
      case "book_mark":
        return t("notes-anchor-book");
      default:
        return note.anchor.kind;
    }
  };

  const anchorValue = (note: Pick<NoteView, "anchor">): string | null => {
    const id = note.anchor.id;
    if (!id) {
      return null;
    }
    if (note.anchor.kind === "kc") {
      return content.kcTitle(id, locale);
    }
    if (note.anchor.kind === "lexeme") {
      return content.sense(id)?.lexeme.lemma ?? id;
    }
    if (note.anchor.kind === "book_mark") {
      const mark = records.bookMarks[id];
      return mark ? (sentenceIndex.get(mark.sentence_id)?.bookTitle ?? mark.book_id) : id;
    }
    return id;
  };

  const highlightText = (mark?: BookMarkView): string => {
    if (!mark) {
      return t("notes-detail-missing");
    }
    const sentence = sentenceIndex.get(mark.sentence_id);
    return sentence ? `“${sentence.text}” — ${sentence.bookTitle}` : mark.sentence_id;
  };

  // A wiki link may carry an alias; the graph names the note itself.
  const noteTitleById = (id: string, fallback: string): string => {
    const note = notes.find((entry) => entry.note_id === id);
    return note ? noteTitle(note) : fallback;
  };

  const graphLabels = useMemo(() => {
    const labels = new Map<string, string>();
    for (const node of graph.nodes) {
      const raw = node.id.slice(node.id.indexOf(":") + 1);
      labels.set(
        node.id,
        node.kind === "highlight"
          ? t("notes-highlight-label")
          : node.kind === "kc"
            ? content.kcTitle(raw, locale)
            : node.kind === "lexeme"
              ? (content.sense(raw)?.lexeme.lemma ?? node.label)
              : node.kind === "note"
                ? raw === activeTab
                  ? title
                  : noteTitleById(raw, node.label)
                : node.label,
      );
    }
    return labels;
  }, [graph.nodes, content, locale, t, activeTab, title, notes]);

  const linkedNotes = useMemo(() => {
    const seen = new Set<string>();
    const out: { note: NoteView; direction: "in" | "out" }[] = [];
    for (const target of backlinks.outgoing) {
      const note =
        target.kind === "note"
          ? (notes.find((entry) => entry.note_id === target.id) ?? null)
          : null;
      if (note && !seen.has(note.note_id) && note.note_id !== activeTab) {
        seen.add(note.note_id);
        out.push({ note, direction: "out" });
      }
    }
    for (const note of backlinks.incoming) {
      if (!seen.has(note.note_id)) {
        seen.add(note.note_id);
        out.push({ note, direction: "in" });
      }
    }
    return out;
  }, [backlinks, activeTab, notes]);

  const linkedHighlights = useMemo(() => {
    const marks = new Map<string, BookMarkView>();
    if (activeNote?.anchor.kind === "book_mark" && activeNote.anchor.id) {
      const mark = records.bookMarks[activeNote.anchor.id];
      if (mark && !mark.deleted) {
        marks.set(mark.mark_id, mark);
      }
    }
    for (const mark of backlinks.highlights) {
      marks.set(mark.mark_id, mark);
    }
    for (const target of backlinks.outgoing) {
      const mark = target.kind === "highlight" ? records.bookMarks[target.id] : undefined;
      if (mark && !mark.deleted) {
        marks.set(mark.mark_id, mark);
      }
    }
    return [...marks.values()];
  }, [activeNote, backlinks, records.bookMarks]);

  const setDraft = (value: string) => {
    if (!activeTab) {
      return;
    }
    setDrafts((current) => ({ ...current, [activeTab]: value }));
    setError(null);
  };

  const openNote = (id: string) => {
    setTabs((current) => (current.includes(id) ? current : [...current, id]));
    setActiveTab(id);
    setExplorerOpen(false);
    setDetail(null);
    setSuggestions([]);
    setConfirmDelete(false);
    setError(null);
  };

  const startNew = () => {
    draftCounter.current += 1;
    const id = `${DRAFT_PREFIX}${draftCounter.current}`;
    const template = templateRef.current;
    setDrafts((current) => ({ ...current, [id]: template }));
    setEditing((current) => [...current, id]);
    setTabs((current) => [...current, id]);
    setActiveTab(id);
    setExplorerOpen(false);
    setDetail(null);
    setConfirmDelete(false);
    setError(null);
    requestAnimationFrame(() => {
      const textarea = textareaRef.current;
      textarea?.focus();
      // Select the placeholder title so typing replaces it.
      textarea?.setSelectionRange(2, template.trimEnd().length);
    });
  };

  const setMode = (next: "edit" | "preview") => {
    if (!activeTab) {
      return;
    }
    setEditing((current) =>
      next === "edit"
        ? current.includes(activeTab)
          ? current
          : [...current, activeTab]
        : current.filter((id) => id !== activeTab),
    );
    setSuggestions([]);
    if (next === "edit") {
      requestAnimationFrame(() => textareaRef.current?.focus());
    }
  };

  /** Writes one tab's draft as `note_saved`; returns the saved note id. */
  const saveTab = async (id: string): Promise<string | null> => {
    const text = drafts[id];
    if (text === undefined) {
      return id;
    }
    if (!text.trim()) {
      setError(t("notes-error-empty"));
      return null;
    }
    if (byteLength(text) > NOTE_LIMIT) {
      setError(t("notes-error-size"));
      return null;
    }
    const note = noteById(id);
    setSavingTab(id);
    setError(null);
    try {
      const result = await recordsWriter.saveNoteRecord(stateRef.current, {
        ...(note ? { noteId: note.note_id } : {}),
        anchor: note?.anchor ?? { kind: "standalone" },
        body: text,
      });
      stateRef.current = result.stateJson;
      onStateChange(result.stateJson);
      const savedId = result.noteId;
      setDrafts((current) => {
        const next = { ...current };
        delete next[id];
        return next;
      });
      if (savedId !== id) {
        const swap = (list: string[]) =>
          list.includes(savedId)
            ? list.filter((entry) => entry !== id)
            : list.map((entry) => (entry === id ? savedId : entry));
        setTabs(swap);
        setEditing(swap);
        setActiveTab((current) => (current === id ? savedId : current));
      }
      return savedId;
    } catch {
      setError(t("notes-error-save"));
      return null;
    } finally {
      setSavingTab(null);
    }
  };

  const closeTab = async (id: string) => {
    let closing = id;
    if (isDirty(id)) {
      const saved = await saveTab(id);
      if (!saved) {
        setActiveTab(id);
        return;
      }
      closing = saved;
    }
    setDrafts((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
    setEditing((current) => current.filter((entry) => entry !== id && entry !== closing));
    setTabs((current) => current.filter((entry) => entry !== id && entry !== closing));
  };

  const remove = async () => {
    if (!activeTab) {
      return;
    }
    setConfirmDelete(false);
    if (!savedNote) {
      // Nothing was written for a new tab; dropping it is enough.
      await closeTabWithoutSaving(activeTab);
      return;
    }
    setSavingTab(activeTab);
    setError(null);
    try {
      const next = await recordsWriter.deleteNote(stateRef.current, savedNote.note_id);
      stateRef.current = next;
      onStateChange(next);
      await closeTabWithoutSaving(activeTab);
    } catch {
      setError(t("notes-error-save"));
    } finally {
      setSavingTab(null);
    }
  };

  const closeTabWithoutSaving = async (id: string) => {
    setDrafts((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
    setEditing((current) => current.filter((entry) => entry !== id));
    setTabs((current) => current.filter((entry) => entry !== id));
  };

  const navigateTarget = (target: WikiTarget) => {
    if (target.kind === "note") {
      if (noteById(target.id)) {
        openNote(target.id);
      }
      return;
    }
    setDetail({ kind: target.kind, id: target.id, label: target.label });
    setInspectorOpen(true);
  };

  const handleWikiLink = (raw: string, label: string) => {
    const parsed = /^(note|kc|lexeme|book|highlight):(.+)$/i.exec(raw);
    const kind = (parsed?.[1]?.toLocaleLowerCase() ?? "note") as WikiKind;
    const id = parsed?.[2]?.trim() ?? raw;
    navigateTarget(resolveWikiLink({ kind, id, label, raw }, context));
  };

  const jumpToHeading = (item: OutlineItem) => {
    setInspectorOpen(false);
    if (mode === "edit") {
      const textarea = textareaRef.current;
      if (textarea) {
        const offset = lineOffset(body, item.line);
        textarea.focus();
        textarea.setSelectionRange(offset, offset + (body.split("\n")[item.line]?.length ?? 0));
        const lineHeight = parseFloat(getComputedStyle(textarea).lineHeight) || 24;
        textarea.scrollTop = Math.max(0, item.line * lineHeight - lineHeight);
      }
      return;
    }
    requestAnimationFrame(() => {
      const target = previewRef.current?.querySelector<HTMLElement>(`#${CSS.escape(item.id)}`);
      target?.scrollIntoView?.({ behavior: "smooth", block: "start" });
    });
  };

  const updateWikiQuery = (value: string, caret: number) => {
    const active = activeWikiQuery(value, caret);
    if (!active) {
      setSuggestions([]);
      setWikiRange(null);
      return;
    }
    setSuggestions(wikiSuggestions(active.query, context, activeTab ?? undefined));
    setWikiRange({ start: active.start, end: active.end });
  };

  const insertWiki = (target: string, label: string) => {
    if (!wikiRange) {
      return;
    }
    // Replace the typed `[[query` (and an auto-inserted `]]`) with the link.
    const after = body.slice(wikiRange.end).startsWith("]]") ? wikiRange.end + 2 : wikiRange.end;
    const inserted = `[[${target}|${label}]]`;
    const next = body.slice(0, wikiRange.start) + inserted + body.slice(after);
    setDraft(next);
    setSuggestions([]);
    setWikiRange(null);
    const caret = wikiRange.start + inserted.length;
    requestAnimationFrame(() => {
      textareaRef.current?.focus();
      textareaRef.current?.setSelectionRange(caret, caret);
    });
  };

  const format = (action: FormatAction) => {
    const textarea = textareaRef.current;
    if (!textarea || mode !== "edit") {
      return;
    }
    const edit = applyFormat(body, textarea.selectionStart, textarea.selectionEnd, action);
    setDraft(edit.text);
    requestAnimationFrame(() => {
      textarea.focus();
      textarea.setSelectionRange(edit.start, edit.end);
      if (action === "wiki") {
        updateWikiQuery(edit.text, edit.start);
      }
    });
  };

  const onEditorKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Escape" && suggestions.length > 0) {
      event.stopPropagation();
      setSuggestions([]);
      return;
    }
    if (!(event.ctrlKey || event.metaKey) || event.altKey) {
      return;
    }
    const key = event.key.toLocaleLowerCase();
    if (key === "s") {
      event.preventDefault();
      if (activeTab) {
        void saveTab(activeTab);
      }
      return;
    }
    const action = SHORTCUTS[key];
    if (action) {
      event.preventDefault();
      format(action);
    }
  };

  const selectGraphNode = (node: LocalGraphNode) => {
    const raw = node.id.slice(node.id.indexOf(":") + 1);
    if (node.kind === "note") {
      if (noteById(raw)) {
        openNote(raw);
      }
      return;
    }
    navigateTarget({ kind: node.kind, id: raw, label: node.label, target: node.id });
  };

  const tabTitle = (id: string): string => {
    const text = drafts[id] ?? noteById(id)?.body ?? "";
    return text.trim() ? noteTitle({ body: text }) : t("notes-untitled");
  };

  const statusText = !activeNote
    ? null
    : savingTab === activeTab
      ? t("notes-status-saving")
      : dirty
        ? t("notes-status-unsaved")
        : savedNote
          ? t("notes-status-saved")
          : t("notes-status-unsaved");

  const detailContent = detail ? (
    <div className="notes-detail">
      <header>
        <span className={`notes-kind notes-kind-${detail.kind}`}>{kindLabel(detail.kind)}</span>
        <button
          type="button"
          className="notes-icon-button"
          onClick={() => setDetail(null)}
          aria-label={t("notes-detail-close")}
        >
          <StudioIcon name="close" size={14} />
        </button>
      </header>
      <strong>
        {detail.kind === "kc"
          ? content.kcTitle(detail.id, locale)
          : detail.kind === "lexeme"
            ? (content.sense(detail.id)?.lexeme.lemma ?? detail.label)
            : detail.kind === "highlight"
              ? highlightText(records.bookMarks[detail.id])
              : detail.label}
      </strong>
      {detail.kind === "lexeme" ? (
        <>
          <p>{content.sense(detail.id)?.sense.definition_en ?? t("notes-detail-missing")}</p>
          {locale === "uz" && content.sense(detail.id)?.sense.gloss_uz ? (
            <p className="bento-note">{content.sense(detail.id)?.sense.gloss_uz}</p>
          ) : null}
          {locale === "ru" && content.sense(detail.id)?.sense.gloss_ru ? (
            <p className="bento-note">{content.sense(detail.id)?.sense.gloss_ru}</p>
          ) : null}
        </>
      ) : null}
      {detail.kind === "kc" && onPractice ? (
        <Button type="button" variant="secondary" size="sm" onClick={() => onPractice(detail.id)}>
          {t("graph-practice")}
        </Button>
      ) : null}
      {(detail.kind === "book" || detail.kind === "highlight") && onOpenBook ? (
        <Button
          type="button"
          variant="secondary"
          size="sm"
          onClick={() =>
            onOpenBook(
              detail.kind === "book" ? detail.id : (records.bookMarks[detail.id]?.book_id ?? ""),
            )
          }
        >
          {t("library-open-book")}
        </Button>
      ) : null}
    </div>
  ) : null;

  const renderOutline = (nodes: OutlineNode[]): ReactNode => (
    <ul>
      {nodes.map((node) => (
        <li key={node.item.id}>
          <button
            type="button"
            className={`notes-outline-item notes-outline-${node.item.level}`}
            onClick={() => jumpToHeading(node.item)}
          >
            <span className="notes-outline-mark" aria-hidden="true">
              {"#".repeat(node.item.level)}
            </span>
            {node.item.text}
          </button>
          {node.children.length > 0 ? renderOutline(node.children) : null}
        </li>
      ))}
    </ul>
  );

  const properties: [string, ReactNode][] = activeNote
    ? [
        [t("notes-prop-anchor"), anchorLabel(activeNote)],
        ...(anchorValue(activeNote)
          ? [[t("notes-prop-source"), anchorValue(activeNote)] as [string, ReactNode]]
          : []),
        [
          t("notes-prop-edited"),
          savedNote ? new Date(savedNote.updated_at).toLocaleString(locale) : "—",
        ],
        [t("notes-prop-words"), noteWordCount(body)],
        [t("notes-prop-size"), t("notes-count", { used: bytes, limit: NOTE_LIMIT })],
        [t("notes-prop-links-out"), resolveWikiLinks(body, context).length],
        [t("notes-prop-links-in"), backlinks.incoming.length],
      ]
    : [];

  const inspector = (
    <aside
      className={`notes-inspector${inspectorOpen ? " open" : ""}`}
      aria-label={t("notes-inspector")}
    >
      <header className="notes-pane-head">
        <div className="notes-inspector-tabs" role="tablist" aria-label={t("notes-inspector")}>
          {(["outline", "backlinks", "graph"] as const).map((entry) => (
            <button
              key={entry}
              type="button"
              role="tab"
              aria-selected={inspectorTab === entry}
              className={`notes-inspector-tab${inspectorTab === entry ? " active" : ""}`}
              onClick={() => setInspectorTab(entry)}
            >
              {t(`notes-tab-${entry}`)}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="notes-icon-button notes-drawer-close"
          aria-label={t("notes-close")}
          onClick={() => setInspectorOpen(false)}
        >
          <StudioIcon name="close" size={16} />
        </button>
      </header>
      <div className="notes-inspector-body">
        {detailContent}
        {!activeNote ? (
          <p className="bento-empty">{t("notes-select-hint")}</p>
        ) : inspectorTab === "outline" ? (
          tree.length === 0 ? (
            <p className="bento-empty">{t("notes-outline-empty")}</p>
          ) : (
            <nav className="notes-outline" aria-label={t("notes-tab-outline")}>
              {renderOutline(tree)}
            </nav>
          )
        ) : inspectorTab === "backlinks" ? (
          <div className="notes-backlinks">
            <section>
              <h4>{t("notes-properties")}</h4>
              <table className="notes-properties">
                <tbody>
                  {properties.map(([label, value]) => (
                    <tr key={label}>
                      <th scope="row">{label}</th>
                      <td>{value}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
            {backlinks.incoming.length > 0 ? (
              <section>
                <h4>{t("notes-backlinks-incoming")}</h4>
                <ul>
                  {backlinks.incoming.map((note) => (
                    <li key={note.note_id}>
                      <button type="button" onClick={() => openNote(note.note_id)}>
                        <span className="notes-kind notes-kind-note">{kindLabel("note")}</span>
                        {noteTitle(note)}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            {backlinks.outgoing.length > 0 ? (
              <section>
                <h4>{t("notes-backlinks-outgoing")}</h4>
                <ul>
                  {backlinks.outgoing.map((target) => (
                    <li key={target.target}>
                      <button type="button" onClick={() => navigateTarget(target)}>
                        <span className={`notes-kind notes-kind-${target.kind}`}>
                          {kindLabel(target.kind)}
                        </span>
                        {target.kind === "kc"
                          ? content.kcTitle(target.id, locale)
                          : target.label}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            {backlinks.shared.length > 0 ? (
              <section>
                <h4>{t("notes-backlinks-shared")}</h4>
                <ul>
                  {backlinks.shared.map((note) => (
                    <li key={note.note_id}>
                      <button type="button" onClick={() => openNote(note.note_id)}>
                        <span className={`notes-kind notes-kind-${note.anchor.kind}`}>
                          {anchorLabel(note)}
                        </span>
                        {noteTitle(note)}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            {linkedHighlights.length > 0 ? (
              <section>
                <h4>{t("notes-backlinks-highlights")}</h4>
                <ul>
                  {linkedHighlights.map((mark) => (
                    <li key={mark.mark_id}>
                      <button
                        type="button"
                        onClick={() =>
                          navigateTarget({
                            kind: "highlight",
                            id: mark.mark_id,
                            label: t("notes-highlight-label"),
                            target: `highlight:${mark.mark_id}`,
                          })
                        }
                      >
                        <span className="notes-kind notes-kind-highlight">
                          {kindLabel("highlight")}
                        </span>
                        {highlightText(mark)}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            {backlinks.outgoing.length === 0 &&
            backlinks.incoming.length === 0 &&
            backlinks.shared.length === 0 &&
            linkedHighlights.length === 0 ? (
              <p className="bento-empty">{t("notes-backlinks-empty")}</p>
            ) : null}
          </div>
        ) : (
          <div className="notes-local-graph-wrap">
            {graph.nodes.length <= 1 ? (
              <p className="bento-empty">{t("notes-graph-empty")}</p>
            ) : (
              <NotesLocalGraph
                nodes={graph.nodes}
                edges={graph.edges}
                labels={graphLabels}
                centerId={`note:${activeNote.note_id}`}
                ariaLabel={t("notes-graph-label")}
                kindLabel={kindLabel}
                onSelect={selectGraphNode}
              />
            )}
            <p className="bento-note">{t("notes-graph-note")}</p>
          </div>
        )}
      </div>
    </aside>
  );

  const explorer = (
    <aside
      className={`notes-explorer${explorerOpen ? " open" : ""}`}
      aria-label={t("notes-explorer")}
    >
      <header className="notes-pane-head">
        <div className="notes-explorer-title">
          <h2>{t("notes-workspace-title")}</h2>
          <span>{t("notes-total", { count: notes.length })}</span>
        </div>
        <button
          type="button"
          className="notes-icon-button notes-drawer-close"
          aria-label={t("notes-close")}
          onClick={() => setExplorerOpen(false)}
        >
          <StudioIcon name="close" size={16} />
        </button>
      </header>
      <Button type="button" variant="primary" size="sm" className="notes-new" onClick={startNew}>
        <StudioIcon name="plus" size={16} />
        {t("notes-new")}
      </Button>
      <label className="notes-search">
        <StudioIcon name="search" size={16} />
        <span className="sr-only">{t("notes-search")}</span>
        <input
          type="search"
          value={query}
          placeholder={t("notes-search")}
          onChange={(event) => setQuery(event.currentTarget.value)}
        />
      </label>
      <div className="notes-filters" role="group" aria-label={t("notes-filter-label")}>
        {NOTE_FILTERS.map((entry) => (
          <button
            key={entry}
            type="button"
            className={`notes-filter${filter === entry ? " active" : ""}`}
            aria-pressed={filter === entry}
            onClick={() => setFilter(entry)}
          >
            {t(`notes-filter-${entry}`)}
            <span className="notes-filter-count">{counts[entry]}</span>
          </button>
        ))}
      </div>
      <ul className="notes-explorer-list">
        {filteredNotes.map((note) => {
          const snippet = noteSnippet(note.body);
          return (
            <li key={note.note_id}>
              <button
                type="button"
                className={`notes-explorer-row${activeTab === note.note_id ? " active" : ""}`}
                aria-current={activeTab === note.note_id ? "true" : undefined}
                onClick={() => openNote(note.note_id)}
              >
                <strong>{noteTitle(note)}</strong>
                {snippet ? <span className="notes-row-snippet">{snippet}</span> : null}
                <span className="notes-row-meta">
                  <span className={`notes-kind notes-kind-${note.anchor.kind}`}>
                    {anchorLabel(note)}
                  </span>
                  <span>{new Date(note.updated_at).toLocaleDateString(locale)}</span>
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      {notes.length === 0 ? (
        <p className="bento-empty">{t("notes-workspace-empty")}</p>
      ) : filteredNotes.length === 0 ? (
        <p className="bento-empty">{t("notes-filter-empty")}</p>
      ) : null}
    </aside>
  );

  return (
    <section
      className="screen notes-workspace"
      aria-labelledby="notes-workspace-title"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setExplorerOpen(false);
          setInspectorOpen(false);
          setConfirmDelete(false);
        }
      }}
    >
      <header className="notes-workspace-head">
        {onBack ? (
          <Button type="button" variant="ghost" size="sm" onClick={onBack}>
            <StudioIcon name="arrow-left" size={16} />
            {t("notes-back")}
          </Button>
        ) : null}
        <div>
          <p className="eyebrow">{t("notes-eyebrow")}</p>
          <h1 id="notes-workspace-title">{t("notes-workspace-title")}</h1>
        </div>
      </header>
      <div className="notes-workspace-grid">
        {explorer}
        <main className="notes-editor-pane">
          <div className="notes-tabbar">
            <button
              type="button"
              className="notes-icon-button notes-explorer-toggle"
              aria-label={t("notes-explorer-show")}
              title={t("notes-explorer-show")}
              aria-expanded={explorerOpen}
              onClick={() => setExplorerOpen(true)}
            >
              <StudioIcon name="panel-left" size={18} />
            </button>
            <div className="notes-tabs" role="tablist" aria-label={t("notes-tabs")}>
              {tabs.map((id) => {
                const name = tabTitle(id);
                return (
                  <div
                    key={id}
                    className={`notes-tab${id === activeTab ? " active" : ""}`}
                  >
                    <button
                      type="button"
                      role="tab"
                      aria-selected={id === activeTab}
                      className="notes-tab-main"
                      title={name}
                      onClick={() => {
                        setActiveTab(id);
                        setConfirmDelete(false);
                        setSuggestions([]);
                      }}
                    >
                      {isDirty(id) ? (
                        <span className="notes-tab-dirty" aria-label={t("notes-status-unsaved")} />
                      ) : null}
                      <span className="notes-tab-title">{name}</span>
                    </button>
                    <button
                      type="button"
                      className="notes-tab-close"
                      aria-label={t("notes-tab-close", { title: name })}
                      onClick={() => void closeTab(id)}
                    >
                      <StudioIcon name="close" size={13} />
                    </button>
                  </div>
                );
              })}
              <button
                type="button"
                className="notes-tab-new"
                aria-label={t("notes-new")}
                title={t("notes-new")}
                onClick={startNew}
              >
                <StudioIcon name="plus" size={16} />
              </button>
            </div>
            <button
              type="button"
              className="notes-icon-button notes-inspector-toggle"
              aria-label={t("notes-inspector-show")}
              title={t("notes-inspector-show")}
              aria-expanded={inspectorOpen}
              onClick={() => setInspectorOpen(true)}
            >
              <StudioIcon name="panel-right" size={18} />
            </button>
          </div>

          {activeNote ? (
            <>
              <div className="notes-doc-head">
                <h2>{title}</h2>
                <div className="notes-doc-actions">
                  <div className="notes-mode-toggle" role="group" aria-label={t("notes-view-mode")}>
                    <button
                      type="button"
                      className={mode === "edit" ? "active" : ""}
                      aria-pressed={mode === "edit"}
                      onClick={() => setMode("edit")}
                    >
                      <StudioIcon name="pencil" size={14} />
                      {t("notes-view-edit")}
                    </button>
                    <button
                      type="button"
                      className={mode === "preview" ? "active" : ""}
                      aria-pressed={mode === "preview"}
                      onClick={() => setMode("preview")}
                    >
                      <StudioIcon name="preview" size={14} />
                      {t("notes-view-read")}
                    </button>
                  </div>
                  <button
                    type="button"
                    className="notes-icon-button notes-delete"
                    aria-label={t("notes-delete")}
                    title={t("notes-delete")}
                    onClick={() => setConfirmDelete(true)}
                  >
                    <StudioIcon name="trash" size={16} />
                  </button>
                </div>
              </div>
              {confirmDelete ? (
                <div className="notes-confirm" role="alertdialog" aria-label={t("notes-delete")}>
                  <p>{t("notes-delete-confirm")}</p>
                  <div>
                    <Button type="button" variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                      {t("notes-delete-keep")}
                    </Button>
                    <Button
                      type="button"
                      variant="danger"
                      size="sm"
                      busy={savingTab === activeTab}
                      onClick={() => void remove()}
                    >
                      {t("notes-delete")}
                    </Button>
                  </div>
                </div>
              ) : null}
              <div className="notes-chips">
                <span className={`notes-kind notes-kind-${activeNote.anchor.kind}`}>
                  {anchorLabel(activeNote)}
                </span>
                {anchorValue(activeNote) ? (
                  <span className="notes-chip">{anchorValue(activeNote)}</span>
                ) : null}
                <span className="notes-chip">{t("notes-private")}</span>
                {savedNote ? (
                  <span className="notes-chip">
                    {t("notes-edited", {
                      date: new Date(savedNote.updated_at).toLocaleDateString(locale),
                    })}
                  </span>
                ) : null}
              </div>
              <div
                className="notes-toolbar"
                role="toolbar"
                aria-label={t("notes-toolbar")}
                title={mode === "preview" ? t("notes-format-preview-hint") : undefined}
              >
                {TOOLBAR.map((entry, index) => (
                  <button
                    key={entry.action}
                    type="button"
                    className={`notes-tool${index === 3 || index === 5 || index === 8 ? " group-start" : ""}`}
                    aria-label={t(entry.key)}
                    title={t(entry.key)}
                    disabled={mode !== "edit"}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => format(entry.action)}
                  >
                    <StudioIcon name={entry.icon} size={16} />
                  </button>
                ))}
              </div>
              {error ? (
                <p className="inline-alert danger" role="alert">
                  {error}
                </p>
              ) : null}
              {mode === "edit" ? (
                <div className="notes-editor">
                  <textarea
                    ref={textareaRef}
                    className="notes-textarea"
                    value={body}
                    spellCheck
                    aria-label={t("notes-edit-label")}
                    placeholder={t("notes-placeholder")}
                    onChange={(event) => {
                      const value = event.currentTarget.value;
                      setDraft(value);
                      updateWikiQuery(value, event.currentTarget.selectionStart);
                    }}
                    onKeyDown={onEditorKeyDown}
                    onKeyUp={(event) => {
                      if (event.key.startsWith("Arrow")) {
                        updateWikiQuery(body, event.currentTarget.selectionStart);
                      }
                    }}
                    onClick={(event) => updateWikiQuery(body, event.currentTarget.selectionStart)}
                  />
                  {suggestions.length > 0 ? (
                    <div
                      className="notes-wiki-suggestions"
                      role="listbox"
                      aria-label={t("notes-wiki-suggestions")}
                    >
                      {suggestions.map((suggestion) => (
                        <button
                          key={suggestion.target}
                          type="button"
                          role="option"
                          aria-selected="false"
                          onMouseDown={(event) => event.preventDefault()}
                          onClick={() => insertWiki(suggestion.target, suggestion.label)}
                        >
                          <span className={`notes-kind notes-kind-${suggestion.kind}`}>
                            {kindLabel(suggestion.kind)}
                          </span>
                          {suggestion.label}
                        </button>
                      ))}
                    </div>
                  ) : null}
                  <div className="notes-editor-actions">
                    <Button
                      type="button"
                      variant="primary"
                      size="sm"
                      busy={savingTab === activeTab}
                      disabled={!dirty || !body.trim() || bytes > NOTE_LIMIT}
                      onClick={() => activeTab && void saveTab(activeTab)}
                    >
                      {t("notes-save")}
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="notes-preview" ref={previewRef}>
                  {body.trim() ? (
                    <NoteMarkdown
                      body={body}
                      onWikiLink={handleWikiLink}
                      className="notes-reader"
                      omitFirstHeading
                    />
                  ) : (
                    <p className="bento-empty">{t("notes-empty-body")}</p>
                  )}
                  {linkedHighlights.length > 0 ? (
                    <details className="notes-block notes-block-highlights" open>
                      <summary>
                        {t("notes-backlinks-highlights")}
                        <span className="notes-filter-count">{linkedHighlights.length}</span>
                      </summary>
                      {linkedHighlights.map((mark) => {
                        const sentence = sentenceIndex.get(mark.sentence_id);
                        return (
                          <blockquote key={mark.mark_id}>
                            “{sentence?.text ?? t("notes-detail-missing")}”
                            {sentence ? <cite>— {sentence.bookTitle}</cite> : null}
                          </blockquote>
                        );
                      })}
                    </details>
                  ) : null}
                  <details className="notes-block notes-block-links" open>
                    <summary>
                      {t("notes-linked-notes")}
                      <span className="notes-filter-count">{linkedNotes.length}</span>
                    </summary>
                    {linkedNotes.length === 0 ? (
                      <p className="bento-note">{t("notes-linked-empty")}</p>
                    ) : (
                      <ul>
                        {linkedNotes.map(({ note, direction }) => (
                          <li key={note.note_id}>
                            <button type="button" onClick={() => openNote(note.note_id)}>
                              <strong>{noteTitle(note)}</strong>
                              <span>{t(direction === "in" ? "notes-linked-in" : "notes-linked-out")}</span>
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </details>
                </div>
              )}
              <footer className="notes-statusbar" aria-label={t("notes-status-label")}>
                <span className={`notes-save-state${dirty ? " dirty" : ""}`} aria-live="polite">
                  {statusText}
                </span>
                <span>{t("notes-word-count", { count: noteWordCount(body) })}</span>
                <span>{t("notes-char-count", { count: body.length })}</span>
                <span className={bytes > NOTE_LIMIT ? "over" : ""}>
                  {t("notes-count", { used: bytes, limit: NOTE_LIMIT })}
                </span>
                {mode === "edit" ? (
                  <span className="notes-shortcut">{t("notes-shortcut-hint")}</span>
                ) : null}
              </footer>
            </>
          ) : (
            <div className="notes-empty-state">
              <StudioIcon name="note" size={28} />
              <p>{notes.length === 0 ? t("notes-workspace-empty") : t("notes-select-hint")}</p>
              <div>
                <Button type="button" variant="primary" size="sm" onClick={startNew}>
                  {t("notes-new")}
                </Button>
                {notes.length > 0 ? (
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className="notes-explorer-toggle"
                    onClick={() => setExplorerOpen(true)}
                  >
                    {t("notes-explorer-show")}
                  </Button>
                ) : null}
              </div>
            </div>
          )}
        </main>
        {inspector}
      </div>
      {explorerOpen || inspectorOpen ? (
        <button
          type="button"
          className="notes-scrim"
          aria-label={t("notes-close")}
          onClick={() => {
            setExplorerOpen(false);
            setInspectorOpen(false);
          }}
        />
      ) : null}
    </section>
  );
}
