import { useState } from "react";

import { StudioIcon } from "../components/StudioIcons";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import { CEFR_LABELS } from "../prototype/progress";
import type { PlacementRunner, PlacementView } from "../prototype/placement";
import type { ItemView } from "../prototype/item";
import type { JsonValue } from "../types/json";

export interface PlacementScreenProps {
  runner: PlacementRunner;
  initialView: PlacementView;
  onComplete: () => void;
  onExit: () => void;
}

function levelLabel(rank: number, preA1: string): string {
  const index = Math.max(0, Math.min(CEFR_LABELS.length - 1, Math.floor(rank)));
  return index === 0 ? preA1 : (CEFR_LABELS[index] ?? preA1);
}

function ItemInput({
  view,
  disabled,
  selected,
  text,
  onSelect,
  onText,
}: {
  view: ItemView;
  disabled: boolean;
  selected: number | null;
  text: string;
  onSelect: (canonicalIndex: number) => void;
  onText: (value: string) => void;
}) {
  if (view.kind === "gap_fill") {
    return (
      <input
        className="answer-input"
        value={text}
        disabled={disabled}
        onChange={(event) => onText(event.target.value)}
        placeholder=""
        aria-label="answer"
      />
    );
  }
  if (view.kind === "find_error") {
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
  return (
    <div className="choice-list">
      {view.choices.map((choice, index) => (
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
            {String.fromCharCode(65 + index)}
          </span>
          <span>{choice.text}</span>
        </button>
      ))}
    </div>
  );
}

export function PlacementScreen({
  runner,
  initialView,
  onComplete,
  onExit,
}: PlacementScreenProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<PlacementView>(initialView);
  const [selected, setSelected] = useState<number | null>(null);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (item: ItemView) => {
    if (busy) {
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
      const next = await runner.submit(response);
      setView(next);
      setSelected(null);
      setText("");
    } catch (caught) {
      setError(
        caught instanceof Error ? caught.message : t("common-unknown-error"),
      );
    } finally {
      setBusy(false);
    }
  };

  if (view.phase === "result") {
    const result = view.result;
    const start = levelLabel(result.startLevel, t("level-pre-a1"));
    const low = result.level
      ? levelLabel(result.level.low, t("level-pre-a1"))
      : start;
    const high = result.level
      ? levelLabel(result.level.high, t("level-pre-a1"))
      : start;
    const range = low === high ? low : `${low}–${high}`;
    return (
      <div className="access-screen">
        <Card tone="plain" className="access-card">
          <p className="eyebrow">{t("placement-eyebrow")}</p>
          <h1>{t("placement-done-title")}</h1>
          <p className="access-body">
            {t("placement-done-body", { level: start })}
          </p>
          <p className="access-note">
            {t("placement-done-range", { range })}
          </p>
          <div className="access-actions">
            <Button type="button" variant="primary" block onClick={onComplete}>
              {t("placement-done-action")}
            </Button>
          </div>
        </Card>
      </div>
    );
  }

  if (view.phase === "unavailable") {
    return (
      <div className="access-screen">
        <Card tone="plain" className="access-card">
          <p className="eyebrow">{t("placement-eyebrow")}</p>
          <h1>{t("placement-title")}</h1>
          <p className="access-body">{t("placement-empty")}</p>
          <div className="access-actions">
            <Button type="button" variant="secondary" block onClick={onExit}>
              {t("placement-exit")}
            </Button>
          </div>
        </Card>
      </div>
    );
  }

  const item = view.item;
  return (
    <div className="lesson-screen">
      <header className="lesson-head">
        <div>
          <h1>{t("placement-title")}</h1>
        </div>
        <div className="lesson-head-actions">
          <Button type="button" variant="ghost" size="sm" onClick={onExit}>
            <StudioIcon name="close" size={16} />
            {t("placement-exit")}
          </Button>
        </div>
      </header>
      <div className="lesson-progress">
        <ProgressBar
          value={
            view.planned === 0
              ? 0
              : Math.round((view.answered / view.planned) * 100)
          }
          label={t("placement-progress", {
            answered: view.answered,
            planned: view.planned,
          })}
        />
        <span>
          {t("placement-progress", {
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

      <Card tone="plain" className="item-card">
        <div className="item-topline">
          <span>{item.instruction}</span>
        </div>
        <p className="item-stimulus">{item.stimulus}</p>
        <ItemInput
          view={item}
          disabled={busy}
          selected={selected}
          text={text}
          onSelect={setSelected}
          onText={setText}
        />
        <Button
          type="button"
          variant="primary"
          block
          busy={busy}
          onClick={() => void submit(item)}
        >
          <StudioIcon name="check" size={16} />
          {t("placement-check")}
        </Button>
        <p className="item-kcs">{t("placement-note")}</p>
      </Card>
    </div>
  );
}
