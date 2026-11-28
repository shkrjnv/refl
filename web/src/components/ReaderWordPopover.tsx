import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";

import type { LexemeSense } from "../generated/core/LexemeSense";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { StudioIcon } from "./StudioIcons";
import { Button } from "./ui";

/** The selected word's viewport rectangle, captured when it is tapped. */
export interface ReaderWordAnchor {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface ReaderWordPopoverProps {
  word: string;
  anchor: ReaderWordAnchor;
  sense: LexemeSense | null;
  locale: Locale;
  sentenceText: string;
  customDefinition: string;
  onCustomDefinitionChange: (value: string) => void;
  busy: boolean;
  inVocabulary: boolean;
  onAdd: () => void;
  onClose: () => void;
}

type Placement = "above" | "below";

interface PopoverLayout {
  top: number;
  left: number;
  placement: Placement;
  arrowLeft: number;
}

const VIEWPORT_MARGIN = 8;
const WORD_GAP = 10;

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

export function ReaderWordPopover({
  word,
  anchor,
  sense,
  locale,
  sentenceText,
  customDefinition,
  onCustomDefinitionChange,
  busy,
  inVocabulary,
  onAdd,
  onClose,
}: ReaderWordPopoverProps) {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLElement | null>(null);
  const [layout, setLayout] = useState<PopoverLayout | null>(null);

  // Measure the rendered card and choose the side with room, so the card
  // never sits on top of the word it explains and never leaves the viewport.
  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }
    const update = () => {
      const width = panel.offsetWidth;
      const height = panel.offsetHeight;
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const anchorCenter = (anchor.left + anchor.right) / 2;
      const left = clamp(
        anchorCenter - width / 2,
        VIEWPORT_MARGIN,
        Math.max(VIEWPORT_MARGIN, viewportWidth - width - VIEWPORT_MARGIN),
      );
      const spaceBelow =
        viewportHeight - anchor.bottom - WORD_GAP - VIEWPORT_MARGIN;
      const spaceAbove = anchor.top - WORD_GAP - VIEWPORT_MARGIN;
      const placement: Placement =
        spaceBelow >= height || spaceBelow >= spaceAbove ? "below" : "above";
      const wantedTop =
        placement === "below"
          ? anchor.bottom + WORD_GAP
          : anchor.top - WORD_GAP - height;
      const top = clamp(
        wantedTop,
        VIEWPORT_MARGIN,
        Math.max(VIEWPORT_MARGIN, viewportHeight - height - VIEWPORT_MARGIN),
      );
      setLayout({
        top,
        left,
        placement,
        arrowLeft: clamp(anchorCenter - left, 20, Math.max(20, width - 20)),
      });
    };
    update();
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(update);
    observer?.observe(panel);
    window.addEventListener("resize", update);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", update);
    };
  }, [
    anchor.bottom,
    anchor.left,
    anchor.right,
    anchor.top,
    customDefinition,
    sense,
  ]);

  const style: CSSProperties = layout
    ? { top: layout.top, left: layout.left }
    : {
        top: anchor.bottom + WORD_GAP,
        left: clamp(anchor.left, VIEWPORT_MARGIN, VIEWPORT_MARGIN),
        visibility: "hidden",
      };
  const placement = layout?.placement ?? "below";

  return (
    <aside
      ref={panelRef}
      className="reader-word-popover"
      role="dialog"
      aria-label={t("reader-word-card")}
      data-placement={placement}
      style={style}
    >
      <span
        className="reader-word-arrow"
        aria-hidden="true"
        style={{ left: layout?.arrowLeft ?? 20 }}
      />
      <div className="reader-word-body">
        <header className="reader-word-head">
          <h3>{word}</h3>
          <button
            type="button"
            className="reader-word-close"
            aria-label={t("reader-word-card-close")}
            onClick={onClose}
          >
            <StudioIcon name="close" size={18} />
          </button>
        </header>
        {sense ? (
          <>
            {sense.ipa_us || sense.ipa_uk ? (
              <p className="vocab-ipa">/{sense.ipa_us || sense.ipa_uk}/</p>
            ) : null}
            <p>{sense.definition_en}</p>
            {locale === "uz" && sense.gloss_uz ? (
              <p className="vocab-gloss">{sense.gloss_uz}</p>
            ) : null}
            {locale === "ru" && sense.gloss_ru ? (
              <p className="vocab-gloss">{sense.gloss_ru}</p>
            ) : null}
          </>
        ) : (
          <p className="reader-word-hint">{t("reader-not-in-dictionary")}</p>
        )}
        <figure className="reader-word-example">
          <figcaption>{t("reader-word-example")}</figcaption>
          <blockquote lang="en">{sentenceText}</blockquote>
        </figure>
        {sense ? null : (
          <label className="field reader-word-field">
            <span>{t("vocab-form-definition")}</span>
            <textarea
              className="reader-word-input"
              rows={2}
              value={customDefinition}
              onChange={(event) =>
                onCustomDefinitionChange(event.currentTarget.value)
              }
            />
          </label>
        )}
        <Button
          type="button"
          variant="primary"
          disabled={busy || inVocabulary}
          onClick={onAdd}
        >
          {inVocabulary
            ? t("reader-in-vocabulary")
            : t("reader-add-vocabulary")}
        </Button>
      </div>
    </aside>
  );
}
