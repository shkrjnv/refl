import { useMemo, useRef, useState } from "react";

import { NotesPanel } from "../components/NotesPanel";
import { Button, StatusMeta } from "../components/ui";
import type { BookParagraph } from "../generated/core/BookParagraph";
import type { BookSentence } from "../generated/core/BookSentence";
import type { BookToken } from "../generated/core/BookToken";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import {
  activeMarks,
  type BookMarkView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import { selectionSpans, sentenceParts, type SelectionSpan } from "../prototype/readerText";

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

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
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
  const articleRef = useRef<HTMLElement | null>(null);
  const chapter = book.book.chapters[chapterIndex];
  const marks = activeMarks(records, book.meta.id);
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

  const located = useMemo(() => {
    const map = new Map<string, LocatedSentence>();
    for (const paragraph of chapter?.paragraphs ?? []) {
      for (const sentence of paragraph.sentences) {
        map.set(sentence.id, { sentence, paragraph });
      }
    }
    return map;
  }, [chapter]);

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

  return (
    <section className="screen reader-screen" aria-labelledby="reader-title">
      <header className="reader-head">
        <Button type="button" variant="ghost" size="sm" onClick={onBack}>
          {t("reader-back")}
        </Button>
        <div>
          <p className="eyebrow">{book.meta.level}</p>
          <h1 id="reader-title">{localizedText(book.meta.title, locale)}</h1>
        </div>
        <StatusMeta tone="neutral">
          {chapter ? `${chapterIndex + 1}/${book.book.chapters.length}` : ""}
        </StatusMeta>
      </header>
      <div className="reader-toolbar">
        <p className="reader-hint">{t("reader-selection-hint")}</p>
        <div className="reader-chapters">
          {book.book.chapters.map((entry, index) => (
            <button
              key={entry.id}
              type="button"
              className={`chapter-chip${index === chapterIndex ? " active" : ""}`}
              onClick={() => {
                setChapterIndex(index);
                setWord(null);
                setSelectionBar(null);
                clearSelection();
              }}
            >
              {localizedText(entry.title, locale)}
            </button>
          ))}
        </div>
      </div>
      {error ? <p className="inline-alert danger">{error}</p> : null}
      {chapter ? (
        <article
          className="reader-paper"
          ref={articleRef}
          onMouseUp={captureSelection}
          onTouchEnd={captureSelection}
          onKeyUp={captureSelection}
        >
          <h2>{localizedText(chapter.title, locale)}</h2>
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
                            <span key={`text-${partIndex}`} className="reader-punct">
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
                              tapToken(event, token, chapter.id, paragraph.id, sentence)
                            }
                            onKeyDown={(event) => {
                              if (event.key === "Enter" || event.key === " ") {
                                event.preventDefault();
                                tapToken(event, token, chapter.id, paragraph.id, sentence);
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
      {marks.length > 0 ? (
        <section className="reader-marks" aria-label={t("reader-marks-title")}>
          <h3>{t("reader-marks-title")}</h3>
          <ul>
            {marks.map((mark) => {
              const sentence = located.get(mark.sentence_id)?.sentence;
              return (
                <li key={mark.mark_id}>
                  <span>
                    {mark.kind === "highlight"
                      ? t("reader-highlight")
                      : t("reader-bookmark")}
                    {sentence ? `: ${sentence.text}` : ""}
                  </span>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => void removeMark(mark)}
                  >
                    {t("notes-delete")}
                  </Button>
                </li>
              );
            })}
          </ul>
        </section>
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
    </section>
  );
}
