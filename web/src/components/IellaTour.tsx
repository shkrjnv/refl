import { useEffect, useMemo, useRef, useState } from "react";

import { useTranslation } from "../i18n";
import {
  completeTour,
  recordTourStep,
} from "../prototype/onboarding";
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

export function IellaTour({ open, onComplete }: IellaTourProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [rect, setRect] = useState<SpotlightRect | null>(null);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const current = useMemo<Stop>(() => {
    const stop = STOPS[step] ?? STOPS[0];
    return {
      target: stop.target,
      message: t(stop.message),
      label: t(stop.label),
    };
  }, [step, t]);

  useEffect(() => {
    if (!open) {
      setStep(0);
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
        top: bounds.top - 8,
        left: bounds.left - 8,
        width: bounds.width + 16,
        height: bounds.height + 16,
      });
    };
    updateRect();
    window.addEventListener("resize", updateRect);
    window.addEventListener("scroll", updateRect, true);
    cardRef.current?.focus();
    return () => {
      window.removeEventListener("resize", updateRect);
      window.removeEventListener("scroll", updateRect, true);
    };
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

  if (!open) {
    return null;
  }

  const finish = (skipped: boolean) => {
    recordTourStep(skipped ? "skip" : "finish");
    completeTour();
    onComplete();
  };

  const isLast = step >= STOPS.length - 1;
  const cardPosition: { top?: number; left?: number } = rect
    ? {
        top: Math.max(
          16,
          Math.min(
            rect.top + rect.height + 16,
            typeof window === "undefined" ? 16 : window.innerHeight - 280,
          ),
        ),
        left: Math.max(
          16,
          Math.min(
            rect.left,
            typeof window === "undefined" ? 16 : window.innerWidth - 372,
          ),
        ),
      }
    : {};
  const spotlightStyle = rect
    ? {
        top: `${rect.top}px`,
        left: `${rect.left}px`,
        width: `${rect.width}px`,
        height: `${rect.height}px`,
      }
    : undefined;

  return (
    <div
      className={`iella-tour${rect ? "" : " iella-tour-centered"}`}
      role="dialog"
      aria-modal="true"
      aria-labelledby="iella-tour-title"
    >
      <div className="iella-tour-blocker" aria-hidden="true" />
      {spotlightStyle ? (
        <div className="iella-spotlight" style={spotlightStyle} aria-hidden="true" />
      ) : null}
      <section
        ref={cardRef}
        className={`iella-tour-card${rect ? "" : " iella-tour-card-centered"}`}
        style={cardPosition}
        tabIndex={-1}
      >
        <div className="iella-tour-sprite">
          <IellaSprite size={64} />
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
    </div>
  );
}
