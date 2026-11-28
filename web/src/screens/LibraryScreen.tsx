import { useEffect, useMemo, useState } from "react";

import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { Button, Card, StatusMeta } from "../components/ui";
import { StudioIcon } from "../components/StudioIcons";
import type { BookSentence } from "../generated/core/BookSentence";
import type { BookToken } from "../generated/core/BookToken";
import type { Explanation } from "../generated/core/Explanation";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { BOOKS, type LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import {
  activeMarks,
  parseRecords,
  type BookMarkView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import type { BookCoverageResult } from "../wasm/types";

export interface LibraryScreenProps {
  content: PrototypeContent;
  stateJson: string;
  locale: Locale;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
  bookCoverage: (
    bookJson: string,
    snapshotJson: string,
    nowMs: number,
  ) => Promise<BookCoverageResult>;
  onPractice: (kcId: string) => void;
}

type Tab = "reference" | "traps" | "books";

function depthLabel(depth: Explanation["depth"]): string {
  return depth === "short" ? "short" : depth === "why" ? "why" : "deep";
}

function tokenLabel(token: BookToken): string {
  return token.text;
}

interface Selection {
  sentenceId: string;
  start: number;
  end: number;
}

interface WordCard {
  text: string;
  lexemeId: string | null;
  sentence: BookSentence;
  chapterId: string;
  paragraphId: string;
}

export function LibraryScreen({
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
  bookCoverage,
  onPractice,
}: LibraryScreenProps) {
  const { t } = useTranslation();
  const records: PersonalRecords = useMemo(
    () => parseRecords(stateJson),
    [stateJson],
  );
  const [tab, setTab] = useState<Tab>("books");
  const [coverage, setCoverage] = useState<Record<string, number | null>>({});
  const [openBook, setOpenBook] = useState<LoadedBook | null>(null);
  const [noteTarget, setNoteTarget] = useState<{
    kind: "kc" | "misconception";
    id: string;
    title: string;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (stateJson.length === 0) {
      return () => {
        cancelled = true;
      };
    }
    void Promise.all(
      BOOKS.map(async (book) => {
        try {
          const result = await bookCoverage(book.json, stateJson, Date.now());
          return [
            book.meta.id,
            result.coverage.total_tokens === 0
              ? null
              : Math.round(result.coverage.percent * 100),
          ] as const;
        } catch {
          return [book.meta.id, null] as const;
        }
      }),
    ).then((entries) => {
      if (!cancelled) {
        setCoverage(Object.fromEntries(entries));
      }
    });
    return () => {
      cancelled = true;
    };
  }, [stateJson, bookCoverage]);

  if (openBook) {
    return (
      <Reader
        book={openBook}
        content={content}
        stateJson={stateJson}
        records={records}
        recordsWriter={recordsWriter}
        onStateChange={onStateChange}
        locale={locale}
        onBack={() => setOpenBook(null)}
      />
    );
  }

  return (
    <section className="screen library-screen" aria-labelledby="library-title">
      <header className="screen-head">
        <div>
          <p className="eyebrow">{t("library-eyebrow")}</p>
          <h1 id="library-title">{t("library-title")}</h1>
          <p className="screen-note">{t("library-note")}</p>
        </div>
      </header>
      <div className="library-tabs" role="tablist" aria-label={t("library-tabs-label")}>
        {(["books", "reference", "traps"] as const).map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={`library-tab${tab === id ? " active" : ""}`}
            onClick={() => setTab(id)}
          >
            {t(
              id === "books"
                ? "library-tab-books"
                : id === "reference"
                  ? "library-tab-reference"
                  : "library-tab-traps",
            )}
          </button>
        ))}
      </div>
      {tab === "books" ? (
        <div className="book-grid">
          {BOOKS.map((book) => {
            const percent = coverage[book.meta.id] ?? null;
            return (
              <Card tone="plain" key={book.meta.id} className="book-card">
                <div className="book-card-head">
                  <span className="book-level-badge">{book.meta.level}</span>
                  <StatusMeta
                    tone={percent === null ? "neutral" : percent >= 80 ? "success" : "accent"}
                  >
                    {percent === null
                      ? t("library-coverage-unknown")
                      : t("library-coverage", { percent })}
                  </StatusMeta>
                </div>
                <h2>{localizedText(book.meta.title, locale)}</h2>
                <p className="book-source">{book.book.source}</p>
                <p className="bento-note">{t("library-coverage-note")}</p>
                <div className="book-card-footer">
                  <Button
                    type="button"
                    variant="primary"
                    onClick={() => setOpenBook(book)}
                  >
                    {t("library-open-book")}
                  </Button>
                </div>
              </Card>
            );
          })}
          {BOOKS.length === 0 ? (
            <Card tone="plain">
              <p>{t("library-no-books")}</p>
            </Card>
          ) : null}
        </div>
      ) : null}
      {tab === "reference" ? (
        <div className="reference-list">
          {content.liveReviews(locale).map((entry) => {
            const explanations = content.explanationsFor("kc", entry.kc.id, locale);
            return (
              <article className="reference-card" key={entry.kc.id}>
                <header>
                  <h2>{content.title(entry, locale)}</h2>
                  <StatusMeta tone="neutral">{entry.kc.level}</StatusMeta>
                </header>
                <p className="reference-rule">{entry.kc.rule}</p>
                {explanations.length === 0 ? (
                  <p className="bento-empty">{t("library-no-explanation")}</p>
                ) : (
                  <div className="explanation-depths">
                    {explanations.map((explanation) => (
                      <details key={explanation.id} open={explanation.depth === "short"}>
                        <summary>{t(`library-depth-${depthLabel(explanation.depth)}`)}</summary>
                        <p>{explanation.body}</p>
                        {explanation.l1_contrast ? (
                          <p className="reference-l1">{explanation.l1_contrast}</p>
                        ) : null}
                      </details>
                    ))}
                  </div>
                )}
                <div className="reference-actions">
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    onClick={() => onPractice(entry.kc.id)}
                  >
                    {t("graph-practice")}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      setNoteTarget({
                        kind: "kc",
                        id: entry.kc.id,
                        title: content.title(entry, locale),
                      })
                    }
                  >
                    {t("graph-note-action")}
                  </Button>
                </div>
              </article>
            );
          })}
        </div>
      ) : null}
      {tab === "traps" ? (
        <div className="reference-list">
          {content.misconceptions
            .filter((misconception) => misconception.origin === "l1-transfer")
            .filter((misconception) =>
              misconception.affects.some((kc) =>
                content.liveKcs().some((entry) => entry.kc.id === kc),
              ) ||
              content.liveKcs().some((entry) => entry.kc.id === misconception.remedy),
            )
            .map((misconception) => {
              const explanations = content.explanationsFor(
                "misconception",
                misconception.id,
                locale,
              );
              return (
                <article className="reference-card" key={misconception.id}>
                  <header>
                    <h2>{localizedText(misconception.title, locale)}</h2>
                    <StatusMeta tone="danger">{t("library-l1-trap")}</StatusMeta>
                  </header>
                  <p className="reference-rule">
                    {t("library-trap-affects", {
                      topics: misconception.affects
                        .map((kc) => content.kcTitle(kc, locale))
                        .join(" · "),
                    })}
                  </p>
                  <p className="reference-rule">
                    {t("library-trap-remedy", {
                      topic: content.kcTitle(misconception.remedy, locale),
                    })}
                  </p>
                  {explanations.map((explanation) => (
                    <p key={explanation.id}>{explanation.body}</p>
                  ))}
                  {(misconception.refutations ?? []).map((refutation, index) => (
                    <p className="reference-l1" key={index}>
                      {refutation.kind === "contrast"
                        ? `${refutation.correct} / ${refutation.incorrect}`
                        : refutation.counterexample}
                    </p>
                  ))}
                </article>
              );
            })}
        </div>
      ) : null}
      {noteTarget ? (
        <NotesPanel
          records={records}
          anchor={{ kind: noteTarget.kind, id: noteTarget.id }}
          recordsWriter={recordsWriter}
          stateJson={stateJson}
          onStateChange={onStateChange}
          title={t("notes-panel-title")}
          onClose={() => setNoteTarget(null)}
        />
      ) : null}
      <IellaTip section="library" />
    </section>
  );
}

function Reader({
  book,
  content,
  stateJson,
  records,
  recordsWriter,
  onStateChange,
  locale,
  onBack,
}: {
  book: LoadedBook;
  content: PrototypeContent;
  stateJson: string;
  records: PersonalRecords;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
  locale: Locale;
  onBack: () => void;
}) {
  const { t } = useTranslation();
  const [chapterIndex, setChapterIndex] = useState(0);
  const [word, setWord] = useState<WordCard | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [noteMark, setNoteMark] = useState<BookMarkView | null>(null);
  const [customDefinition, setCustomDefinition] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
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

  const tapToken = (
    token: BookToken,
    chapterId: string,
    paragraphId: string,
    sentence: BookSentence,
    index: number,
  ) => {
    if (selecting) {
      setSelection((current) => {
        if (!current || current.sentenceId !== sentence.id) {
          return { sentenceId: sentence.id, start: index, end: index };
        }
        return {
          ...current,
          start: Math.min(current.start, index),
          end: Math.max(current.end, index),
        };
      });
      return;
    }
    setCustomDefinition("");
    setWord({
      text: token.text,
      lexemeId:
        token.lexeme_id ??
        (token.sense_ids.length === 1 ? token.sense_ids[0] ?? null : null),
      sentence,
      chapterId,
      paragraphId,
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

  const createMark = async (
    sentence: BookSentence,
    chapterId: string,
    paragraphId: string,
    start: number,
    end: number,
    kind: "highlight" | "bookmark",
  ): Promise<BookMarkView | null> => {
    setBusy(true);
    setError(null);
    try {
      const result = await recordsWriter.createBookMark(stateJson, {
        bookId: book.meta.id,
        bookVersion: book.meta.version,
        chapterId,
        paragraphId,
        sentenceId: sentence.id,
        sentenceText: sentence.text,
        startToken: start,
        endToken: end,
        kind,
      });
      onStateChange(result.stateJson);
      return {
        mark_id: result.markId,
        book_id: book.meta.id,
        book_version: book.meta.version,
        kind,
        chapter_id: chapterId,
        paragraph_id: paragraphId,
        sentence_id: sentence.id,
        start_token: start,
        end_token: end,
        quote_hash: "",
        deleted: false,
        event_id: result.markId,
        updated_at: Date.now(),
      };
    } catch {
      setError(t("reader-mark-error"));
      return null;
    } finally {
      setBusy(false);
    }
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

  const selectedSentence = selection
    ? chapter?.paragraphs
        .flatMap((paragraph) =>
          paragraph.sentences.map((sentence) => ({ sentence, paragraph })),
        )
        .find((entry) => entry.sentence.id === selection.sentenceId)
    : null;

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
        <Button
          type="button"
          variant={selecting ? "primary" : "secondary"}
          size="sm"
          onClick={() => {
            setSelecting((value) => !value);
            setSelection(null);
          }}
        >
          {selecting ? t("reader-select-done") : t("reader-select")}
        </Button>
        {selection && selectedSentence ? (
          <Button
            type="button"
            variant="primary"
            size="sm"
            disabled={busy}
            onClick={() =>
              void createMark(
                selectedSentence.sentence,
                chapter?.id ?? "",
                selectedSentence.paragraph.id,
                selection.start,
                selection.end,
                "highlight",
              ).then((mark) => {
                if (mark) {
                  setSelection(null);
                  setSelecting(false);
                }
              })
            }
          >
            {t("reader-highlight")}
          </Button>
        ) : null}
        <div className="reader-chapters">
          {book.book.chapters.map((entry, index) => (
            <button
              key={entry.id}
              type="button"
              className={`chapter-chip${index === chapterIndex ? " active" : ""}`}
              onClick={() => {
                setChapterIndex(index);
                setWord(null);
                setSelection(null);
                setSelecting(false);
              }}
            >
              {localizedText(entry.title, locale)}
            </button>
          ))}
        </div>
      </div>
      {error ? <p className="inline-alert danger">{error}</p> : null}
      {chapter ? (
        <article className="reader-paper">
          <h2>{localizedText(chapter.title, locale)}</h2>
          {chapter.paragraphs.map((paragraph) => (
            <p key={paragraph.id} className="reader-paragraph">
              {paragraph.sentences.map((sentence) => {
                const sentenceMarks = marks.filter(
                  (mark) => mark.sentence_id === sentence.id,
                );
                return (
                  <span key={sentence.id} className="reader-sentence">
                    {sentence.tokens.map((token, index) => {
                      const highlighted = sentenceMarks.some(
                        (mark) =>
                          mark.kind === "highlight" &&
                          index >= mark.start_token &&
                          index <= mark.end_token,
                      );
                      const selected =
                        selection?.sentenceId === sentence.id &&
                        index >= selection.start &&
                        index <= selection.end;
                      return (
                        <button
                          key={token.id}
                          type="button"
                          className={`reader-token${highlighted ? " highlighted" : ""}${selected ? " selected" : ""}`}
                          onClick={() =>
                            tapToken(
                              token,
                              chapter.id,
                              paragraph.id,
                              sentence,
                              index,
                            )
                          }
                        >
                          {tokenLabel(token)}
                          {" "}
                        </button>
                      );
                    })}
                    <span className="reader-sentence-actions">
                      <button
                        type="button"
                        className="reader-sentence-action"
                        aria-label={t("reader-bookmark-sentence")}
                        onClick={() =>
                          void createMark(
                            sentence,
                            chapter.id,
                            paragraph.id,
                            0,
                            Math.max(0, sentence.tokens.length - 1),
                            "bookmark",
                          )
                        }
                      >
                        <StudioIcon name="bookmark" size={16} />
                      </button>
                      <button
                        type="button"
                        className="reader-sentence-action"
                        aria-label={t("reader-note-sentence")}
                        onClick={() => {
                          const existing = sentenceMarks.find(
                            (mark) => mark.kind === "bookmark",
                          );
                          if (existing) {
                            setNoteMark(existing);
                            return;
                          }
                          void createMark(
                            sentence,
                            chapter.id,
                            paragraph.id,
                            0,
                            Math.max(0, sentence.tokens.length - 1),
                            "bookmark",
                          ).then((mark) => {
                            if (mark) {
                              setNoteMark(mark);
                            }
                          });
                        }}
                      >
                        <StudioIcon name="note" size={16} />
                      </button>
                    </span>
                  </span>
                );
              })}
            </p>
          ))}
        </article>
      ) : null}
      {marks.length > 0 ? (
        <section className="reader-marks" aria-label={t("reader-marks-title")}>
          <h3>{t("reader-marks-title")}</h3>
          <ul>
            {marks.map((mark) => {
              const sentence = chapter?.paragraphs
                .flatMap((paragraph) => paragraph.sentences)
                .find((entry) => entry.id === mark.sentence_id);
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
        <aside className="word-card" aria-label={t("reader-word-card")}>
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
              <p className="vocab-ipa">
                /{currentSense.sense.ipa_us || currentSense.sense.ipa_uk}/
              </p>
              <p>{currentSense.sense.definition_en}</p>
              {locale === "uz" && currentSense.sense.gloss_uz ? (
                <p className="vocab-gloss">{currentSense.sense.gloss_uz}</p>
              ) : null}
              {locale === "ru" && currentSense.sense.gloss_ru ? (
                <p className="vocab-gloss">{currentSense.sense.gloss_ru}</p>
              ) : null}
              {currentSense.sense.examples[0] ? (
                <p className="bento-note">{currentSense.sense.examples[0].text}</p>
              ) : null}
              {currentSense.sense.collocations.length > 0 ? (
                <p className="vocab-collocations">
                  {currentSense.sense.collocations.join(" · ")}
                </p>
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
          onClose={() => setNoteMark(null)}
        />
      ) : null}
    </section>
  );
}
