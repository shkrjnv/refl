import { useEffect, useMemo, useRef, useState } from "react";

import { BookCover } from "../components/BookCover";
import { BookUploadDialog } from "../components/BookUploadDialog";
import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { StudioIcon } from "../components/StudioIcons";
import { Button, Card, StatusMeta } from "../components/ui";
import type { BookCoverRecord, LearnerStorage, PersonalBookRecord } from "../db/storage";
import type { Explanation } from "../generated/core/Explanation";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { BOOKS, type LoadedBook } from "../prototype/books";
import type { PrototypeContent } from "../prototype/content";
import { localizedText } from "../prototype/content";
import { prepareCover } from "../prototype/cover";
import {
  parseRecords,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import type { BookCoverageResult } from "../wasm/types";
import { BookReader } from "./BookReader";
import { NotesWorkspace } from "./NotesWorkspace";

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
  storage?: LearnerStorage | null;
  onOpenNotes?: (noteId?: string) => void;
}

type Tab = "reference" | "traps" | "books" | "notes";

function depthLabel(depth: Explanation["depth"]): string {
  return depth === "short" ? "short" : depth === "why" ? "why" : "deep";
}

function personalBook(record: PersonalBookRecord, coverUrl: string | null): LoadedBook {
  return {
    meta: {
      id: record.book.id,
      version: record.book.version,
      bytes: record.bytes,
      digest: record.book.version,
      level: record.book.level,
      title: record.book.title,
    },
    book: record.book,
    json: JSON.stringify(record.book),
    coverUrl,
  };
}

export function LibraryScreen({
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
  bookCoverage,
  onPractice,
  storage = null,
  onOpenNotes,
}: LibraryScreenProps) {
  const { t } = useTranslation();
  const records: PersonalRecords = useMemo(
    () => parseRecords(stateJson),
    [stateJson],
  );
  const [tab, setTab] = useState<Tab>("books");
  const [coverage, setCoverage] = useState<Record<string, number | null>>({});
  const [openBook, setOpenBook] = useState<LoadedBook | null>(null);
  const [personalBooks, setPersonalBooks] = useState<PersonalBookRecord[]>([]);
  const [covers, setCovers] = useState<Record<string, string>>({});
  const [uploadOpen, setUploadOpen] = useState(false);
  const [coverTarget, setCoverTarget] = useState<string | null>(null);
  const [coverError, setCoverError] = useState<string | null>(null);
  const [shelfRevision, setShelfRevision] = useState(0);
  const coverInput = useRef<HTMLInputElement | null>(null);
  const [noteTarget, setNoteTarget] = useState<{
    kind: "kc" | "misconception";
    id: string;
    title: string;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!storage) {
      setPersonalBooks([]);
      setCovers({});
      return () => {
        cancelled = true;
      };
    }
    void Promise.all([storage.listPersonalBooks(), storage.listBookCovers()])
      .then(([records, storedCovers]) => {
        if (cancelled) {
          return;
        }
        setPersonalBooks(records);
        setCovers(
          Object.fromEntries(
            storedCovers.map((cover: BookCoverRecord) => [
              cover.book_id,
              cover.data_url,
            ]),
          ),
        );
      })
      .catch(() => {
        if (!cancelled) {
          setCoverError(t("library-cover-error"));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [storage, shelfRevision]);

  const personalIds = useMemo(
    () => new Set(personalBooks.map((record) => record.book.id)),
    [personalBooks],
  );

  const allBooks = useMemo<LoadedBook[]>(() => {
    const builtIn = BOOKS.map((book) => ({
      ...book,
      coverUrl: covers[book.meta.id] ?? book.coverUrl,
    }));
    return [
      ...builtIn,
      ...personalBooks.map((record) =>
        personalBook(record, covers[record.book.id] ?? null),
      ),
    ];
  }, [covers, personalBooks]);

  useEffect(() => {
    let cancelled = false;
    if (stateJson.length === 0) {
      return () => {
        cancelled = true;
      };
    }
    void Promise.all(
      allBooks.map(async (book) => {
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
  }, [stateJson, bookCoverage, allBooks]);

  const removeBook = async (bookId: string) => {
    if (!storage) {
      return;
    }
    setCoverError(null);
    try {
      await storage.deletePersonalBook(bookId);
      setPersonalBooks((current) =>
        current.filter((record) => record.book.id !== bookId),
      );
      setCovers((current) => {
        const next = { ...current };
        delete next[bookId];
        return next;
      });
      setCoverage((current) => {
        const next = { ...current };
        delete next[bookId];
        return next;
      });
      setShelfRevision((value) => value + 1);
    } catch {
      setCoverError(t("library-remove-error"));
    }
  };

  const chooseCover = (bookId: string) => {
    if (!storage) {
      return;
    }
    setCoverTarget(bookId);
    setCoverError(null);
    coverInput.current?.click();
  };

  const saveCover = async (file: File | null) => {
    if (!file || !storage || !coverTarget) {
      return;
    }
    setCoverError(null);
    try {
      const prepared = await prepareCover(file);
      await storage.putBookCover({
        learner_id: storage.learnerId,
        book_id: coverTarget,
        data_url: prepared.dataUrl,
        updated_at: Date.now(),
      });
      setCovers((current) => ({ ...current, [coverTarget]: prepared.dataUrl }));
    } catch {
      setCoverError(t("library-cover-error"));
    } finally {
      setCoverTarget(null);
      if (coverInput.current) {
        coverInput.current.value = "";
      }
    }
  };

  if (openBook) {
    return (
      <BookReader
        book={openBook}
        content={content}
        stateJson={stateJson}
        records={records}
        recordsWriter={recordsWriter}
        onStateChange={onStateChange}
        locale={locale}
        onBack={() => setOpenBook(null)}
        onOpenNotes={onOpenNotes}
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
        {(["books", "reference", "notes", "traps"] as const).map((id) => (
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
                  : id === "notes"
                    ? "library-tab-notes"
                    : "library-tab-traps",
            )}
          </button>
        ))}
      </div>
      {tab === "books" ? (
        <>
          <div className="library-shelf-head">
            <p className="bento-note">
              {t("library-shelf-note", { count: allBooks.length })}
            </p>
            <Button
              type="button"
              variant="primary"
              onClick={() => setUploadOpen(true)}
            >
              {t("library-upload")}
            </Button>
          </div>
          {coverError ? (
            <p className="inline-alert danger" role="alert">
              {coverError}
            </p>
          ) : null}
          <div className="book-grid">
            {allBooks.map((book) => {
              const percent = coverage[book.meta.id] ?? null;
              const title = localizedText(book.meta.title, locale);
              return (
                <article className="book-card" key={book.meta.id}>
                  <BookCover
                    title={title}
                    level={book.meta.level}
                    coverUrl={book.coverUrl}
                    onEdit={storage ? () => chooseCover(book.meta.id) : undefined}
                    editLabel={t("library-change-cover", { title })}
                  />
                  <div className="book-card-head">
                    <span className="book-level-badge">{book.meta.level}</span>
                    <StatusMeta
                      tone={
                        percent === null
                          ? "neutral"
                          : percent >= 80
                            ? "success"
                            : "accent"
                      }
                    >
                      {percent === null
                        ? t("library-coverage-unknown")
                        : t("library-coverage", { percent })}
                    </StatusMeta>
                  </div>
                  <h2>{title}</h2>
                  <p className="book-source">{book.book.source}</p>
                  <div className="book-card-footer">
                    <Button
                      type="button"
                      variant="primary"
                      onClick={() => setOpenBook(book)}
                    >
                      {t("library-open-book")}
                    </Button>
                    {personalIds.has(book.meta.id) && storage ? (
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="book-remove-button"
                        aria-label={t("library-remove-book-aria", { title })}
                        onClick={() => void removeBook(book.meta.id)}
                      >
                        <StudioIcon name="trash" size={16} />
                      </Button>
                    ) : null}
                  </div>
                </article>
              );
            })}
            {allBooks.length === 0 ? (
              <Card tone="plain">
                <p>{t("library-no-books")}</p>
              </Card>
            ) : null}
          </div>
        </>
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
      {tab === "notes" ? (
        <NotesWorkspace
          records={records}
          content={content}
          stateJson={stateJson}
          locale={locale}
          recordsWriter={recordsWriter}
          onStateChange={onStateChange}
          books={allBooks}
          onPractice={onPractice}
          onOpenBook={(bookId) => {
            const book = allBooks.find((entry) => entry.meta.id === bookId);
            if (book) {
              setOpenBook(book);
            }
          }}
        />
      ) : null}
      {noteTarget ? (
        <NotesPanel
          records={records}
          anchor={{ kind: noteTarget.kind, id: noteTarget.id }}
          recordsWriter={recordsWriter}
          stateJson={stateJson}
          onStateChange={onStateChange}
          title={t("notes-panel-title")}
          onOpenWorkspace={onOpenNotes}
          onClose={() => setNoteTarget(null)}
        />
      ) : null}
      <input
        ref={coverInput}
        className="vocab-file-input"
        type="file"
        accept="image/jpeg,image/png,image/webp"
        aria-label={t("library-choose-cover")}
        onChange={(event) => void saveCover(event.currentTarget.files?.[0] ?? null)}
      />
      <BookUploadDialog
        open={uploadOpen}
        content={content}
        storage={storage}
        onClose={() => setUploadOpen(false)}
        onImported={(record, cover) => {
          setPersonalBooks((current) => [
            record,
            ...current.filter((entry) => entry.book.id !== record.book.id),
          ]);
          if (cover) {
            setCovers((current) => ({ ...current, [record.book.id]: cover }));
          }
          setShelfRevision((value) => value + 1);
        }}
      />
      <IellaTip section="library" />
    </section>
  );
}
