import { useEffect, useMemo, useRef, useState } from "react";

import { NoteMarkdown } from "../components/NoteMarkdown";
import { Button } from "../components/ui";
import { StudioIcon, type StudioIconName } from "../components/StudioIcons";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import {
  activeNotes,
  type BookMarkView,
  type NoteView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import {
  activeWikiQuery,
  buildBacklinks,
  localNoteGraph,
  noteOutline,
  noteTitle,
  noteWordCount,
  resolveWikiLink,
  wikiSuggestions,
  type LocalGraphNode,
  type WikiContext,
  type WikiKind,
  type WikiTarget,
} from "../prototype/notes";

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
type ViewMode = "read" | "edit";

interface NoteDetail {
  kind: WikiKind;
  id: string;
  label: string;
}

const GRAPH_ICON: Record<WikiKind, StudioIconName> = {
  note: "note",
  kc: "practice",
  lexeme: "vocabulary",
  book: "library",
  highlight: "bookmark",
};

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
  const [selectedId, setSelectedId] = useState<string | null>(firstId);
  const [mode, setMode] = useState<ViewMode>("read");
  const [draft, setDraft] = useState("");
  const [query, setQuery] = useState("");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("outline");
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [detail, setDetail] = useState<NoteDetail | null>(null);
  const [suggestions, setSuggestions] = useState<
    ReturnType<typeof wikiSuggestions>
  >([]);
  const [wikiRange, setWikiRange] = useState<{ start: number; end: number } | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const selectedNote =
    notes.find((note) => note.note_id === selectedId) ?? null;

  useEffect(() => {
    if (initialNoteId) {
      setSelectedId(initialNoteId);
      setMode("read");
    }
  }, [initialNoteId]);

  useEffect(() => {
    if (mode !== "read") {
      return;
    }
    if (!selectedId) {
      if (notes[0]) {
        setSelectedId(notes[0].note_id);
      }
      return;
    }
    if (!notes.some((note) => note.note_id === selectedId)) {
      setSelectedId(notes[0]?.note_id ?? null);
    }
  }, [notes, selectedId, mode]);

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
    const map = new Map<
      string,
      { text: string; bookId: string; bookTitle: string }
    >();
    for (const book of books) {
      const bookTitle = localizedText(book.meta.title, locale);
      for (const chapter of book.book.chapters) {
        for (const paragraph of chapter.paragraphs) {
          for (const sentence of paragraph.sentences) {
            map.set(sentence.id, {
              text: sentence.text,
              bookId: book.meta.id,
              bookTitle,
            });
          }
        }
      }
    }
    return map;
  }, [books, locale]);

  const activeBody = selectedNote?.body ?? "";
  const shownBody = mode === "edit" ? draft : activeBody;
  const outline = useMemo(() => noteOutline(shownBody), [shownBody]);
  const title = selectedNote
    ? noteTitle(selectedNote)
    : draft.trim()
      ? noteTitle({ body: draft })
      : t("notes-untitled");
  const bytes = new TextEncoder().encode(draft).length;
  const backlinks = useMemo(
    () =>
      selectedNote
        ? buildBacklinks(selectedNote, records, context)
        : { outgoing: [], incoming: [], shared: [], highlights: [] },
    [context, records, selectedNote],
  );
  const graph = useMemo(
    () =>
      selectedNote
        ? localNoteGraph(selectedNote, records, context)
        : { nodes: [], edges: [] },
    [context, records, selectedNote],
  );
  const filteredNotes = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    if (!needle) {
      return notes;
    }
    return notes.filter((note) =>
      `${noteTitle(note)} ${note.body}`.toLocaleLowerCase().includes(needle),
    );
  }, [notes, query]);

  const linkedHighlight =
    selectedNote?.anchor.kind === "book_mark" && selectedNote.anchor.id
      ? records.bookMarks[selectedNote.anchor.id] ?? null
      : null;
  const highlightText = linkedHighlight
    ? sentenceIndex.get(linkedHighlight.sentence_id) ?? null
    : null;

  const selectNote = (note: NoteView) => {
    setSelectedId(note.note_id);
    setMode("read");
    setDetail(null);
    setSuggestions([]);
  };

  const startNew = () => {
    setSelectedId(null);
    setDraft(`# ${t("notes-untitled")}\n\n`);
    setMode("edit");
    setDetail(null);
    setError(null);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  const startEdit = () => {
    if (!selectedNote) {
      startNew();
      return;
    }
    setDraft(selectedNote.body);
    setMode("edit");
    setError(null);
    requestAnimationFrame(() => textareaRef.current?.focus());
  };

  const save = async () => {
    if (!draft.trim() || bytes > 4096) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await recordsWriter.saveNoteRecord(stateJson, {
        ...(selectedNote ? { noteId: selectedNote.note_id } : {}),
        anchor: selectedNote?.anchor ?? { kind: "standalone" },
        body: draft,
      });
      onStateChange(result.stateJson);
      setSelectedId(result.noteId);
      setMode("read");
      setSuggestions([]);
    } catch {
      setError(t("notes-error-save"));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!selectedNote) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      onStateChange(await recordsWriter.deleteNote(stateJson, selectedNote.note_id));
      setSelectedId(null);
      setMode("read");
      setDetail(null);
    } catch {
      setError(t("notes-error-save"));
    } finally {
      setBusy(false);
    }
  };

  const navigateTarget = (target: WikiTarget) => {
    if (target.kind === "note") {
      const note = notes.find((entry) => entry.note_id === target.id);
      if (note) {
        selectNote(note);
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
    navigateTarget(
      resolveWikiLink(
        { kind, id, label, raw },
        context,
      ),
    );
  };

  const jumpToHeading = (id: string) => {
    setMode("read");
    requestAnimationFrame(() => {
      document.getElementById(id)?.scrollIntoView({
        behavior: "smooth",
        block: "start",
      });
    });
  };

  const updateWikiQuery = (value: string, caret: number) => {
    const active = activeWikiQuery(value, caret);
    if (!active) {
      setSuggestions([]);
      setWikiRange(null);
      return;
    }
    const found = wikiSuggestions(active.query, context, selectedId ?? undefined);
    setSuggestions(found);
    setWikiRange({ start: active.start, end: active.end });
  };

  const insertWiki = (target: string, label: string) => {
    if (!wikiRange) {
      return;
    }
    const inserted = `[[${target}|${label}]]`;
    const next =
      draft.slice(0, wikiRange.start) + inserted + draft.slice(wikiRange.end);
    setDraft(next);
    setSuggestions([]);
    setWikiRange(null);
    const caret = wikiRange.start + inserted.length;
    requestAnimationFrame(() => {
      textareaRef.current?.focus();
      textareaRef.current?.setSelectionRange(caret, caret);
    });
  };

  const anchorLabel = (note: NoteView): string => {
    if (note.anchor.kind === "standalone") {
      return t("notes-anchor-standalone");
    }
    if (note.anchor.kind === "kc") {
      return t("notes-anchor-kc");
    }
    if (note.anchor.kind === "lexeme") {
      return t("notes-anchor-lexeme");
    }
    if (note.anchor.kind === "book_mark") {
      return t("notes-anchor-book");
    }
    return note.anchor.kind;
  };

  const anchorValue = (note: NoteView): string | null => {
    if (!note.anchor.id) {
      return null;
    }
    if (note.anchor.kind === "kc") {
      return content.kcTitle(note.anchor.id, locale);
    }
    if (note.anchor.kind === "lexeme") {
      return content.sense(note.anchor.id)?.sense.definition_en ?? note.anchor.id;
    }
    if (note.anchor.kind === "book_mark") {
      const mark = records.bookMarks[note.anchor.id];
      return mark
        ? sentenceIndex.get(mark.sentence_id)?.bookTitle ?? mark.book_id
        : note.anchor.id;
    }
    return note.anchor.id;
  };

  const graphPositions = useMemo(() => {
    const positions = new Map<string, { x: number; y: number }>();
    const center = graph.nodes[0];
    if (!center) {
      return positions;
    }
    positions.set(center.id, { x: 50, y: 50 });
    const others = graph.nodes.slice(1);
    others.forEach((node, index) => {
      const angle = (index / Math.max(1, others.length)) * Math.PI * 2 - Math.PI / 2;
      positions.set(node.id, {
        x: 50 + Math.cos(angle) * 36,
        y: 50 + Math.sin(angle) * 34,
      });
    });
    return positions;
  }, [graph.nodes]);

  const highlightDetail = (mark?: BookMarkView): string => {
    if (!mark) {
      return t("notes-detail-missing");
    }
    const sentence = sentenceIndex.get(mark.sentence_id);
    return sentence
      ? `“${sentence.text}” — ${sentence.bookTitle}`
      : mark.sentence_id;
  };

  const detailContent = detail ? (
    <div className="notes-detail">
      <header>
        <span className={`notes-kind notes-kind-${detail.kind}`}>{detail.kind}</span>
        <button
          type="button"
          className="notes-detail-close"
          onClick={() => setDetail(null)}
          aria-label={t("notes-detail-close")}
        >
          ×
        </button>
      </header>
      <strong>
        {detail.kind === "kc"
          ? content.kcTitle(detail.id, locale)
          : detail.kind === "lexeme"
            ? content.sense(detail.id)?.lexeme.lemma ?? detail.label
            : detail.kind === "highlight"
              ? highlightDetail(records.bookMarks[detail.id])
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
        <Button
          type="button"
          variant="secondary"
          size="sm"
          onClick={() => onPractice(detail.id)}
        >
          {t("graph-practice")}
        </Button>
      ) : null}
      {detail.kind === "book" && onOpenBook ? (
        <Button
          type="button"
          variant="secondary"
          size="sm"
          onClick={() => onOpenBook(detail.id)}
        >
          {t("library-open-book")}
        </Button>
      ) : null}
      {detail.kind === "highlight" && records.bookMarks[detail.id]?.book_id && onOpenBook ? (
        <Button
          type="button"
          variant="secondary"
          size="sm"
          onClick={() =>
            onOpenBook(records.bookMarks[detail.id]?.book_id ?? "")
          }
        >
          {t("library-open-book")}
        </Button>
      ) : null}
    </div>
  ) : null;

  const inspector = (
    <aside className={`notes-inspector${inspectorOpen ? " open" : ""}`}>
      <header className="notes-inspector-head">
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
          className="notes-inspector-close"
          aria-label={t("notes-close")}
          onClick={() => setInspectorOpen(false)}
        >
          ×
        </button>
      </header>
      {detail ? detailContent : null}
      {inspectorTab === "outline" ? (
        outline.length === 0 ? (
          <p className="bento-empty">{t("notes-outline-empty")}</p>
        ) : (
          <nav className="notes-outline" aria-label={t("notes-tab-outline")}>
            {outline.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`notes-outline-item notes-outline-${item.level}`}
                onClick={() => jumpToHeading(item.id)}
              >
                {item.text}
              </button>
            ))}
          </nav>
        )
      ) : null}
      {inspectorTab === "backlinks" ? (
        <div className="notes-backlinks">
          {backlinks.outgoing.length === 0 &&
          backlinks.incoming.length === 0 &&
          backlinks.shared.length === 0 &&
          backlinks.highlights.length === 0 ? (
            <p className="bento-empty">{t("notes-backlinks-empty")}</p>
          ) : null}
          {backlinks.incoming.length > 0 ? (
            <section>
              <h4>{t("notes-backlinks-incoming")}</h4>
              <ul>
                {backlinks.incoming.map((note) => (
                  <li key={note.note_id}>
                    <button type="button" onClick={() => selectNote(note)}>
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
                        {target.kind}
                      </span>
                      {target.label}
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
                    <button type="button" onClick={() => selectNote(note)}>
                      {noteTitle(note)}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {backlinks.highlights.length > 0 ? (
            <section>
              <h4>{t("notes-backlinks-highlights")}</h4>
              <ul>
                {backlinks.highlights.map((mark) => (
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
                      {highlightDetail(mark)}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      ) : null}
      {inspectorTab === "graph" ? (
        <div className="notes-local-graph-wrap">
          {graph.nodes.length <= 1 ? (
            <p className="bento-empty">{t("notes-graph-empty")}</p>
          ) : (
            <div className="notes-local-graph">
              <svg viewBox="0 0 100 100" aria-hidden="true">
                {graph.edges.map((edge) => {
                  const from = graphPositions.get(edge.from);
                  const to = graphPositions.get(edge.to);
                  return from && to ? (
                    <line
                      key={`${edge.from}-${edge.to}`}
                      x1={from.x}
                      y1={from.y}
                      x2={to.x}
                      y2={to.y}
                    />
                  ) : null;
                })}
              </svg>
              {graph.nodes.map((node: LocalGraphNode) => {
                const position = graphPositions.get(node.id);
                if (!position) {
                  return null;
                }
                const center = node.id === `note:${selectedNote?.note_id}`;
                const rawNodeId = node.id.slice(node.id.indexOf(":") + 1);
                const nodeLabel =
                  node.kind === "highlight"
                    ? t("notes-highlight-label")
                    : node.kind === "kc"
                      ? content.kcTitle(rawNodeId, locale)
                      : node.kind === "lexeme"
                        ? content.sense(rawNodeId)?.lexeme.lemma ?? node.label
                        : node.label;
                return (
                  <button
                    key={node.id}
                    type="button"
                    className={`notes-graph-node notes-kind-${node.kind}${center ? " center" : ""}`}
                    style={{ left: `${position.x}%`, top: `${position.y}%` }}
                    onClick={() => {
                      if (node.id.startsWith("note:")) {
                        const note = notes.find(
                          (entry) => entry.note_id === node.id.slice(5),
                        );
                        if (note) {
                          selectNote(note);
                        }
                      } else {
                        const id = rawNodeId;
                        if (node.kind === "kc" && onPractice) {
                          onPractice(id);
                        } else if (node.kind === "book" && onOpenBook) {
                          onOpenBook(id);
                        } else {
                          navigateTarget({
                            kind: node.kind,
                            id,
                            label: node.label,
                            target: node.id,
                          });
                        }
                      }
                    }}
                    title={nodeLabel}
                  >
                    <StudioIcon name={GRAPH_ICON[node.kind]} size={14} />
                    <span>{nodeLabel}</span>
                  </button>
                );
              })}
            </div>
          )}
          <p className="bento-note">{t("notes-graph-note")}</p>
        </div>
      ) : null}
    </aside>
  );

  return (
    <section className={`screen notes-workspace${mode === "edit" ? " editing" : ""}`}>
      <header className="notes-workspace-head">
        {onBack ? (
          <Button type="button" variant="ghost" size="sm" onClick={onBack}>
            {t("notes-back")}
          </Button>
        ) : null}
        <div>
          <p className="eyebrow">{t("notes-eyebrow")}</p>
          <h1>{t("notes-workspace-title")}</h1>
        </div>
        <div className="notes-workspace-head-actions">
          <Button type="button" variant="secondary" size="sm" onClick={startNew}>
            {t("notes-new")}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            aria-expanded={inspectorOpen}
            onClick={() => setInspectorOpen((value) => !value)}
          >
            <StudioIcon name="panel-right" size={16} />
            {t("notes-inspector")}
          </Button>
        </div>
      </header>
      <div className="notes-workspace-grid">
        <aside className="notes-list-pane">
          <label className="field">
            <span>{t("notes-search")}</span>
            <input
              className="answer-input"
              type="search"
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
          </label>
          <ul className="notes-workspace-list">
            {filteredNotes.map((note) => (
              <li key={note.note_id}>
                <button
                  type="button"
                  className={`notes-workspace-row${selectedId === note.note_id ? " active" : ""}`}
                  onClick={() => selectNote(note)}
                >
                  <span className={`notes-kind notes-kind-${note.anchor.kind}`}>
                    {anchorLabel(note)}
                  </span>
                  <strong>{noteTitle(note)}</strong>
                  <span className="notes-row-date">
                    {new Date(note.updated_at).toLocaleDateString(locale)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          {notes.length === 0 ? (
            <p className="bento-empty">{t("notes-workspace-empty")}</p>
          ) : null}
        </aside>
        <main className="notes-editor-pane">
          <div className="notes-editor-title">
            <div>
              <p className="eyebrow">{t("notes-editor-eyebrow")}</p>
              <h2>{title}</h2>
            </div>
            <div className="notes-mode-toggle" role="group" aria-label={t("notes-view-mode")}>
              <button
                type="button"
                className={mode === "read" ? "active" : ""}
                aria-pressed={mode === "read"}
                onClick={() => {
                  setMode("read");
                  setDraft(activeBody);
                }}
              >
                {t("notes-view-read")}
              </button>
              <button
                type="button"
                className={mode === "edit" ? "active" : ""}
                aria-pressed={mode === "edit"}
                onClick={startEdit}
              >
                {t("notes-view-edit")}
              </button>
            </div>
          </div>
          <div className="notes-anchor-pills">
            {selectedNote ? (
              <>
                <span className={`notes-kind notes-kind-${selectedNote.anchor.kind}`}>
                  {anchorLabel(selectedNote)}
                </span>
                {anchorValue(selectedNote) ? (
                  <span className="notes-anchor-value">{anchorValue(selectedNote)}</span>
                ) : null}
              </>
            ) : (
              <span className="notes-kind notes-kind-standalone">
                {t("notes-anchor-standalone")}
              </span>
            )}
          </div>
          {mode === "read" ? (
            selectedNote ? (
              <>
                {linkedHighlight && highlightText ? (
                  <details className="note-linked-highlight" open>
                    <summary>{t("notes-linked-highlight")}</summary>
                    <blockquote>
                      “{highlightText.text}” <cite>— {highlightText.bookTitle}</cite>
                    </blockquote>
                  </details>
                ) : null}
                <NoteMarkdown
                  body={selectedNote.body}
                  onWikiLink={handleWikiLink}
                  className="notes-reader"
                  omitFirstHeading
                />
                {selectedNote.body.trim() ? null : (
                  <p className="bento-empty">{t("notes-empty-body")}</p>
                )}
                {selectedNote.anchor.kind === "book_mark" &&
                selectedNote.anchor.id &&
                records.bookMarks[selectedNote.anchor.id] ? (
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      const mark = records.bookMarks[selectedNote.anchor.id ?? ""];
                      if (mark) {
                        navigateTarget({
                          kind: "highlight",
                          id: mark.mark_id,
                          label: t("notes-highlight-label"),
                          target: `highlight:${mark.mark_id}`,
                        });
                      }
                    }}
                  >
                    {t("notes-open-highlight")}
                  </Button>
                ) : null}
              </>
            ) : (
              <p className="bento-empty">{t("notes-select-hint")}</p>
            )
          ) : (
            <>
              <textarea
                ref={textareaRef}
                className="answer-input notes-textarea"
                value={draft}
                rows={18}
                placeholder={t("notes-placeholder")}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  setDraft(value);
                  updateWikiQuery(value, event.currentTarget.selectionStart);
                }}
                onKeyUp={(event) =>
                  updateWikiQuery(draft, event.currentTarget.selectionStart)
                }
                onClick={(event) =>
                  updateWikiQuery(draft, event.currentTarget.selectionStart)
                }
              />
              {suggestions.length > 0 ? (
                <div className="notes-wiki-suggestions" role="listbox" aria-label={t("notes-wiki-suggestions")}>
                  {suggestions.map((suggestion) => (
                    <button
                      key={suggestion.target}
                      type="button"
                      role="option"
                      aria-selected="false"
                      onClick={() => insertWiki(suggestion.target, suggestion.label)}
                    >
                      <span className={`notes-kind notes-kind-${suggestion.kind}`}>
                        {suggestion.kind}
                      </span>
                      {suggestion.label}
                    </button>
                  ))}
                </div>
              ) : null}
              <div className="notes-editor-meta">
                <span>{t("notes-word-count", { count: noteWordCount(draft) })}</span>
                <span>{t("notes-char-count", { count: draft.length })}</span>
                <span className={bytes > 4096 ? "over" : ""}>
                  {t("notes-count", { used: bytes, limit: 4096 })}
                </span>
              </div>
              {error ? <p className="inline-alert danger">{error}</p> : null}
              <div className="notes-editor-actions">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setMode("read");
                    setDraft(activeBody);
                    setSuggestions([]);
                  }}
                >
                  {t("notes-cancel")}
                </Button>
                {selectedNote ? (
                  <Button
                    type="button"
                    variant="danger"
                    size="sm"
                    disabled={busy}
                    onClick={() => void remove()}
                  >
                    {t("notes-delete")}
                  </Button>
                ) : null}
                <Button
                  type="button"
                  variant="primary"
                  size="sm"
                  busy={busy}
                  disabled={!draft.trim() || bytes > 4096}
                  onClick={() => void save()}
                >
                  {t("notes-save")}
                </Button>
              </div>
            </>
          )}
        </main>
        {inspector}
      </div>
      {inspectorOpen ? (
        <button
          type="button"
          className="notes-inspector-scrim"
          aria-label={t("notes-close")}
          onClick={() => setInspectorOpen(false)}
        />
      ) : null}
    </section>
  );
}
