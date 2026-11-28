import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";

import { NotesPanel } from "../components/NotesPanel";
import { StudioIcon } from "../components/StudioIcons";
import { Button } from "../components/ui";
import type { BookParagraph } from "../generated/core/BookParagraph";
import type { BookSentence } from "../generated/core/BookSentence";
import type { BookToken } from "../generated/core/BookToken";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import { noteTitle } from "../prototype/notes";
import {
  activeMarks,
  activeNotes,
  type BookMarkView,
  type NoteView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import {
  READER_LINE_HEIGHTS,
  READER_MEASURES,
  READER_SIZES,
  READER_THEMES,
  readReaderSettings,
  readerStyle,
  writeReaderSettings,
  type ReaderSettings,
} from "../prototype/readerSettings";
import {
  selectionSpans,
  sentenceParts,
  type SelectionSpan,
} from "../prototype/readerText";

export interface BookReaderProps {
  book: LoadedBook;
  content: PrototypeContent;
  stateJson: string;
  records: PersonalRecords;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
  locale: Locale;
  onBack: () => void;
  onOpenNotes?: (noteId?: string) => void;
}

interface SelectionBar {
  spans: SelectionSpan[];
  text: string;
  top: number;
  left: number;
}

interface WordCard {
  text: string;
  lexemeId: string | null;
  sentence: BookSentence;
  chapterId: string;
  paragraphId: string;
  top: number;
  left: number;
}

interface LocatedSentence {
  sentence: BookSentence;
  paragraph: BookParagraph;
}

interface BookSentenceLocation {
  sentence: BookSentence;
  chapterIndex: number;
  chapterTitle: string;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function markKindKey(mark: BookMarkView): string {
  return mark.kind === "highlight" ? "reader-highlight" : "reader-bookmark";
}

export function BookReader({
  book,
  content,
  stateJson,
  records,
  recordsWriter,
  onStateChange,
  locale,
  onBack,
  onOpenNotes,
}: BookReaderProps) {
  const { t } = useTranslation();
  const [chapterIndex, setChapterIndex] = useState(0);
  const [word, setWord] = useState<WordCard | null>(null);
  const [selectionBar, setSelectionBar] = useState<SelectionBar | null>(null);
  const [noteMark, setNoteMark] = useState<BookMarkView | null>(null);
  const [noteDraft, setNoteDraft] = useState("");
  const [customDefinition, setCustomDefinition] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settings, setSettings] = useState<ReaderSettings>(readReaderSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [annotationsOpen, setAnnotationsOpen] = useState(false);
  const [progress, setProgress] = useState(0);
  const [jumpSentence, setJumpSentence] = useState<string | null>(null);
  const articleRef = useRef<HTMLElement | null>(null);
  const scrollRef = useRef<HTMLElement | null>(null);
  const chapter = book.book.chapters[chapterIndex];
  const marks = activeMarks(records, book.meta.id);
  const annotationMarks = useMemo(() => {
    const chapterOrder = new Map(
      book.book.chapters.map((entry, index) => [entry.id, index]),
    );
    return [...marks].sort((left, right) => {
      const chapter =
        (chapterOrder.get(left.chapter_id) ?? 0) -
        (chapterOrder.get(right.chapter_id) ?? 0);
      return (
        chapter ||
        left.start_token - right.start_token ||
        left.mark_id.localeCompare(right.mark_id)
      );
    });
  }, [book.book.chapters, marks]);
  const notes = useMemo(() => activeNotes(records), [records]);
  const notesByMark = useMemo(() => {
    const map = new Map<string, NoteView[]>();
    for (const note of notes) {
      if (note.anchor.kind !== "book_mark" || !note.anchor.id) {
        continue;
      }
      const list = map.get(note.anchor.id);
      if (list) {
        list.push(note);
      } else {
        map.set(note.anchor.id, [note]);
      }
    }
    return map;
  }, [notes]);
  const allSentences = useMemo(() => {
    const map = new Map<string, BookSentenceLocation>();
    book.book.chapters.forEach((entry, index) => {
      const chapterTitle = localizedText(entry.title, locale);
      for (const paragraph of entry.paragraphs) {
        for (const sentence of paragraph.sentences) {
          map.set(sentence.id, {
            sentence,
            chapterIndex: index,
            chapterTitle,
          });
        }
      }
    });
    return map;
  }, [book.book.chapters, locale]);
  const currentSense = word
    ? word.lexemeId
      ? content.sense(word.lexemeId)
      : content.senseForForm(word.text)
    : null;
  const custom =
    currentSense &&
    Object.values(records.customLexemes).find(
      (record) =>
        !record.archived &&
        (record.bundle_lexeme_id === currentSense.sense.id ||
          record.custom_id === currentSense.sense.id),
    );
  const readerVars = readerStyle(settings) as unknown as CSSProperties;

  const located = useMemo(() => {
    const map = new Map<string, LocatedSentence>();
    for (const paragraph of chapter?.paragraphs ?? []) {
      for (const sentence of paragraph.sentences) {
        map.set(sentence.id, { sentence, paragraph });
      }
    }
    return map;
  }, [chapter]);

  useEffect(() => {
    document.body.classList.add("reader-focus");
    return () => {
      document.body.classList.remove("reader-focus");
    };
  }, []);

  useEffect(() => {
    writeReaderSettings(settings);
  }, [settings]);

  useEffect(() => {
    const scroll = scrollRef.current;
    if (!scroll) {
      return;
    }
    const update = () => {
      const max = scroll.scrollHeight - scroll.clientHeight;
      setProgress(
        max <= 0 ? 100 : Math.round(clamp(scroll.scrollTop / max, 0, 1) * 100),
      );
    };
    update();
    scroll.addEventListener("scroll", update, { passive: true });
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(update);
    observer?.observe(scroll);
    return () => {
      scroll.removeEventListener("scroll", update);
      observer?.disconnect();
    };
  }, [chapterIndex, settings]);

  useEffect(() => {
    const scroll = scrollRef.current;
    if (scroll) {
      scroll.scrollTop = 0;
    }
    setProgress(0);
    setWord(null);
    setSelectionBar(null);
    setNoteMark(null);
    window.getSelection()?.removeAllRanges();
  }, [chapterIndex]);

  useEffect(() => {
    if (!jumpSentence) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      const node = scrollRef.current?.querySelector<HTMLElement>(
        `[data-sentence-id="${jumpSentence}"]`,
      );
      if (typeof node?.scrollIntoView === "function") {
        node.scrollIntoView({ behavior: "smooth", block: "center" });
      }
      setJumpSentence(null);
    });
    return () => cancelAnimationFrame(frame);
  }, [jumpSentence, chapterIndex]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      setSettingsOpen(false);
      setAnnotationsOpen(false);
      setWord(null);
      setSelectionBar(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const tapToken = (
    event: React.MouseEvent<HTMLElement> | React.KeyboardEvent<HTMLElement>,
    token: BookToken,
    chapterId: string,
    paragraphId: string,
    sentence: BookSentence,
  ) => {
    const selection = window.getSelection();
    if (selection && !selection.isCollapsed) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    setCustomDefinition("");
    setSelectionBar(null);
    setSettingsOpen(false);
    setWord({
      text: token.text,
      lexemeId:
        token.lexeme_id ??
        (token.sense_ids.length === 1 ? token.sense_ids[0] ?? null : null),
      sentence,
      chapterId,
      paragraphId,
      left: clamp(rect.left - 18, 8, Math.max(8, window.innerWidth - 336)),
      top: clamp(rect.bottom + 8, 8, Math.max(8, window.innerHeight - 380)),
    });
  };

  const clearSelection = () => {
    window.getSelection()?.removeAllRanges();
    setSelectionBar(null);
  };

  const captureSelection = () => {
    const selection = window.getSelection();
    const root = articleRef.current;
    if (!selection || !root || selection.isCollapsed) {
      setSelectionBar(null);
      return;
    }
    const spans = selectionSpans(selection, root);
    if (spans.length === 0) {
      setSelectionBar(null);
      return;
    }
    setWord(null);
    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    setSelectionBar({
      spans,
      text: selection.toString().replace(/\s+/g, " ").trim(),
      left: clamp(rect.left, 8, Math.max(8, window.innerWidth - 320)),
      top: clamp(rect.top - 54, 8, Math.max(8, window.innerHeight - 60)),
    });
  };

  const addToVocabulary = async () => {
    if (!word) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const sense = currentSense?.sense ?? null;
      const definition = sense?.definition_en ?? customDefinition;
      if (!definition.trim()) {
        setError(t("reader-definition-required"));
        return;
      }
      const next = await recordsWriter.saveCustomWord(stateJson, {
        lemma: word.text,
        definitionEn: definition,
        glossUz: sense?.gloss_uz ?? "",
        glossRu: sense?.gloss_ru ?? "",
        examples: [word.sentence.text],
        collocations: [],
        ...(sense
          ? {
              bundleLexemeId: sense.id,
              ipa: sense.ipa_us || sense.ipa_uk,
              audioKey: sense.audio_key,
            }
          : {}),
        context: {
          text: word.sentence.text,
          bookId: book.meta.id,
          sentenceId: word.sentence.id,
        },
        source: "book",
      });
      onStateChange(next);
      setWord(null);
    } catch {
      setError(t("vocab-error-save"));
    } finally {
      setBusy(false);
    }
  };

  const writeMarks = async (
    spans: SelectionSpan[],
    kind: "highlight" | "bookmark",
  ): Promise<BookMarkView[]> => {
    setBusy(true);
    setError(null);
    let currentState = stateJson;
    const created: BookMarkView[] = [];
    try {
      for (const span of spans) {
        const target = located.get(span.sentenceId);
        if (!target) {
          continue;
        }
        const result = await recordsWriter.createBookMark(currentState, {
          bookId: book.meta.id,
          bookVersion: book.meta.version,
          chapterId: chapter?.id ?? "",
          paragraphId: target.paragraph.id,
          sentenceId: target.sentence.id,
          sentenceText: target.sentence.text,
          startToken: span.start,
          endToken: span.end,
          kind,
        });
        currentState = result.stateJson;
        created.push({
          mark_id: result.markId,
          book_id: book.meta.id,
          book_version: book.meta.version,
          kind,
          chapter_id: chapter?.id ?? "",
          paragraph_id: target.paragraph.id,
          sentence_id: target.sentence.id,
          start_token: span.start,
          end_token: span.end,
          quote_hash: "",
          deleted: false,
          event_id: result.markId,
          updated_at: Date.now(),
        });
      }
      if (created.length > 0) {
        onStateChange(currentState);
      }
      return created;
    } catch {
      if (created.length > 0) {
        onStateChange(currentState);
      }
      setError(t("reader-mark-error"));
      return created;
    } finally {
      setBusy(false);
    }
  };

  const selectionAction = async (
    action: "highlight" | "bookmark" | "note",
  ) => {
    if (!selectionBar) {
      return;
    }
    const created = await writeMarks(
      selectionBar.spans,
      action === "bookmark" ? "bookmark" : "highlight",
    );
    if (created.length === 0) {
      return;
    }
    if (action === "note") {
      const quote = selectionBar.text.slice(0, 380);
      setNoteDraft(quote ? `> “${quote}”\n\n` : "");
      setNoteMark(created[0] ?? null);
    }
    clearSelection();
  };

  const removeMark = async (mark: BookMarkView) => {
    setBusy(true);
    setError(null);
    try {
      onStateChange(await recordsWriter.deleteBookMark(stateJson, mark.mark_id));
    } catch {
      setError(t("reader-mark-error"));
    } finally {
      setBusy(false);
    }
  };

  const jumpToMark = (mark: BookMarkView) => {
    const index = book.book.chapters.findIndex(
      (entry) => entry.id === mark.chapter_id,
    );
    setAnnotationsOpen(false);
    setSettingsOpen(false);
    setChapterIndex(index >= 0 ? index : 0);
    setJumpSentence(mark.sentence_id);
  };

  const updateSetting = <Key extends keyof ReaderSettings>(
    key: Key,
    value: ReaderSettings[Key],
  ) => {
    setSettings((current) => ({ ...current, [key]: value }));
  };

  const openNote = (mark: BookMarkView) => {
    setAnnotationsOpen(false);
    setNoteDraft("");
    setNoteMark(mark);
  };

  const kindLabel = (mark: BookMarkView) => t(markKindKey(mark));

  return (
    <div
      className="reader-full"
      data-reader-theme={settings.theme}
      style={readerVars}
    >
      <header className="reader-topbar">
        <button
          type="button"
          className="reader-back"
          aria-label={t("reader-back-library")}
          onClick={onBack}
        >
          <StudioIcon name="arrow-left" size={17} />
          <span>{t("reader-back-library")}</span>
        </button>
        <div className="reader-topbar-title">
          <span className="reader-topbar-level">{book.meta.level}</span>
          <h1 id="reader-title">{localizedText(book.meta.title, locale)}</h1>
        </div>
        <div className="reader-topbar-actions">
          <label className="reader-chapter-select">
            <span className="sr-only">{t("reader-chapter-selector")}</span>
            <select
              value={chapterIndex}
              aria-label={t("reader-chapter-selector")}
              onChange={(event) => setChapterIndex(Number(event.currentTarget.value))}
            >
              {book.book.chapters.map((entry, index) => (
                <option key={entry.id} value={index}>
                  {localizedText(entry.title, locale)}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className="reader-icon-button"
            aria-label={t("reader-settings")}
            title={t("reader-settings")}
            aria-expanded={settingsOpen}
            onClick={() => {
              setSettingsOpen((open) => !open);
              setAnnotationsOpen(false);
            }}
          >
            <StudioIcon name="settings" size={18} />
          </button>
          <button
            type="button"
            className="reader-icon-button"
            aria-label={t("reader-annotations")}
            title={t("reader-annotations")}
            aria-expanded={annotationsOpen}
            onClick={() => {
              setAnnotationsOpen((open) => !open);
              setSettingsOpen(false);
            }}
          >
            <StudioIcon name="bookmark" size={18} />
            {marks.length > 0 ? (
              <span className="reader-count">{marks.length}</span>
            ) : null}
          </button>
        </div>
      </header>

      <main className="reader-scroll" ref={scrollRef}>
        <div className="reader-reading">
          <div className="reader-chapter-head">
            <span className="reader-chapter-index">
              {chapterIndex + 1} / {book.book.chapters.length}
            </span>
            <h2>{chapter ? localizedText(chapter.title, locale) : ""}</h2>
            <p className="reader-hint">{t("reader-selection-hint")}</p>
          </div>
          {error ? (
            <p className="reader-error" role="alert">
              {error}
            </p>
          ) : null}
          {chapter ? (
            <article
              className="reader-paper"
              ref={articleRef}
              onMouseUp={captureSelection}
              onTouchEnd={captureSelection}
              onKeyUp={captureSelection}
            >
              {chapter.paragraphs.map((paragraph) => {
                const sentences = paragraph.sentences;
                return (
                  <p key={paragraph.id} className="reader-paragraph">
                    {sentences.map((sentence, sentenceIndex) => {
                      const sentenceMarks = marks.filter(
                        (mark) => mark.sentence_id === sentence.id,
                      );
                      return (
                        <span
                          key={sentence.id}
                          className="reader-sentence"
                          data-sentence-id={sentence.id}
                        >
                          {sentenceParts(sentence).map((part, partIndex) => {
                            if (part.kind === "text") {
                              return (
                                <span
                                  key={`text-${partIndex}`}
                                  className="reader-punct"
                                >
                                  {part.text}
                                </span>
                              );
                            }
                            const token = sentence.tokens[part.tokenIndex];
                            if (!token) {
                              return part.text;
                            }
                            const highlighted = sentenceMarks.some(
                              (mark) =>
                                mark.kind === "highlight" &&
                                part.tokenIndex >= mark.start_token &&
                                part.tokenIndex <= mark.end_token,
                            );
                            return (
                              <span
                                key={token.id}
                                role="button"
                                tabIndex={0}
                                className={`reader-token${highlighted ? " highlighted" : ""}`}
                                data-token-index={part.tokenIndex}
                                data-sentence-id={sentence.id}
                                onClick={(event) =>
                                  tapToken(
                                    event,
                                    token,
                                    chapter.id,
                                    paragraph.id,
                                    sentence,
                                  )
                                }
                                onKeyDown={(event) => {
                                  if (
                                    event.key === "Enter" ||
                                    event.key === " "
                                  ) {
                                    event.preventDefault();
                                    tapToken(
                                      event,
                                      token,
                                      chapter.id,
                                      paragraph.id,
                                      sentence,
                                    );
                                  }
                                }}
                              >
                                {part.text}
                              </span>
                            );
                          })}
                          {sentenceIndex < sentences.length - 1 ? " " : null}
                        </span>
                      );
                    })}
                  </p>
                );
              })}
            </article>
          ) : null}
        </div>
      </main>

      <footer className="reader-footer">
        <div className="reader-progress-line" aria-hidden="true">
          <span style={{ width: `${progress}%` }} />
        </div>
        <span>
          {t("reader-progress", {
            chapter: chapterIndex + 1,
            total: book.book.chapters.length,
            percent: progress,
          })}
        </span>
      </footer>

      {settingsOpen ? (
        <>
          <button
            type="button"
            className="reader-scrim"
            aria-hidden="true"
            tabIndex={-1}
            onClick={() => setSettingsOpen(false)}
          />
          <section
            className="reader-settings-popover"
            role="dialog"
            aria-label={t("reader-settings-title")}
          >
            <header>
              <h2>{t("reader-settings-title")}</h2>
              <button
                type="button"
                className="reader-icon-button"
                aria-label={t("reader-settings-close")}
                onClick={() => setSettingsOpen(false)}
              >
                <StudioIcon name="close" size={16} />
              </button>
            </header>
            <fieldset>
              <legend>{t("reader-theme")}</legend>
              <div className="reader-theme-grid">
                {READER_THEMES.map((theme) => (
                  <button
                    key={theme}
                    type="button"
                    className={`reader-theme-choice${settings.theme === theme ? " active" : ""}`}
                    data-swatch={theme}
                    aria-pressed={settings.theme === theme}
                    onClick={() => updateSetting("theme", theme)}
                  >
                    <span className="reader-theme-swatch" data-swatch={theme} />
                    {t(`reader-theme-${theme}`)}
                  </button>
                ))}
              </div>
            </fieldset>
            <fieldset>
              <legend>{t("reader-size")}</legend>
              <div className="reader-segment">
                {READER_SIZES.map((size) => (
                  <button
                    key={size}
                    type="button"
                    className={settings.size === size ? "active" : ""}
                    aria-pressed={settings.size === size}
                    onClick={() => updateSetting("size", size)}
                  >
                    {t(`reader-size-${size}`)}
                  </button>
                ))}
              </div>
            </fieldset>
            <fieldset>
              <legend>{t("reader-line-height")}</legend>
              <div className="reader-segment">
                {READER_LINE_HEIGHTS.map((height) => (
                  <button
                    key={height}
                    type="button"
                    className={settings.lineHeight === height ? "active" : ""}
                    aria-pressed={settings.lineHeight === height}
                    onClick={() => updateSetting("lineHeight", height)}
                  >
                    {height}
                  </button>
                ))}
              </div>
            </fieldset>
            <fieldset>
              <legend>{t("reader-measure")}</legend>
              <div className="reader-segment">
                {READER_MEASURES.map((measure) => (
                  <button
                    key={measure}
                    type="button"
                    className={settings.measure === measure ? "active" : ""}
                    aria-pressed={settings.measure === measure}
                    onClick={() => updateSetting("measure", measure)}
                  >
                    {measure}ch
                  </button>
                ))}
              </div>
            </fieldset>
          </section>
        </>
      ) : null}

      {annotationsOpen ? (
        <>
          <button
            type="button"
            className="reader-scrim"
            aria-hidden="true"
            tabIndex={-1}
            onClick={() => setAnnotationsOpen(false)}
          />
          <aside
            className="reader-annotations"
            role="dialog"
            aria-label={t("reader-annotations")}
          >
            <header>
              <div>
                <span className="reader-topbar-level">
                  {t("reader-annotation-count", { count: annotationMarks.length })}
                </span>
                <h2>{t("reader-annotations-title")}</h2>
              </div>
              <button
                type="button"
                className="reader-icon-button"
                aria-label={t("reader-annotations-close")}
                onClick={() => setAnnotationsOpen(false)}
              >
                <StudioIcon name="close" size={16} />
              </button>
            </header>
            {annotationMarks.length === 0 ? (
              <p className="reader-annotations-empty">
                {t("reader-annotations-empty")}
              </p>
            ) : (
              <ul>
                {annotationMarks.map((mark) => {
                  const location = allSentences.get(mark.sentence_id);
                  const markNotes = notesByMark.get(mark.mark_id) ?? [];
                  return (
                    <li key={mark.mark_id}>
                      <button
                        type="button"
                        className="reader-annotation-main"
                        onClick={() => jumpToMark(mark)}
                      >
                        <span className="reader-annotation-meta">
                          <span>{kindLabel(mark)}</span>
                          <span>{location?.chapterTitle ?? ""}</span>
                        </span>
                        <span className="reader-annotation-quote">
                          “{location?.sentence.text ?? ""}”
                        </span>
                        <span className="reader-annotation-jump">
                          {t("reader-annotation-jump")}
                        </span>
                      </button>
                      {markNotes.map((note) => (
                        <button
                          key={note.note_id}
                          type="button"
                          className="reader-annotation-note"
                          onClick={() => openNote(mark)}
                        >
                          <span>{t("reader-annotation-note")}</span>
                          <strong>{noteTitle(note)}</strong>
                        </button>
                      ))}
                      <button
                        type="button"
                        className="reader-annotation-remove"
                        disabled={busy}
                        onClick={() => void removeMark(mark)}
                      >
                        {t("notes-delete")}
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </aside>
        </>
      ) : null}

      {selectionBar ? (
        <div
          className="reader-selection-bar"
          role="toolbar"
          aria-label={t("reader-selection-bar")}
          style={{ top: `${selectionBar.top}px`, left: `${selectionBar.left}px` }}
          onMouseDown={(event) => event.preventDefault()}
        >
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={busy}
            onClick={() => void selectionAction("highlight")}
          >
            {t("reader-highlight")}
          </Button>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={busy}
            onClick={() => void selectionAction("bookmark")}
          >
            {t("reader-bookmark")}
          </Button>
          <Button
            type="button"
            variant="primary"
            size="sm"
            disabled={busy}
            onClick={() => void selectionAction("note")}
          >
            {t("reader-add-note")}
          </Button>
        </div>
      ) : null}

      {word ? (
        <aside
          className="word-card word-popover"
          role="dialog"
          aria-label={t("reader-word-card")}
          style={{ top: `${word.top}px`, left: `${word.left}px` }}
        >
          <header>
            <h3>{word.text}</h3>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setWord(null)}
            >
              {t("notes-close")}
            </Button>
          </header>
          {currentSense ? (
            <>
              {currentSense.sense.ipa_us || currentSense.sense.ipa_uk ? (
                <p className="vocab-ipa">
                  /{currentSense.sense.ipa_us || currentSense.sense.ipa_uk}/
                </p>
              ) : null}
              <p>{currentSense.sense.definition_en}</p>
              {locale === "uz" && currentSense.sense.gloss_uz ? (
                <p className="vocab-gloss">{currentSense.sense.gloss_uz}</p>
              ) : null}
              {locale === "ru" && currentSense.sense.gloss_ru ? (
                <p className="vocab-gloss">{currentSense.sense.gloss_ru}</p>
              ) : null}
            </>
          ) : (
            <>
              <p className="bento-note">{t("reader-not-in-dictionary")}</p>
              <label className="field">
                <span>{t("vocab-form-definition")}</span>
                <textarea
                  className="answer-input"
                  rows={2}
                  value={customDefinition}
                  onChange={(event) =>
                    setCustomDefinition(event.currentTarget.value)
                  }
                />
              </label>
            </>
          )}
          <p className="bento-note">{word.sentence.text}</p>
          <Button
            type="button"
            variant="primary"
            disabled={busy || Boolean(custom)}
            onClick={() => void addToVocabulary()}
          >
            {custom ? t("reader-in-vocabulary") : t("reader-add-vocabulary")}
          </Button>
        </aside>
      ) : null}

      {noteMark ? (
        <NotesPanel
          records={records}
          anchor={{ kind: "book_mark", id: noteMark.mark_id }}
          recordsWriter={recordsWriter}
          stateJson={stateJson}
          onStateChange={onStateChange}
          title={t("reader-note-title")}
          initialDraft={noteDraft}
          onOpenWorkspace={onOpenNotes}
          onClose={() => {
            setNoteMark(null);
            setNoteDraft("");
          }}
        />
      ) : null}
    </div>
  );
}
