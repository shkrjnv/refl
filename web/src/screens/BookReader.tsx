import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";

import { NotesPanel } from "../components/NotesPanel";
import {
  ReaderWordPopover,
  type ReaderWordAnchor,
} from "../components/ReaderWordPopover";
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
  READER_LAYOUTS,
  READER_LINE_HEIGHTS,
  READER_MEASURES,
  READER_SIZES,
  READER_THEMES,
  readReaderSettings,
  readerStyle,
  writeReaderSettings,
  type ReaderLayout,
  type ReaderSettings,
} from "../prototype/readerSettings";
import { chapterSections, type ReaderSection } from "../prototype/readerToc";
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
  anchor: ReaderWordAnchor;
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

/** Column geometry for paged reading, in CSS pixels. */
interface PageGeometry {
  columnWidth: number;
  gap: number;
  /** Visible width of one page (or a two-page spread). */
  width: number;
  height: number;
  columns: 1 | 2;
}

const SWIPE_DISTANCE = 48;
const SPREAD_GUTTER = 64;
const PAGE_GUTTER = 48;

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return (
    target.isContentEditable ||
    ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
    target.closest("[role='dialog']") !== null
  );
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
  const [tocOpen, setTocOpen] = useState(false);
  const [geometry, setGeometry] = useState<PageGeometry | null>(null);
  const [page, setPage] = useState(0);
  const [pageCount, setPageCount] = useState(1);
  const [fontsVersion, setFontsVersion] = useState(0);
  const articleRef = useRef<HTMLElement | null>(null);
  const scrollRef = useRef<HTMLElement | null>(null);
  const pagedRef = useRef<HTMLElement | null>(null);
  const pagesRef = useRef<HTMLDivElement | null>(null);
  const probeRef = useRef<HTMLDivElement | null>(null);
  const pendingPage = useRef<"start" | "end" | null>(null);
  const pageRatio = useRef(0);
  const swipe = useRef<{ x: number; y: number } | null>(null);
  const paged = settings.layout === "paged";
  const chapters = book.book.chapters;
  const chapter = chapters[chapterIndex];
  const sections = useMemo(
    () => chapters.map((entry) => chapterSections(entry)),
    [chapters],
  );
  const stride = geometry ? geometry.width + geometry.gap : 0;
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
    if (!scroll || paged) {
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
  }, [chapterIndex, settings, paged]);

  // Paged geometry: one page per column (two on a wide screen), sized from
  // the free space and the chosen measure. The probe resolves `--reader-measure`
  // (in `ch` of the reading font) to pixels.
  useLayoutEffect(() => {
    const container = pagedRef.current;
    if (!paged || !container) {
      setGeometry(null);
      return;
    }
    const update = () => {
      const style = getComputedStyle(container);
      const availableWidth =
        container.clientWidth -
        parseFloat(style.paddingLeft || "0") -
        parseFloat(style.paddingRight || "0");
      const availableHeight =
        container.clientHeight -
        parseFloat(style.paddingTop || "0") -
        parseFloat(style.paddingBottom || "0");
      if (availableWidth < 1 || availableHeight < 1) {
        setGeometry(null);
        return;
      }
      const measure = probeRef.current?.getBoundingClientRect().width || availableWidth;
      const columns: 1 | 2 =
        availableWidth >= measure * 2 + SPREAD_GUTTER ? 2 : 1;
      const gap = columns === 2 ? SPREAD_GUTTER : PAGE_GUTTER;
      const columnWidth = Math.floor(
        columns === 2
          ? Math.min(measure, (availableWidth - gap) / 2)
          : Math.min(measure, availableWidth),
      );
      const next: PageGeometry = {
        columnWidth,
        gap,
        width: columns * columnWidth + (columns - 1) * gap,
        height: Math.floor(availableHeight),
        columns,
      };
      setGeometry((current) =>
        current &&
        current.columnWidth === next.columnWidth &&
        current.width === next.width &&
        current.height === next.height &&
        current.columns === next.columns
          ? current
          : next,
      );
    };
    update();
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(container);
    return () => observer?.disconnect();
  }, [paged, settings.measure, settings.size, fontsVersion]);

  // Page count follows layout: the columns overflow sideways inside the
  // fixed-height viewport, so its scroll width is the number of columns.
  useLayoutEffect(() => {
    const pages = pagesRef.current;
    if (!paged || !pages || !geometry) {
      setPageCount(1);
      return;
    }
    const columns = Math.max(
      1,
      Math.round((pages.scrollWidth + geometry.gap) / (geometry.columnWidth + geometry.gap)),
    );
    const count = Math.max(1, Math.ceil(columns / geometry.columns));
    setPageCount(count);
    const pending = pendingPage.current;
    pendingPage.current = null;
    setPage(
      pending === "end"
        ? count - 1
        : pending === "start"
          ? 0
          : Math.min(count - 1, Math.round(pageRatio.current * (count - 1))),
    );
  }, [paged, geometry, chapterIndex, settings.lineHeight, settings.size, fontsVersion]);

  useEffect(() => {
    pageRatio.current = pageCount > 1 ? page / (pageCount - 1) : 0;
    const pages = pagesRef.current;
    if (paged && pages && stride > 0) {
      pages.scrollLeft = page * stride;
    }
  }, [page, pageCount, paged, stride]);

  useEffect(() => {
    let cancelled = false;
    void document.fonts?.ready
      .then(() => {
        if (!cancelled) {
          setFontsVersion((value) => value + 1);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

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
      const root = paged ? pagesRef.current : scrollRef.current;
      const node = root?.querySelector<HTMLElement>(
        `[data-sentence-id="${jumpSentence}"]`,
      );
      if (paged) {
        if (node) {
          setPage(pageOf(node));
        }
      } else if (typeof node?.scrollIntoView === "function") {
        node.scrollIntoView({ behavior: "smooth", block: "center" });
      }
      setJumpSentence(null);
    });
    return () => cancelAnimationFrame(frame);
    // `pageCount` re-runs the jump once a new chapter has been paginated.
  }, [jumpSentence, chapterIndex, paged, pageCount]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSettingsOpen(false);
        setAnnotationsOpen(false);
        setTocOpen(false);
        setWord(null);
        setSelectionBar(null);
        return;
      }
      if (
        !paged ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        isTypingTarget(event.target)
      ) {
        return;
      }
      if (event.key === "ArrowRight" || event.key === "PageDown") {
        event.preventDefault();
        turnRef.current(1);
      } else if (event.key === "ArrowLeft" || event.key === "PageUp") {
        event.preventDefault();
        turnRef.current(-1);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [paged]);

  /** Page index that shows an element inside the paged viewport. */
  function pageOf(element: HTMLElement): number {
    const pages = pagesRef.current;
    if (!pages || stride <= 0) {
      return 0;
    }
    const left =
      element.getBoundingClientRect().left -
      pages.getBoundingClientRect().left +
      pages.scrollLeft;
    return clamp(Math.floor(left / stride), 0, pageCount - 1);
  }

  const dismissFloating = () => {
    setWord(null);
    setSelectionBar(null);
    window.getSelection()?.removeAllRanges();
  };

  const turn = (direction: 1 | -1) => {
    dismissFloating();
    if (direction === 1) {
      if (page < pageCount - 1) {
        setPage(page + 1);
      } else if (chapterIndex < chapters.length - 1) {
        pendingPage.current = "start";
        setChapterIndex(chapterIndex + 1);
      }
    } else if (page > 0) {
      setPage(page - 1);
    } else if (chapterIndex > 0) {
      pendingPage.current = "end";
      setChapterIndex(chapterIndex - 1);
    }
  };
  const turnRef = useRef(turn);
  turnRef.current = turn;

  const canTurnBack = page > 0 || chapterIndex > 0;
  const canTurnForward = page < pageCount - 1 || chapterIndex < chapters.length - 1;

  const swipeStart = (event: ReactPointerEvent<HTMLElement>) => {
    swipe.current =
      event.pointerType === "mouse" ? null : { x: event.clientX, y: event.clientY };
  };

  const swipeEnd = (event: ReactPointerEvent<HTMLElement>) => {
    const start = swipe.current;
    swipe.current = null;
    if (!start) {
      return;
    }
    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    const selection = window.getSelection();
    if (
      Math.abs(dx) < SWIPE_DISTANCE ||
      Math.abs(dx) < Math.abs(dy) * 1.5 ||
      (selection && !selection.isCollapsed)
    ) {
      return;
    }
    turn(dx < 0 ? 1 : -1);
  };

  const openChapter = (index: number, sentenceId?: string) => {
    setTocOpen(false);
    dismissFloating();
    if (index !== chapterIndex) {
      pendingPage.current = "start";
      setChapterIndex(index);
    } else if (!sentenceId) {
      setPage(0);
      scrollRef.current?.scrollTo?.({ top: 0 });
    }
    if (sentenceId) {
      setJumpSentence(sentenceId);
    }
  };

  const changeLayout = (layout: ReaderLayout) => {
    dismissFloating();
    pageRatio.current = 0;
    updateSetting("layout", layout);
  };

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
      anchor: {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
      },
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
    openChapter(index >= 0 ? index : 0, mark.sentence_id);
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

  const chapterBody = (
    <>
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
    </>
  );

  return (
    <div
      className="reader-full"
      data-reader-theme={settings.theme}
      data-reader-layout={settings.layout}
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
          <div
            className="reader-layout-toggle"
            role="group"
            aria-label={t("reader-layout")}
          >
            {READER_LAYOUTS.map((layout) => (
              <button
                key={layout}
                type="button"
                className={settings.layout === layout ? "active" : ""}
                aria-pressed={settings.layout === layout}
                onClick={() => changeLayout(layout)}
              >
                {t(`reader-layout-${layout}`)}
              </button>
            ))}
          </div>
          <button
            type="button"
            className="reader-icon-button"
            aria-label={t("reader-toc")}
            title={t("reader-toc")}
            aria-expanded={tocOpen}
            onClick={() => {
              setTocOpen((open) => !open);
              setSettingsOpen(false);
              setAnnotationsOpen(false);
            }}
          >
            <StudioIcon name="list" size={18} />
          </button>
          <button
            type="button"
            className="reader-icon-button"
            aria-label={t("reader-settings")}
            title={t("reader-settings")}
            aria-expanded={settingsOpen}
            onClick={() => {
              setSettingsOpen((open) => !open);
              setAnnotationsOpen(false);
              setTocOpen(false);
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
              setTocOpen(false);
            }}
          >
            <StudioIcon name="bookmark" size={18} />
            {marks.length > 0 ? (
              <span className="reader-count">{marks.length}</span>
            ) : null}
          </button>
        </div>
      </header>

      {paged ? (
        <main
          className="reader-paged"
          ref={pagedRef}
          onPointerDown={swipeStart}
          onPointerUp={swipeEnd}
          onPointerCancel={() => {
            swipe.current = null;
          }}
        >
          <div className="reader-measure-probe" ref={probeRef} aria-hidden="true" />
          <div
            className="reader-pages"
            ref={pagesRef}
            style={
              geometry
                ? { width: `${geometry.width}px`, height: `${geometry.height}px` }
                : undefined
            }
            onFocus={(event) => {
              // Keyboard focus may land on a token on another page.
              if (event.target instanceof HTMLElement && event.target !== event.currentTarget) {
                const target = pageOf(event.target);
                if (target !== page) {
                  setPage(target);
                }
              }
            }}
          >
            <div
              className="reader-columns"
              style={
                geometry
                  ? {
                      width: `${geometry.width}px`,
                      height: `${geometry.height}px`,
                      columnWidth: `${geometry.columnWidth}px`,
                      columnGap: `${geometry.gap}px`,
                    }
                  : undefined
              }
            >
              {chapterBody}
            </div>
          </div>
        </main>
      ) : (
        <main className="reader-scroll" ref={scrollRef}>
          <div className="reader-reading">
            {chapterBody}
            {chapterIndex < chapters.length - 1 ? (
              <button
                type="button"
                className="reader-next-chapter"
                onClick={() => openChapter(chapterIndex + 1)}
              >
                <span>{t("reader-next-chapter")}</span>
                <strong>
                  {localizedText(chapters[chapterIndex + 1]?.title ?? {}, locale)}
                </strong>
              </button>
            ) : null}
          </div>
        </main>
      )}

      <footer className="reader-footer">
        {paged ? (
          <button
            type="button"
            className="reader-turn reader-turn-prev"
            aria-label={t("reader-page-prev")}
            title={t("reader-page-prev")}
            disabled={!canTurnBack}
            onClick={() => turn(-1)}
          >
            <StudioIcon name="arrow-left" size={18} />
          </button>
        ) : null}
        <div className="reader-progress-line" aria-hidden="true">
          <span
            style={{
              width: `${paged ? Math.round(((page + 1) / pageCount) * 100) : progress}%`,
            }}
          />
        </div>
        {paged ? (
          <span className="reader-page-status" aria-live="polite">
            <strong>{t("reader-page", { page: page + 1, total: pageCount })}</strong>
            <span>
              {t("reader-chapter-position", {
                chapter: chapterIndex + 1,
                total: chapters.length,
              })}
            </span>
          </span>
        ) : (
          <span>
            {t("reader-progress", {
              chapter: chapterIndex + 1,
              total: chapters.length,
              percent: progress,
            })}
          </span>
        )}
        {paged ? (
          <button
            type="button"
            className="reader-turn reader-turn-next"
            aria-label={t("reader-page-next")}
            title={t("reader-page-next")}
            disabled={!canTurnForward}
            onClick={() => turn(1)}
          >
            <StudioIcon name="arrow-right" size={18} />
          </button>
        ) : null}
      </footer>

      {tocOpen ? (
        <>
          <button
            type="button"
            className="reader-scrim"
            aria-hidden="true"
            tabIndex={-1}
            onClick={() => setTocOpen(false)}
          />
          <nav className="reader-toc" role="dialog" aria-label={t("reader-toc")}>
            <header>
              <div>
                <span className="reader-topbar-level">
                  {t("reader-toc-count", { count: chapters.length })}
                </span>
                <h2>{t("reader-toc-title")}</h2>
              </div>
              <button
                type="button"
                className="reader-icon-button"
                aria-label={t("reader-toc-close")}
                onClick={() => setTocOpen(false)}
              >
                <StudioIcon name="close" size={16} />
              </button>
            </header>
            <ol className="reader-toc-list">
              {chapters.map((entry, index) => (
                <li key={entry.id}>
                  <button
                    type="button"
                    className={`reader-toc-chapter${index === chapterIndex ? " active" : ""}`}
                    aria-current={index === chapterIndex ? "true" : undefined}
                    onClick={() => openChapter(index)}
                  >
                    <span className="reader-toc-number">{index + 1}</span>
                    <span className="reader-toc-title">
                      {localizedText(entry.title, locale)}
                    </span>
                  </button>
                  {(sections[index] ?? []).length > 0 ? (
                    <ol className="reader-toc-sections">
                      {(sections[index] ?? []).map((section: ReaderSection) => (
                        <li key={section.id}>
                          <button
                            type="button"
                            onClick={() => openChapter(index, section.firstSentenceId)}
                          >
                            <span className="reader-toc-number">
                              {index + 1}.{section.index + 1}
                            </span>
                            <span className="reader-toc-title">{section.label}</span>
                          </button>
                        </li>
                      ))}
                    </ol>
                  ) : null}
                </li>
              ))}
            </ol>
          </nav>
        </>
      ) : null}

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
        <ReaderWordPopover
          word={word.text}
          anchor={word.anchor}
          sense={currentSense?.sense ?? null}
          locale={locale}
          sentenceText={word.sentence.text}
          customDefinition={customDefinition}
          onCustomDefinitionChange={setCustomDefinition}
          busy={busy}
          inVocabulary={Boolean(custom)}
          onAdd={() => void addToVocabulary()}
          onClose={() => setWord(null)}
        />
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
