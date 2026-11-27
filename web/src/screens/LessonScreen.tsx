import { useEffect, useRef, useState } from "react";

import { RichText } from "../components/RichText";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import {
  type LessonRunner,
  type LessonSummary,
  type LessonView,
} from "../prototype/lesson";
import type { ItemView } from "../prototype/item";
import type { JsonValue } from "../types/json";

export interface LessonScreenProps {
  runner: LessonRunner;
  initialView: LessonView;
  onComplete: (summary: LessonSummary) => void;
  onExit: () => void;
  onViewChange?: (view: LessonView) => void;
}

interface ChoiceInputProps {
  view: ItemView;
  disabled: boolean;
  selected: number | null;
  onSelect: (canonicalIndex: number) => void;
}

function ChoiceInput({ view, disabled, selected, onSelect }: ChoiceInputProps) {
  return (
    <div className="choice-list">
      {view.choices.map((choice) => (
        <button
          key={`${choice.canonicalIndex}:${choice.text}`}
          type="button"
          className={`choice-option${
            selected === choice.canonicalIndex ? " selected" : ""
          }`}
          disabled={disabled}
          aria-pressed={selected === choice.canonicalIndex}
          onClick={() => onSelect(choice.canonicalIndex)}
        >
          <span className="choice-letter" aria-hidden="true">
            {String.fromCharCode(65 + view.choices.indexOf(choice))}
          </span>
          <span>{choice.text}</span>
        </button>
      ))}
    </div>
  );
}

interface FindErrorInputProps {
  view: ItemView;
  disabled: boolean;
  selected: number | null;
  onSelect: (canonicalIndex: number) => void;
}

function FindErrorInput({
  view,
  disabled,
  selected,
  onSelect,
}: FindErrorInputProps) {
  return (
    <div className="token-sentence" aria-label={view.stimulus}>
      {view.tokens.map((token, index) => {
        const canonical = view.tokenChoice[index] ?? -1;
        const active = canonical >= 0 && canonical === selected;
        return (
          <button
            key={`${token}:${index}`}
            type="button"
            className={`sentence-token${active ? " selected" : ""}`}
            disabled={disabled || canonical < 0}
            onClick={() => {
              if (canonical >= 0) {
                onSelect(canonical);
              }
            }}
          >
            {token}
          </button>
        );
      })}
    </div>
  );
}

export function LessonScreen({
  runner,
  initialView,
  onComplete,
  onExit,
  onViewChange,
}: LessonScreenProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<LessonView>(initialView);
  const [selected, setSelected] = useState<number | null>(null);
  const [text, setText] = useState("");
  const [hintOpen, setHintOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const completionRef = useRef(false);

  useEffect(() => {
    onViewChange?.(view);
  }, [onViewChange, view]);

  useEffect(() => {
    if (view.phase === "summary" && view.summary && !completionRef.current) {
      completionRef.current = true;
      onComplete(view.summary);
    }
  }, [onComplete, view]);

  const item = view.item;
  const phaseLabel =
    view.phase === "pretest"
      ? t("lesson-pretest")
      : view.phase === "explanation"
        ? t("lesson-explanation")
        : view.phase === "summary"
          ? t("lesson-summary")
          : t("lesson-practice");

  const submit = async () => {
    if (!item || busy) {
      return;
    }
    let response: JsonValue;
    if (item.kind === "gap_fill") {
      if (text.trim().length === 0) {
        return;
      }
      response = { kind: "text", value: text.trim() };
    } else {
      if (selected === null) {
        return;
      }
      response = { kind: "option", value: selected };
    }
    setBusy(true);
    setError(null);
    try {
      const next =
        view.phase === "pretest"
          ? await runner.submitPretest(response)
          : await runner.submitAnswer(response);
      setView(next);
      setHintOpen(false);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : t("common-unknown-error"));
    } finally {
      setBusy(false);
    }
  };

  const continueExplanation = async () => {
    setBusy(true);
    setError(null);
    try {
      setView(await runner.continueExplanation());
      setSelected(null);
      setText("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : t("common-unknown-error"));
    } finally {
      setBusy(false);
    }
  };

  const advance = async () => {
    setBusy(true);
    setError(null);
    try {
      setView(await runner.advanceAfterFeedback());
      setSelected(null);
      setText("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : t("common-unknown-error"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="lesson-screen">
      <header className="lesson-head">
        <div>
          <p className="eyebrow">{t("lesson-session-eyebrow")}</p>
          <h1>{phaseLabel}</h1>
        </div>
        <Button type="button" variant="ghost" size="sm" onClick={onExit}>
          {t("lesson-exit")}
        </Button>
      </header>
      <div className="lesson-progress">
        <ProgressBar
          value={
            view.planned === 0
              ? 0
              : Math.round((view.answered / view.planned) * 100)
          }
          label={t("lesson-progress-label")}
        />
        <span>
          {t("lesson-progress", {
            answered: view.answered,
            planned: view.planned,
          })}
        </span>
      </div>

      {error ? (
        <p className="inline-alert danger" role="alert">
          {error}
        </p>
      ) : null}

      {view.phase === "pretest" && item ? (
        <Card tone="plain" className="item-card">
          <div className="item-topline">
            <span>{item.instruction}</span>
            <button
              type="button"
              className="hint-toggle"
              onClick={() => setHintOpen((open) => !open)}
            >
              {t("lesson-hint")}
            </button>
          </div>
          <p className="item-stimulus">{item.stimulus}</p>
          {hintOpen ? <p className="item-hint">{item.hint}</p> : null}
          {item.kind === "gap_fill" ? (
            <input
              className="answer-input"
              value={text}
              disabled={busy}
              onChange={(event) => setText(event.target.value)}
              placeholder={t("lesson-answer-placeholder")}
              aria-label={t("lesson-answer-label")}
            />
          ) : item.kind === "find_error" ? (
            <FindErrorInput
              view={item}
              disabled={busy}
              selected={selected}
              onSelect={setSelected}
            />
          ) : (
            <ChoiceInput
              view={item}
              disabled={busy}
              selected={selected}
              onSelect={setSelected}
            />
          )}
          <Button
            type="button"
            variant="primary"
            block
            busy={busy}
            onClick={() => void submit()}
          >
            {t("lesson-check")}
          </Button>
          <p className="item-kcs">{item.kcTitles.join(" · ")}</p>
        </Card>
      ) : null}

      {view.phase === "explanation" && view.explanation ? (
        <Card tone="tinted" className="explanation-card">
          <p className="eyebrow">{t("lesson-explanation-eyebrow")}</p>
          <h2>{view.explanation.title}</h2>
          <p className="explanation-body">
            <RichText text={view.explanation.body} />
          </p>
          {view.explanation.l1Contrast ? (
            <p className="explanation-contrast">
              <RichText text={view.explanation.l1Contrast} />
            </p>
          ) : null}
          <div className="example-list">
            {view.explanation.examples.map((example) => (
              <span key={example}>{example}</span>
            ))}
          </div>
          {view.feedback ? (
            <p className="pretest-note">{t("lesson-pretest-note")}</p>
          ) : null}
          <Button
            type="button"
            variant="primary"
            block
            busy={busy}
            onClick={() => void continueExplanation()}
          >
            {t("lesson-got-it")}
          </Button>
        </Card>
      ) : null}

      {view.phase === "practice" && item ? (
        <Card tone="plain" className="item-card">
          <div className="item-topline">
            <span>{item.instruction}</span>
            <button
              type="button"
              className="hint-toggle"
              onClick={() => setHintOpen((open) => !open)}
            >
              {t("lesson-hint")}
            </button>
          </div>
          <p className="item-stimulus">{item.stimulus}</p>
          {hintOpen ? <p className="item-hint">{item.hint}</p> : null}
          {item.kind === "gap_fill" ? (
            <input
              className="answer-input"
              value={text}
              disabled={busy}
              onChange={(event) => setText(event.target.value)}
              placeholder={t("lesson-answer-placeholder")}
              aria-label={t("lesson-answer-label")}
            />
          ) : item.kind === "find_error" ? (
            <FindErrorInput
              view={item}
              disabled={busy}
              selected={selected}
              onSelect={setSelected}
            />
          ) : (
            <ChoiceInput
              view={item}
              disabled={busy}
              selected={selected}
              onSelect={setSelected}
            />
          )}
          <Button
            type="button"
            variant="primary"
            block
            busy={busy}
            onClick={() => void submit()}
          >
            {t("lesson-check")}
          </Button>
          <p className="item-kcs">{item.kcTitles.join(" · ")}</p>
        </Card>
      ) : null}

      {view.phase === "feedback" && view.feedback ? (
        <Card
          tone="plain"
          className={`feedback-card ${
            view.feedback.correct ? "feedback-correct" : "feedback-wrong"
          }`}
        >
          <p className="feedback-verdict">
            {view.feedback.correct
              ? t("lesson-answer-correct")
              : t("lesson-answer-wrong")}
          </p>
          <p className="feedback-why">
            <RichText text={view.feedback.why} />
          </p>
          {view.feedback.misconception ? (
            <p className="feedback-misconception">
              <RichText text={view.feedback.misconception} />
            </p>
          ) : null}
          {!view.feedback.correct ? (
            <p className="feedback-answer">
              {t("lesson-correct-answer", {
                answer: view.feedback.correctAnswer,
              })}
            </p>
          ) : null}
          <Button
            type="button"
            variant="primary"
            block
            busy={busy}
            onClick={() => void advance()}
          >
            {t("lesson-next")}
          </Button>
        </Card>
      ) : null}
    </div>
  );
}
