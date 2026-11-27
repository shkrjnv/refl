import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";

import { useTranslation } from "../i18n";
import { completeTour, recordTourStep } from "../prototype/onboarding";
import { IellaSprite } from "./IellaSprite";
import { Button } from "./ui";

export interface IellaTourProps {
  open: boolean;
  onComplete: () => void;
}

interface SpotlightRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface Stop {
  target: string | null;
  message: string;
  label: string;
}

const STOPS = [
  { target: null, message: "tour-arrival", label: "tour-arrival-label" },
  { target: '[data-tour="route"]', message: "tour-route", label: "tour-route-label" },
  { target: '[data-tour="level"]', message: "tour-level", label: "tour-level-label" },
  { target: '[data-tour="start"]', message: "tour-start", label: "tour-start-label" },
] as const;

const HIGHLIGHT_PADDING = 8;
const CARD_GAP = 14;
const VIEWPORT_MARGIN = 16;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function cardPlacement(
  rect: SpotlightRect,
  width: number,
  height: number,
): { top: number; left: number } {
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;
  const cardWidth = Math.min(width, viewportWidth - VIEWPORT_MARGIN * 2);
  const cardHeight = Math.min(height, viewportHeight - VIEWPORT_MARGIN * 2);
  const below = viewportHeight - (rect.top + rect.height) - VIEWPORT_MARGIN;
  const above = rect.top - VIEWPORT_MARGIN;
  const right = viewportWidth - (rect.left + rect.width) - VIEWPORT_MARGIN;
  const left = rect.left - VIEWPORT_MARGIN;

  let top = viewportHeight - cardHeight - VIEWPORT_MARGIN;
  let x = viewportWidth - cardWidth - VIEWPORT_MARGIN;

  if (below >= cardHeight + CARD_GAP) {
    top = rect.top + rect.height + CARD_GAP;
    x = rect.left;
  } else if (above >= cardHeight + CARD_GAP) {
    top = rect.top - cardHeight - CARD_GAP;
    x = rect.left;
  } else if (right >= cardWidth + CARD_GAP) {
    top = rect.top;
    x = rect.left + rect.width + CARD_GAP;
  } else if (left >= cardWidth + CARD_GAP) {
    top = rect.top;
    x = rect.left - cardWidth - CARD_GAP;
  }

  return {
    top: clamp(top, VIEWPORT_MARGIN, viewportHeight - cardHeight - VIEWPORT_MARGIN),
    left: clamp(x, VIEWPORT_MARGIN, viewportWidth - cardWidth - VIEWPORT_MARGIN),
  };
}

export function IellaTour({ open, onComplete }: IellaTourProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [rect, setRect] = useState<SpotlightRect | null>(null);
  const [cardSize, setCardSize] = useState({ width: 320, height: 236 });
  const cardRef = useRef<HTMLElement | null>(null);
  const current = useMemo<Stop>(() => {
    const stop = STOPS[step] ?? STOPS[0];
    return {
      target: stop.target,
      message: t(stop.message),
      label: t(stop.label),
    };
  }, [step, t]);

  useLayoutEffect(() => {
    if (!open) {
      setStep(0);
      setRect(null);
      return;
    }
    const stop = STOPS[step] ?? STOPS[0];
    recordTourStep(`stop-${step}`);
    const updateRect = () => {
      if (!stop.target) {
        setRect(null);
        return;
      }
      const element = document.querySelector(stop.target);
      if (!(element instanceof HTMLElement)) {
        setRect(null);
        return;
      }
      const bounds = element.getBoundingClientRect();
      setRect({
        top: bounds.top - HIGHLIGHT_PADDING,
        left: bounds.left - HIGHLIGHT_PADDING,
        width: bounds.width + HIGHLIGHT_PADDING * 2,
        height: bounds.height + HIGHLIGHT_PADDING * 2,
      });
    };

    if (stop.target) {
      const element = document.querySelector(stop.target);
      if (
        element instanceof HTMLElement &&
        typeof element.scrollIntoView === "function"
      ) {
        const reduceMotion =
          typeof window.matchMedia === "function" &&
          window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        element.scrollIntoView({
          behavior: reduceMotion ? "auto" : "smooth",
          block: "center",
          inline: "nearest",
        });
      }
    }

    updateRect();
    window.addEventListener("resize", updateRect);
    window.addEventListener("scroll", updateRect, true);
    cardRef.current?.focus();
    return () => {
      window.removeEventListener("resize", updateRect);
      window.removeEventListener("scroll", updateRect, true);
    };
  }, [open, step]);

  useLayoutEffect(() => {
    if (!open) {
      return;
    }
    const node = cardRef.current;
    if (!node) {
      return;
    }
    const bounds = node.getBoundingClientRect();
    if (bounds.width < 1 || bounds.height < 1) {
      return;
    }
    setCardSize((previous) =>
      Math.abs(previous.width - bounds.width) < 1 &&
      Math.abs(previous.height - bounds.height) < 1
        ? previous
        : { width: bounds.width, height: bounds.height },
    );
  }, [open, step]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        recordTourStep("skip");
        completeTour();
        onComplete();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onComplete]);

  if (!open || typeof document === "undefined") {
    return null;
  }

  const finish = (skipped: boolean) => {
    recordTourStep(skipped ? "skip" : "finish");
    completeTour();
    onComplete();
  };

  const isLast = step >= STOPS.length - 1;
  const cardPosition = rect
    ? cardPlacement(rect, cardSize.width, cardSize.height)
    : undefined;
  const highlightTop = rect ? Math.round(rect.top) : 0;
  const highlightLeft = rect ? Math.round(rect.left) : 0;
  const highlightRight = rect ? Math.round(rect.left + rect.width) : 0;
  const highlightBottom = rect ? Math.round(rect.top + rect.height) : 0;

  return createPortal(
    <div
      className={`iella-tour${rect ? "" : " iella-tour-centered"}`}
      role="dialog"
      aria-modal="true"
      aria-labelledby="iella-tour-title"
    >
      {rect ? (
        <>
          <div
            className="iella-tour-shade"
            style={{
              top: 0,
              left: 0,
              right: 0,
              height: Math.max(0, highlightTop),
            }}
            aria-hidden="true"
          />
          <div
            className="iella-tour-shade"
            style={{
              top: highlightBottom,
              left: 0,
              right: 0,
              bottom: 0,
            }}
            aria-hidden="true"
          />
          <div
            className="iella-tour-shade"
            style={{
              top: highlightTop,
              left: 0,
              width: Math.max(0, highlightLeft),
              height: Math.max(0, highlightBottom - highlightTop),
            }}
            aria-hidden="true"
          />
          <div
            className="iella-tour-shade"
            style={{
              top: highlightTop,
              left: highlightRight,
              right: 0,
              height: Math.max(0, highlightBottom - highlightTop),
            }}
            aria-hidden="true"
          />
          <div
            className="iella-tour-highlight"
            style={{
              top: highlightTop,
              left: highlightLeft,
              width: Math.max(0, highlightRight - highlightLeft),
              height: Math.max(0, highlightBottom - highlightTop),
            }}
            aria-hidden="true"
          />
        </>
      ) : (
        <div className="iella-tour-blocker" aria-hidden="true" />
      )}
      <section
        ref={cardRef}
        className={`iella-tour-card${rect ? "" : " iella-tour-card-centered"}`}
        style={cardPosition}
        tabIndex={-1}
      >
        <div className="iella-tour-sprite">
          <IellaSprite size={48} />
        </div>
        <p className="iella-tour-kicker">{current.label}</p>
        <h2 id="iella-tour-title">{current.message}</h2>
        <div className="iella-tour-actions">
          <Button variant="ghost" size="sm" onClick={() => finish(true)}>
            {t("tour-skip")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => {
              if (isLast) {
                finish(false);
              } else {
                setStep((value) => value + 1);
              }
            }}
          >
            {isLast ? t("tour-finish") : t("tour-next")}
          </Button>
        </div>
      </section>
    </div>,
    document.body,
  );
}
