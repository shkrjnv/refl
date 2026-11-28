import {
  useEffect,
  useId,
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

type StopKind = "arrival" | "try" | "route" | "level" | "start";

interface Stop {
  kind: StopKind;
  target: string | null;
  message: string;
  label: string;
}

const STOPS: readonly Stop[] = [
  { kind: "arrival", target: null, message: "tour-arrival", label: "tour-arrival-label" },
  { kind: "try", target: null, message: "tour-try", label: "tour-try-label" },
  { kind: "route", target: '[data-tour="route"]', message: "tour-route", label: "tour-route-label" },
  { kind: "level", target: '[data-tour="level"]', message: "tour-level", label: "tour-level-label" },
  { kind: "start", target: '[data-tour="start"]', message: "tour-start", label: "tour-start-label" },
];

const HIGHLIGHT_PADDING = 8;
const HIGHLIGHT_RADIUS = 12;
const CARD_GAP = 14;
const VIEWPORT_MARGIN = 16;
const SETTLE_MS = 900;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function cancelFrame(frame: number): void {
  if (typeof window.cancelAnimationFrame === "function") {
    window.cancelAnimationFrame(frame);
  }
}

function sameRect(left: SpotlightRect | null, right: SpotlightRect): boolean {
  return (
    left !== null &&
    Math.abs(left.top - right.top) < 0.05 &&
    Math.abs(left.left - right.left) < 0.05 &&
    Math.abs(left.width - right.width) < 0.05 &&
    Math.abs(left.height - right.height) < 0.05
  );
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
  const [tryAnswer, setTryAnswer] = useState<"a" | "an" | null>(null);
  const cardRef = useRef<HTMLElement | null>(null);
  const rawMaskId = useId();
  const maskId = `iella-tour-mask-${rawMaskId.replace(/[^A-Za-z0-9_]/g, "")}`;
  const current = useMemo<Stop>(() => {
    const stop = STOPS[step] ?? STOPS[0]!;
    return {
      kind: stop.kind,
      target: stop.target,
      message: t(stop.message),
      label: t(stop.label),
    };
  }, [step, t]);

  useLayoutEffect(() => {
    if (!open) {
      setStep(0);
      setRect(null);
      setTryAnswer(null);
      return;
    }
    const stop = STOPS[step] ?? STOPS[0]!;
    recordTourStep(`stop-${stop.kind}`);
    setTryAnswer(null);
    let element: HTMLElement | null = null;
    let observer: ResizeObserver | null = null;
    let frame = 0;
    let settleUntil = 0;

    const find = (): HTMLElement | null =>
      stop.target
        ? document.querySelector<HTMLElement>(stop.target)
        : null;

    const measure = () => {
      const found = find();
      if (found !== element) {
        observer?.disconnect();
        observer = null;
        element = found;
        if (element && typeof ResizeObserver !== "undefined") {
          observer = new ResizeObserver(() => settle(SETTLE_MS));
          observer.observe(element);
        }
      }
      if (!element) {
        setRect((previous) => (previous === null ? previous : null));
        return;
      }
      const bounds = element.getBoundingClientRect();
      const next = {
        top: bounds.top - HIGHLIGHT_PADDING,
        left: bounds.left - HIGHLIGHT_PADDING,
        width: bounds.width + HIGHLIGHT_PADDING * 2,
        height: bounds.height + HIGHLIGHT_PADDING * 2,
      };
      setRect((previous) => (sameRect(previous, next) ? previous : next));
    };

    const settle = (duration = SETTLE_MS) => {
      if (typeof requestAnimationFrame !== "function") {
        measure();
        return;
      }
      cancelFrame(frame);
      settleUntil = performance.now() + duration;
      const tick = (now: number) => {
        measure();
        if (now < settleUntil) {
          frame = window.requestAnimationFrame(tick);
        }
      };
      frame = window.requestAnimationFrame(tick);
    };

    const target = find();
    if (target && typeof target.scrollIntoView === "function") {
      const reduceMotion =
        typeof window.matchMedia === "function" &&
        window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      target.scrollIntoView({
        behavior: reduceMotion ? "auto" : "smooth",
        block: "center",
        inline: "nearest",
      });
    }

    const onResize = () => settle(500);
    const onScroll = () => settle(500);
    const onTransitionEnd = () => settle(350);
    measure();
    settle(1_200);
    window.addEventListener("resize", onResize);
    window.addEventListener("scroll", onScroll, true);
    document.addEventListener("transitionend", onTransitionEnd, true);
    document.addEventListener("animationend", onTransitionEnd, true);
    cardRef.current?.focus();
    return () => {
      cancelFrame(frame);
      observer?.disconnect();
      window.removeEventListener("resize", onResize);
      window.removeEventListener("scroll", onScroll, true);
      document.removeEventListener("transitionend", onTransitionEnd, true);
      document.removeEventListener("animationend", onTransitionEnd, true);
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
    let frame = 0;
    const measure = () => {
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
    };
    measure();
    if (typeof requestAnimationFrame === "function") {
      frame = window.requestAnimationFrame(measure);
    }
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            if (typeof requestAnimationFrame === "function") {
              cancelFrame(frame);
              frame = window.requestAnimationFrame(measure);
            } else {
              measure();
            }
          });
    observer?.observe(node);
    return () => {
      cancelFrame(frame);
      observer?.disconnect();
    };
  }, [open, step, tryAnswer]);

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

  return createPortal(
    <div
      className={`iella-tour${rect ? "" : " iella-tour-centered"}`}
      role="dialog"
      aria-modal="true"
      aria-labelledby="iella-tour-title"
    >
      {rect ? (
        <svg
          className="iella-tour-mask"
          width="100%"
          height="100%"
          aria-hidden="true"
        >
          <defs>
            <mask
              id={maskId}
              maskUnits="userSpaceOnUse"
              x="0"
              y="0"
              width="100%"
              height="100%"
            >
              <rect width="100%" height="100%" fill="white" />
              <rect
                x={rect.left}
                y={rect.top}
                width={rect.width}
                height={rect.height}
                rx={HIGHLIGHT_RADIUS}
                fill="black"
              />
            </mask>
          </defs>
          <rect
            width="100%"
            height="100%"
            fill="var(--overlay)"
            mask={`url(#${maskId})`}
          />
          <rect
            className="iella-tour-highlight-svg"
            x={rect.left}
            y={rect.top}
            width={rect.width}
            height={rect.height}
            rx={HIGHLIGHT_RADIUS}
          />
        </svg>
      ) : (
        <div className="iella-tour-blocker" aria-hidden="true" />
      )}
      <section
        key={`${step}-${current.kind}`}
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
        {current.kind === "try" ? (
          <div className="iella-tour-try">
            <p className="iella-tour-try-question">
              {t("tour-try-question")}
            </p>
            <div
              className="iella-tour-try-options"
              role="group"
              aria-label={t("tour-try-label")}
            >
              {(["a", "an"] as const).map((option) => (
                <button
                  key={option}
                  type="button"
                  className={`iella-tour-try-option${tryAnswer === option ? " selected" : ""}`}
                  aria-pressed={tryAnswer === option}
                  disabled={tryAnswer !== null}
                  onClick={() => setTryAnswer(option)}
                >
                  {t(`tour-try-${option}`)}
                </button>
              ))}
            </div>
            {tryAnswer ? (
              <p
                className={`iella-tour-try-feedback${tryAnswer === "a" ? " correct" : " wrong"}`}
                role="status"
              >
                {t(
                  tryAnswer === "a"
                    ? "tour-try-right"
                    : "tour-try-wrong",
                )}
              </p>
            ) : null}
          </div>
        ) : null}
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
