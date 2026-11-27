import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { useTranslation } from "../i18n";
import { IellaSprite, type IellaLook, type IellaMood } from "./IellaSprite";

export interface IellaCompanionProps {
  activeRoute: string;
  mood?: IellaMood;
  signal?: number;
  onActivate?: () => void;
}

interface Point {
  x: number;
  y: number;
}

const DEFAULT_SIZE = 64;
const SIDEBAR_SIZE = 40;
const SVG_RATIO = 16 / 17;
const FEET_RATIO = 13 / 16;

function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function anchorPoint(anchor: HTMLElement, size: number): Point {
  const rect = anchor.getBoundingClientRect();
  return {
    x: rect.left + rect.width / 2 - size / 2,
    y: rect.top - size * SVG_RATIO * FEET_RATIO,
  };
}

function visibleAnchors(): HTMLElement[] {
  return Array.from(
    document.querySelectorAll<HTMLElement>("[data-iella-perch]"),
  ).filter((anchor) => {
    const rect = anchor.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  });
}

function perchForRoute(route: string): HTMLElement | null {
  const all = visibleAnchors();
  if (all.length === 0) {
    return null;
  }
  return (
    all.find((anchor) => anchor.dataset["iellaRoute"] === route) ??
    all.find((anchor) => anchor.dataset["iellaRoute"] === "app") ??
    all[0] ??
    null
  );
}

function easeInOut(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
}

export function IellaCompanion({
  activeRoute,
  mood = "perched",
  signal = 0,
  onActivate,
}: IellaCompanionProps) {
  const { t } = useTranslation();
  const [placed, setPlaced] = useState(false);
  const [moodState, setMoodState] = useState<IellaMood>("perched");
  const [look, setLook] = useState<IellaLook>("center");
  const [spriteSize, setSpriteSize] = useState(DEFAULT_SIZE);
  const [landing, setLanding] = useState(false);
  const nodeRef = useRef<HTMLDivElement | null>(null);
  const anchorRef = useRef<HTMLElement | null>(null);
  const positionRef = useRef<Point>({ x: 0, y: 0 });
  const frameRef = useRef<number | null>(null);
  const animationRef = useRef(0);
  const moodTimerRef = useRef<number | null>(null);
  const landingTimerRef = useRef<number | null>(null);
  const lastSignalRef = useRef(0);
  const lastRouteRef = useRef(activeRoute);
  const mountedRef = useRef(false);
  const placedRef = useRef(false);

  const paint = (point: Point, rotation = 0) => {
    positionRef.current = point;
    const node = nodeRef.current;
    if (node) {
      node.style.transform = `translate3d(${point.x}px, ${point.y}px, 0) rotate(${rotation}deg)`;
      node.classList.add("placed");
    }
    if (!placedRef.current) {
      placedRef.current = true;
      setPlaced(true);
    }
  };

  const setTemporaryMood = (next: IellaMood, duration: number) => {
    setMoodState(next);
    if (moodTimerRef.current !== null) {
      window.clearTimeout(moodTimerRef.current);
    }
    moodTimerRef.current = window.setTimeout(() => {
      setMoodState("perched");
      moodTimerRef.current = null;
    }, duration);
  };

  const moveTo = (anchor: HTMLElement, immediate: boolean) => {
    anchorRef.current = anchor;
    const perch = anchor.dataset["iellaPerch"];
    const nextSize =
      perch === "sidebar" || perch === "mobile-brand"
        ? SIDEBAR_SIZE
        : DEFAULT_SIZE;
    setSpriteSize(nextSize);
    const target = anchorPoint(anchor, nextSize);
    const from = positionRef.current;
    if (frameRef.current !== null) {
      window.cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
    if (landingTimerRef.current !== null) {
      window.clearTimeout(landingTimerRef.current);
      landingTimerRef.current = null;
    }
    setLanding(false);
    if (immediate || reducedMotion() || typeof window.requestAnimationFrame !== "function") {
      paint(target);
      setMoodState("perched");
      return;
    }
    const span = Math.hypot(target.x - from.x, target.y - from.y);
    const duration = Math.min(1450, Math.max(680, span * 1.65));
    const normalLength = Math.max(1, span);
    const curve = Math.min(110, 24 + span * 0.22);
    const mid = {
      x: (from.x + target.x) / 2,
      y: (from.y + target.y) / 2,
    };
    const normal = {
      x: -(target.y - from.y) / normalLength,
      y: (target.x - from.x) / normalLength,
    };
    const control = {
      x: mid.x + normal.x * curve,
      y: mid.y + normal.y * curve,
    };
    const direction = target.x >= from.x ? 1 : -1;
    const animation = animationRef.current + 1;
    animationRef.current = animation;
    const start = performance.now();
    setLook(direction > 0 ? "right" : "left");
    setMoodState("flying");
    const tick = (now: number) => {
      if (animationRef.current !== animation) {
        return;
      }
      const progress = Math.min(1, (now - start) / duration);
      const p = easeInOut(progress);
      const one = 1 - p;
      const baseY =
        one * one * from.y + 2 * one * p * control.y + p * p * target.y;
      const anticipation =
        progress < 0.12
          ? Math.sin((progress / 0.12) * Math.PI) * 2.4
          : 0;
      const lift = -Math.sin(Math.PI * progress) * Math.min(54, 14 + span * 0.06);
      const point = {
        x:
          one * one * from.x +
          2 * one * p * control.x +
          p * p * target.x,
        y: baseY + anticipation + lift,
      };
      paint(point, Math.sin(Math.PI * progress) * direction * 5);
      if (progress < 1) {
        frameRef.current = window.requestAnimationFrame(tick);
        return;
      }
      frameRef.current = null;
      paint(target, 0);
      setLook(direction > 0 ? "left" : "right");
      setMoodState("perched");
      setLanding(true);
      landingTimerRef.current = window.setTimeout(() => {
        setLanding(false);
        landingTimerRef.current = null;
      }, 260);
    };
    frameRef.current = window.requestAnimationFrame(tick);
  };

  useLayoutEffect(() => {
    if (mountedRef.current) {
      return;
    }
    mountedRef.current = true;
    const anchor = perchForRoute(activeRoute);
    if (anchor) {
      moveTo(anchor, true);
    }
    return () => undefined;
    // Initial placement only; later route changes are handled below.
  }, []);

  useEffect(() => {
    if (lastRouteRef.current === activeRoute) {
      return;
    }
    lastRouteRef.current = activeRoute;
    const timer = window.setTimeout(() => {
      const anchor = perchForRoute(activeRoute);
      if (anchor) {
        moveTo(anchor, false);
      }
    }, 140);
    return () => window.clearTimeout(timer);
  }, [activeRoute]);

  useEffect(() => {
    if (signal === lastSignalRef.current) {
      return;
    }
    lastSignalRef.current = signal;
    if (signal === 0) {
      return;
    }
    if (mood === "flying") {
      setTemporaryMood("flying", 700);
      return;
    }
    setTemporaryMood(mood, mood === "proud" ? 1500 : 1100);
    // The signal is a reaction; route changes are the only self-initiated move.
  }, [signal, mood]);


  useEffect(() => {
    const settle = () => {
      const anchor = anchorRef.current;
      if (anchor && frameRef.current === null) {
        paint(
          anchorPoint(
            anchor,
            anchor.dataset["iellaPerch"] === "sidebar" ||
              anchor.dataset["iellaPerch"] === "mobile-brand"
              ? SIDEBAR_SIZE
              : DEFAULT_SIZE,
          ),
        );
      }
    };
    window.addEventListener("resize", settle);
    window.addEventListener("scroll", settle, true);
    return () => {
      window.removeEventListener("resize", settle);
      window.removeEventListener("scroll", settle, true);
    };
  }, []);

  useEffect(
    () => () => {
      animationRef.current += 1;
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
      }
      if (moodTimerRef.current !== null) {
        window.clearTimeout(moodTimerRef.current);
      }
      if (landingTimerRef.current !== null) {
        window.clearTimeout(landingTimerRef.current);
      }
    },
    [],
  );

  const handleActivate = () => {
    setTemporaryMood("proud", 1400);
    onActivate?.();
  };

  const sprite = onActivate ? (
    <IellaSprite
      size={spriteSize}
      mood={moodState}
      look={look}
      label={t("iella-open-tour")}
      onActivate={handleActivate}
      onHover={() => setTemporaryMood("curious", 900)}
    />
  ) : (
    <IellaSprite size={spriteSize} mood={moodState} look={look} />
  );

  return (
    <div
      ref={nodeRef}
      className={`iella-companion${placed ? " placed" : ""}${landing ? " landing" : ""}`}
      data-mood={moodState}
      aria-hidden={onActivate ? undefined : true}
    >
      {sprite}
    </div>
  );
}
