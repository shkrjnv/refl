import { useMemo, useState } from "react";

import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { Button, Card, StatusMeta } from "../components/ui";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { PrototypeContent } from "../prototype/content";
import { retrievability } from "../prototype/progress";
import {
  parseRecords,
  type CustomLexemeView,
  type LexicalCardView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";

export interface VocabularyScreenProps {
  content: PrototypeContent;
  stateJson: string;
  locale: Locale;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
}

type Status = "all" | "new" | "learning" | "in_memory" | "due";

interface WordView {
  id: string;
  lemma: string;
  pos: string;
  level: string;
  definition: string;
  glossUz: string;
  glossRu: string;
  examples: string[];
  collocations: string[];
  ipa: string;
  audioKey: string | null;
  card: LexicalCardView | null;
  custom: CustomLexemeView | null;
  bundleId: string | null;
}

function statusOf(card: LexicalCardView | null, now: number): Exclude<Status, "all"> {
  if (!card) {
    return "new";
  }
  if (card.due <= now) {
    return "due";
  }
  const recall = retrievability(card.stability, card.last_review, now);
  return recall >= 0.8 && card.review_count >= 2 ? "in_memory" : "learning";
}

const STATUS_KEYS: Record<Exclude<Status, "all">, string> = {
  new: "vocab-status-new",
  learning: "vocab-status-learning",
  in_memory: "vocab-status-memory",
  due: "vocab-status-due",
};

function gloss(word: WordView, locale: Locale): string {
  if (locale === "uz") {
    return word.glossUz;
  }
  if (locale === "ru") {
    return word.glossRu;
  }
  return "";
}

interface FormState {
  id?: string;
  lemma: string;
  definition: string;
  glossUz: string;
  glossRu: string;
  examples: string;
  collocations: string;
}

const EMPTY_FORM: FormState = {
  lemma: "",
  definition: "",
  glossUz: "",
  glossRu: "",
  examples: "",
  collocations: "",
};

export function VocabularyScreen({
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
}: VocabularyScreenProps) {
  const { t } = useTranslation();
  const records: PersonalRecords = useMemo(
    () => parseRecords(stateJson),
    [stateJson],
  );
  const now = Date.now();
  const words = useMemo<WordView[]>(() => {
    const bank: WordView[] = content.lexemes.flatMap((lexeme) =>
      lexeme.senses.map((sense) => {
        const custom =
          Object.values(records.customLexemes).find(
            (record) => !record.archived && record.bundle_lexeme_id === sense.id,
          ) ?? null;
        const card = records.lexicalCards[sense.id]?.recognize ?? null;
        return {
          id: sense.id,
          lemma: lexeme.lemma,
          pos: lexeme.pos,
          level: lexeme.level,
          definition: custom?.definition_en || sense.definition_en,
          glossUz: custom?.gloss_uz || sense.gloss_uz,
          glossRu: custom?.gloss_ru || sense.gloss_ru,
          examples:
            custom && custom.examples.length > 0
              ? custom.examples
              : sense.examples.map((example) => example.text),
          collocations:
            custom && custom.collocations.length > 0
              ? custom.collocations
              : sense.collocations,
          ipa: custom?.ipa || sense.ipa_us || sense.ipa_uk,
          audioKey: custom?.audio_key ?? sense.audio_key ?? null,
          card,
          custom,
          bundleId: sense.id,
        };
      }),
    );
    const personal: WordView[] = Object.values(records.customLexemes)
      .filter((record) => !record.archived && !record.bundle_lexeme_id)
      .map((record) => ({
        id: record.custom_id,
        lemma: record.lemma,
        pos: record.pos ?? "",
        level: "",
        definition: record.definition_en,
        glossUz: record.gloss_uz,
        glossRu: record.gloss_ru,
        examples: record.examples,
        collocations: record.collocations,
        ipa: record.ipa ?? "",
        audioKey: record.audio_key ?? null,
        card: records.lexicalCards[record.custom_id]?.recognize ?? null,
        custom: record,
        bundleId: null,
      }));
    return [...bank, ...personal].sort((left, right) =>
      left.lemma.localeCompare(right.lemma),
    );
  }, [content, records]);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState<Status>("all");
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [formOpen, setFormOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reviewing, setReviewing] = useState<WordView[] | null>(null);
  const [reviewIndex, setReviewIndex] = useState(0);
  const [revealed, setRevealed] = useState(false);
  const [noteTarget, setNoteTarget] = useState<WordView | null>(null);

  const filtered = words.filter((word) => {
    const needle = query.trim().toLocaleLowerCase();
    const haystack = [
      word.lemma,
      word.definition,
      word.glossUz,
      word.glossRu,
      ...word.collocations,
    ]
      .join(" ")
      .toLocaleLowerCase();
    if (needle.length > 0 && !haystack.includes(needle)) {
      return false;
    }
    return status === "all" || statusOf(word.card, now) === status;
  });
  const due = words.filter((word) => statusOf(word.card, now) === "due");
  const customForEdit = form.id ? records.customLexemes[form.id] ?? null : null;

  const startReview = () => {
    if (due.length === 0) {
      return;
    }
    setReviewing(due);
    setReviewIndex(0);
    setRevealed(false);
  };

  const answerReview = async (correct: boolean) => {
    if (!reviewing) {
      return;
    }
    const word = reviewing[reviewIndex];
    if (!word) {
      setReviewing(null);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = await recordsWriter.reviewLexeme(stateJson, word.id, correct);
      onStateChange(next);
      if (reviewIndex + 1 >= reviewing.length) {
        setReviewing(null);
      } else {
        setReviewIndex((index) => index + 1);
        setRevealed(false);
      }
    } catch {
      setError(t("vocab-error-save"));
    } finally {
      setBusy(false);
    }
  };

  const saveCustom = async () => {
    setBusy(true);
    setError(null);
    try {
      const match = content.senseForForm(form.lemma);
      const next = await recordsWriter.saveCustomWord(stateJson, {
        customId: form.id,
        lemma: form.lemma,
        definitionEn: form.definition,
        glossUz: form.glossUz,
        glossRu: form.glossRu,
        examples: form.examples.split("\n"),
        collocations: form.collocations.split(","),
        ...(match
          ? {
              bundleLexemeId: match.sense.id,
              ipa: match.sense.ipa_us || match.sense.ipa_uk,
              audioKey: match.sense.audio_key,
            }
          : {}),
      });
      onStateChange(next);
      setForm(EMPTY_FORM);
      setFormOpen(false);
    } catch {
      setError(t("vocab-error-save"));
    } finally {
      setBusy(false);
    }
  };

  const reviewWord = reviewing?.[reviewIndex] ?? null;
  if (reviewWord) {
    return (
      <section className="screen vocab-screen" aria-labelledby="vocab-review-title">
        <header className="screen-head">
          <div>
            <p className="eyebrow">{t("vocab-review-eyebrow")}</p>
            <h1 id="vocab-review-title">{reviewWord.lemma}</h1>
          </div>
          <StatusMeta tone="accent">
            {t("vocab-review-progress", {
              current: reviewIndex + 1,
              total: reviewing?.length ?? 0,
            })}
          </StatusMeta>
        </header>
        {!revealed ? (
          <Card tone="plain" className="vocab-review-card">
            <p>{t("vocab-review-prompt")}</p>
            <Button type="button" variant="primary" onClick={() => setRevealed(true)}>
              {t("vocab-review-show")}
            </Button>
          </Card>
        ) : (
          <Card tone="plain" className="vocab-review-card">
            <h2>{reviewWord.definition}</h2>
            {gloss(reviewWord, locale) ? (
              <p className="vocab-gloss">{gloss(reviewWord, locale)}</p>
            ) : null}
            <div className="vocab-review-actions">
              <Button
                type="button"
                variant="secondary"
                disabled={busy}
                onClick={() => void answerReview(false)}
              >
                {t("vocab-review-again")}
              </Button>
              <Button
                type="button"
                variant="primary"
                disabled={busy}
                onClick={() => void answerReview(true)}
              >
                {t("vocab-review-good")}
              </Button>
            </div>
          </Card>
        )}
        {error ? <p className="inline-alert danger">{error}</p> : null}
        <Button type="button" variant="ghost" onClick={() => setReviewing(null)}>
          {t("vocab-review-exit")}
        </Button>
        <IellaTip section="vocabulary" />
      </section>
    );
  }

  return (
    <section className="screen vocab-screen" aria-labelledby="vocab-title">
      <header className="screen-head">
        <div>
          <p className="eyebrow">{t("vocab-eyebrow")}</p>
          <h1 id="vocab-title">{t("vocab-title")}</h1>
          <p className="screen-note">{t("vocab-note", { count: words.length })}</p>
        </div>
        <Button
          type="button"
          variant="primary"
          disabled={due.length === 0}
          onClick={startReview}
        >
          {t("vocab-review-start", { count: due.length })}
        </Button>
      </header>
      <div className="vocab-tools">
        <label className="field vocab-search">
          <span>{t("vocab-search-label")}</span>
          <input
            className="answer-input"
            type="search"
            value={query}
            placeholder={t("vocab-search-placeholder")}
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
        <div className="vocab-filters" role="group" aria-label={t("vocab-filter-label")}>
          {(["all", "new", "learning", "in_memory", "due"] as const).map((id) => (
            <button
              key={id}
              type="button"
              className={`vocab-filter${status === id ? " active" : ""}`}
              aria-pressed={status === id}
              onClick={() => setStatus(id)}
            >
              {id === "all" ? t("vocab-filter-all") : t(STATUS_KEYS[id])}
            </button>
          ))}
        </div>
      </div>
      {error ? <p className="inline-alert danger">{error}</p> : null}
      <div className="vocab-list">
        {filtered.length === 0 ? (
          <Card tone="plain">
            <p>{t("vocab-empty")}</p>
          </Card>
        ) : (
          filtered.map((word) => {
            const wordStatus = statusOf(word.card, now);
            const wordGloss = gloss(word, locale);
            return (
              <article className="vocab-card" key={word.id}>
                <div className="vocab-card-head">
                  <div>
                    <h2>{word.lemma}</h2>
                    <p className="vocab-meta">
                      {[word.pos, word.level].filter(Boolean).join(" · ")}
                      {word.ipa ? <span className="vocab-ipa">/{word.ipa}/</span> : null}
                    </p>
                  </div>
                  <StatusMeta
                    tone={
                      wordStatus === "in_memory"
                        ? "success"
                        : wordStatus === "due"
                          ? "danger"
                          : wordStatus === "learning"
                            ? "accent"
                            : "neutral"
                    }
                  >
                    {t(STATUS_KEYS[wordStatus])}
                  </StatusMeta>
                </div>
                <p className="vocab-definition">{word.definition}</p>
                {wordGloss ? <p className="vocab-gloss">{wordGloss}</p> : null}
                {word.examples.length > 0 ? (
                  <ul className="vocab-examples">
                    {word.examples.slice(0, 2).map((example) => (
                      <li key={example}>{example}</li>
                    ))}
                  </ul>
                ) : null}
                {word.collocations.length > 0 ? (
                  <p className="vocab-collocations">
                    {word.collocations.slice(0, 4).join(" · ")}
                  </p>
                ) : null}
                <div className="vocab-card-actions">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setNoteTarget(word)}
                  >
                    {t("graph-note-action")}
                  </Button>
                </div>
                {word.custom ? (
                  <div className="vocab-card-actions">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setForm({
                          id: word.custom?.custom_id,
                          lemma: word.custom?.lemma ?? "",
                          definition: word.custom?.definition_en ?? "",
                          glossUz: word.custom?.gloss_uz ?? "",
                          glossRu: word.custom?.gloss_ru ?? "",
                          examples: (word.custom?.examples ?? []).join("\n"),
                          collocations: (word.custom?.collocations ?? []).join(", "),
                        });
                        setFormOpen(true);
                      }}
                    >
                      {t("vocab-edit-word")}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      disabled={busy}
                      onClick={() => {
                        setBusy(true);
                        void recordsWriter
                          .archiveCustomWord(stateJson, word.custom?.custom_id ?? "")
                          .then(onStateChange)
                          .catch(() => setError(t("vocab-error-save")))
                          .finally(() => setBusy(false));
                      }}
                    >
                      {t("vocab-archive-word")}
                    </Button>
                  </div>
                ) : null}
              </article>
            );
          })
        )}
      </div>
      <Card tone="plain" className="vocab-form-card">
        <div className="vocab-form-head">
          <h2>{formOpen ? t("vocab-form-edit-title") : t("vocab-form-title")}</h2>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              setFormOpen((open) => !open);
              if (formOpen) {
                setForm(EMPTY_FORM);
              }
            }}
          >
            {formOpen ? t("vocab-form-close") : t("vocab-form-open")}
          </Button>
        </div>
        {formOpen ? (
          <form
            className="vocab-form"
            onSubmit={(event) => {
              event.preventDefault();
              void saveCustom();
            }}
          >
            <label className="field">
              <span>{t("vocab-form-lemma")}</span>
              <input
                className="answer-input"
                value={form.lemma}
                required
                onChange={(event) =>
                  setForm({ ...form, lemma: event.currentTarget.value })
                }
              />
            </label>
            <label className="field">
              <span>{t("vocab-form-definition")}</span>
              <textarea
                className="answer-input"
                rows={2}
                value={form.definition}
                required
                onChange={(event) =>
                  setForm({ ...form, definition: event.currentTarget.value })
                }
              />
            </label>
            <div className="vocab-form-row">
              <label className="field">
                <span>{t("vocab-form-gloss-uz")}</span>
                <input
                  className="answer-input"
                  value={form.glossUz}
                  onChange={(event) =>
                    setForm({ ...form, glossUz: event.currentTarget.value })
                  }
                />
              </label>
              <label className="field">
                <span>{t("vocab-form-gloss-ru")}</span>
                <input
                  className="answer-input"
                  value={form.glossRu}
                  onChange={(event) =>
                    setForm({ ...form, glossRu: event.currentTarget.value })
                  }
                />
              </label>
            </div>
            <label className="field">
              <span>{t("vocab-form-examples")}</span>
              <textarea
                className="answer-input"
                rows={2}
                value={form.examples}
                onChange={(event) =>
                  setForm({ ...form, examples: event.currentTarget.value })
                }
              />
            </label>
            <label className="field">
              <span>{t("vocab-form-collocations")}</span>
              <input
                className="answer-input"
                value={form.collocations}
                onChange={(event) =>
                  setForm({ ...form, collocations: event.currentTarget.value })
                }
              />
            </label>
            {content.senseForForm(form.lemma) ? (
              <p className="bento-note">{t("vocab-form-bank-match")}</p>
            ) : null}
            {customForEdit ? null : null}
            <Button type="submit" variant="primary" disabled={busy}>
              {form.id ? t("vocab-form-save-edit") : t("vocab-form-save")}
            </Button>
          </form>
        ) : null}
      </Card>
      {noteTarget ? (
        <NotesPanel
          records={records}
          anchor={{ kind: "lexeme", id: noteTarget.id }}
          recordsWriter={recordsWriter}
          stateJson={stateJson}
          onStateChange={onStateChange}
          title={t("notes-panel-title")}
          onClose={() => setNoteTarget(null)}
        />
      ) : null}
      <IellaTip section="vocabulary" />
    </section>
  );
}
