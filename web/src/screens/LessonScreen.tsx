import { useEffect, useRef, useState } from "react";

import { NotesPanel } from "../components/NotesPanel";
import { RichText } from "../components/RichText";
import { StudioIcon } from "../components/StudioIcons";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import {
  type LessonRunner,
  type LessonSummary,
  type LessonView,
} from "../prototype/lesson";
import type { ItemView, PresentedChoice } from "../prototype/item";
import { parseRecords, type RecordStore } from "../prototype/records";
import type { JsonValue } from "../types/json";

export interface LessonScreenProps {
  runner: LessonRunner;
  initialView: LessonView;
  onComplete: (summary: LessonSummary) => void;
  onExit: () => void;
  onViewChange?: (view: LessonView) => void;
  recordsWriter?: RecordStore | null;
  stateJson?: string;
  onStateChange?: (stateJson: string) => void;
  onOpenNotes?: (noteId?: string) => void;
}

interface ChoiceInputProps {
  view: ItemView;
  disabled: boolean;
  selected: number | null;
  onSelect: (canonicalIndex: number) => void;
}

const CHOICE_DRAG_BOUND = 24;
const CHOICE_DRAG_DAMPING = 0.28;

interface DraggableChoiceProps {
  choice: PresentedChoice;
  letter: string;
  disabled: boolean;
  selected: boolean;
  onSelect: () => void;
}

function DraggableChoice({
  choice,
  letter,
  disabled,
  selected,
  onSelect,
}: DraggableChoiceProps) {
  const [offset, setOffset] = useState(0);
  const [snapping, setSnapping] = useState(false);
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    moved: boolean;
  } | null>(null);
  const suppressClickRef = useRef(false);
  const snapTimerRef = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (snapTimerRef.current !== null) {
        window.clearTimeout(snapTimerRef.current);
      }
    },
    [],
  );

  const settle = () => {
    setOffset(0);
    setSnapping(true);
    if (snapTimerRef.current !== null) {
      window.clearTimeout(snapTimerRef.current);
    }
    snapTimerRef.current = window.setTimeout(() => {
      snapTimerRef.current = null;
      setSnapping(false);
    }, 340);
  };

  return (
    <button
      type="button"
      className={`choice-option${selected ? " selected" : ""}${
        snapping ? " snapping" : ""
      }`}
      disabled={disabled}
      aria-pressed={selected}
      style={{ transform: `translateX(${offset}px)` }}
      onPointerDown={(event) => {
        if (disabled) {
          return;
        }
        dragRef.current = {
          pointerId: event.pointerId,
          startX: event.clientX,
          moved: false,
        };
        setSnapping(false);
        event.currentTarget.setPointerCapture?.(event.pointerId);
      }}
      onPointerMove={(event) => {
        const current = dragRef.current;
        if (!current || current.pointerId !== event.pointerId) {
          return;
        }
        const delta = event.clientX - current.startX;
        if (Math.abs(delta) > 4) {
          current.moved = true;
        }
        const magnitude = Math.abs(delta);
        const resisted =
          magnitude <= CHOICE_DRAG_BOUND
            ? delta
            : Math.sign(delta) *
              (CHOICE_DRAG_BOUND +
                (magnitude - CHOICE_DRAG_BOUND) * CHOICE_DRAG_DAMPING);
        setOffset(
          Math.max(
            -CHOICE_DRAG_BOUND * 1.6,
            Math.min(CHOICE_DRAG_BOUND * 1.6, resisted),
          ),
        );
      }}
      onPointerUp={() => {
        const current = dragRef.current;
        if (!current) {
          return;
        }
        dragRef.current = null;
        if (current.moved) {
          suppressClickRef.current = true;
          window.setTimeout(() => {
            suppressClickRef.current = false;
          }, 0);
        }
        settle();
      }}
      onPointerCancel={() => {
        dragRef.current = null;
        settle();
      }}
      onClick={(event) => {
        if (suppressClickRef.current) {
          event.preventDefault();
          event.stopPropagation();
          return;
        }
        onSelect();
      }}
    >
      <span className="choice-letter" aria-hidden="true">
        {letter}
      </span>
      <span>{choice.text}</span>
    </button>
  );
}

function ChoiceInput({ view, disabled, selected, onSelect }: ChoiceInputProps) {
  return (
    <div className="choice-list">
      {view.choices.map((choice, index) => (
        <DraggableChoice
          key={`${choice.canonicalIndex}:${choice.text}`}
          choice={choice}
          letter={String.fromCharCode(65 + index)}
          disabled={disabled}
          selected={selected === choice.canonicalIndex}
          onSelect={() => onSelect(choice.canonicalIndex)}
        />
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
  recordsWriter,
  stateJson,
  onStateChange,
  onOpenNotes,
}: LessonScreenProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<LessonView>(initialView);
  const [selected, setSelected] = useState<number | null>(null);
  const [text, setText] = useState("");
  const [hintOpen, setHintOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [noteOpen, setNoteOpen] = useState(false);
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
      if (next.feedback) {
      }
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
          <h1>{phaseLabel}</h1>
        </div>
        <div className="lesson-head-actions">
          {recordsWriter && item && stateJson ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setNoteOpen((open) => !open)}
            >
              {t("graph-note-action")}
            </Button>
          ) : null}
          <Button type="button" variant="ghost" size="sm" onClick={onExit}>
            <StudioIcon name="close" size={16} />
            {t("lesson-exit")}
          </Button>
        </div>
      </header>
      {noteOpen && recordsWriter && item && stateJson && onStateChange ? (
        <NotesPanel
          records={parseRecords(stateJson)}
          anchor={{
            kind: "item",
            id: `${item.ref.id}@${item.ref.version}`,
          }}
          recordsWriter={recordsWriter}
          stateJson={stateJson}
          onStateChange={onStateChange}
          title={t("notes-panel-title")}
          onOpenWorkspace={onOpenNotes}
          onClose={() => setNoteOpen(false)}
        />
      ) : null}
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
              <StudioIcon name="spark" size={15} />
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
            <StudioIcon name="check" size={16} />
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
            <StudioIcon name="check" size={16} />
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
            <StudioIcon name="check" size={16} />
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
            <StudioIcon
              name={view.feedback.correct ? "check" : "close"}
              size={18}
            />
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
            <StudioIcon name="arrow-right" size={16} />
            {t("lesson-next")}
          </Button>
        </Card>
      ) : null}
    </div>
  );
}
