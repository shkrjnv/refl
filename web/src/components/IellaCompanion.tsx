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

const SIZE = 64;
const SVG_HEIGHT = (SIZE * 19) / 24;
const FEET_RATIO = 18 / 19;

function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function anchorPoint(anchor: HTMLElement): Point {
  const rect = anchor.getBoundingClientRect();
  return {
    x: rect.left + rect.width / 2 - SIZE / 2,
    y: rect.top - SVG_HEIGHT * FEET_RATIO,
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

function pickAnchor(
  route: string,
  previous: HTMLElement | null,
): HTMLElement | null {
  const all = visibleAnchors();
  if (all.length === 0) {
    return null;
  }
  const routeAnchors = all.filter((anchor) => {
    const anchorRoute = anchor.dataset["iellaRoute"];
    return anchorRoute === route || anchorRoute === "app";
  });
  const pool = routeAnchors.length > 0 ? routeAnchors : all;
  const candidates = pool.filter((anchor) => anchor !== previous);
  const list = candidates.length > 0 ? candidates : pool;
  return list[Math.floor(Math.random() * list.length)] ?? null;
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
  const nodeRef = useRef<HTMLDivElement | null>(null);
  const anchorRef = useRef<HTMLElement | null>(null);
  const positionRef = useRef<Point>({ x: 0, y: 0 });
  const frameRef = useRef<number | null>(null);
  const animationRef = useRef(0);
  const moodTimerRef = useRef<number | null>(null);
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
    const target = anchorPoint(anchor);
    const from = positionRef.current;
    if (frameRef.current !== null) {
      window.cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
    if (immediate || reducedMotion() || typeof window.requestAnimationFrame !== "function") {
      paint(target);
      setMoodState("perched");
      return;
    }
    const span = Math.hypot(target.x - from.x, target.y - from.y);
    const duration = Math.min(1500, Math.max(650, span * 1.8));
    const normalLength = Math.max(1, span);
    const curve = Math.min(130, 28 + span * 0.26);
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
      const point = {
        x:
          one * one * from.x +
          2 * one * p * control.x +
          p * p * target.x,
        y:
          one * one * from.y +
          2 * one * p * control.y +
          p * p * target.y,
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
    };
    frameRef.current = window.requestAnimationFrame(tick);
  };

  useLayoutEffect(() => {
    if (mountedRef.current) {
      return;
    }
    mountedRef.current = true;
    const anchor = pickAnchor(activeRoute, null);
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
      const anchor = pickAnchor(activeRoute, anchorRef.current);
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
    if (mood === "flying" || mood === "curious") {
      const anchor = pickAnchor(activeRoute, anchorRef.current);
      if (anchor) {
        moveTo(anchor, false);
      } else {
        setTemporaryMood(mood, 1200);
      }
      return;
    }
    setTemporaryMood(mood, mood === "proud" ? 1500 : 1100);
    // The signal is the trigger; the mood value describes the reaction.
  }, [signal, mood, activeRoute]);

  useEffect(() => {
    if (reducedMotion()) {
      return;
    }
    let timer = 0;
    const schedule = () => {
      const delay = 18_000 + Math.random() * 7_000;
      timer = window.setTimeout(() => {
        const anchor = pickAnchor(activeRoute, anchorRef.current);
        if (anchor) {
          moveTo(anchor, false);
        }
        schedule();
      }, delay);
    };
    schedule();
    return () => window.clearTimeout(timer);
  }, [activeRoute]);

  useEffect(() => {
    const settle = () => {
      const anchor = anchorRef.current;
      if (anchor && frameRef.current === null) {
        paint(anchorPoint(anchor));
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
    },
    [],
  );

  const handleActivate = () => {
    setTemporaryMood("proud", 1400);
    onActivate?.();
  };

  const sprite = onActivate ? (
    <IellaSprite
      size={SIZE}
      mood={moodState}
      look={look}
      label={t("iella-open-tour")}
      onActivate={handleActivate}
    />
  ) : (
    <IellaSprite size={SIZE} mood={moodState} look={look} />
  );

  return (
    <div
      ref={nodeRef}
      className={`iella-companion${placed ? " placed" : ""}`}
      data-mood={moodState}
      aria-hidden={onActivate ? undefined : true}
    >
      {sprite}
    </div>
  );
}
